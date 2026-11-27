// Batch 427 (nominal) — the pico-dgx-ds4 engine distill's `rust_perf`
// member (Plan 420 / Research 454). Source: Picom-code/pico-dgx-ds4 @
// `922dddb1a35e521148022ee11264632da1f0ddc0` (tip == pin at authoring,
// commit-dated 2026-11-20, MIT — Copyright (c) 2026 Andy Shu; upstream
// blaesus/dsgx10). SEVEN rules: R6 R7 R8 R9 R10 R12 (measurement-honesty)
// + R13 (io) — every one suggestion-only `NotBenchable` bench_spec None
// (the dial9/136 annotation class); R7's GOAT falsifier cell lives in the
// fixtures file (`b427_perf_fixtures.rs`).
//
// Dedup re-verified at the landing HEAD (both vocabularies, every hit
// read; Research 454's dedup gate, adjudicated IN-RULE below): the
// measurement-honesty family is the corpus's deepest — entries_70/84/93/
// 94/96/100/105 own spread-verdicts, decomposition, self-consistency
// blindness, input realism, pair selection, band derivation and live
// baselines; gpu_98/99/101 own bit-equality oracles and planted controls;
// entries_21/74 own keepalive. The seven unowned axes land here: the
// mismatch-definition + forced-worst-case check posture (R6), the
// paired+decomposition proof at a quantified spread/gain ratio (R7), the
// epistemic label per number (R8), external-figure provenance + the
// ours–kit–ours bracket (R9), the MoE expert-union prompt class (R10),
// the seed-valued digest refusal (R12), and the pad-to-interrupt-
// threshold completion atom (R13).

use crate::domains::rust_perf::PerfRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b427_rules() -> Vec<PerfRule> {
    vec![
        // ── R6: a bound-gated fast path is verified by the slow path in the
        //    SAME step, under a precise definition, with forced worst-case
        //    arms ──
        PerfRule {
            rule_id: "certified-fast-path-verified-by-slow-path-beside-it".to_string(),
            name: "an approximation with a rigorous bound ships with a CHECK MODE that runs the exact path IN THE SAME STEP on every rank and counts mismatches under a precise two-disjunct definition, plus TWO forced-worst-case arms (the fallback forced on a stride; the bound scaled up so almost nothing can be pruned) — a happy-path-only check proves nothing about the fallback branch the bound exists to skip".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// A bound-gated certified fast path whose verification is
// HAPPY-PATH only: the check flag runs beside the fast path,
// "any difference" is logged and discounted (no definition),
// and nothing ever forces the two branches the bound exists
// to skip — the fallback and the no-prune regime.
fn decode_step(row_bounds: &[Bound], best_lower: f64) -> Output {
    if certified_ok(row_bounds, best_lower) {
        let out = certified_head(row_bounds);
        if CHECK {
            // reached ONLY when the fast path was taken; any
            // difference logs and moves on — no definition, no
            // counter, no census.
            let full = full_head(row_bounds);
            if out != full {
                eprintln!("cert drift (informational)");
            }
        }
        return out;
    }
    full_head(row_bounds) // the fallback branch: never checked
}
"#.to_string(),
            fixed_code: r"// FIXED: check mode runs the full head BESIDE the certified one
// in the same step; a row counts as a mismatch under the precise
// definition (same (maximum, lowest id), OR the full head's max
// is below the SHARED threshold — that rank cannot hold the
// winner under either head); the census counts rows, mismatches,
// fallback steps and groups read; the two forced arms exercise
// the branches the bound skips (force the fallback on a stride;
// scale the bound so almost nothing can be pruned).
use std::sync::atomic::{AtomicU64, Ordering};

/// one threshold shared across ranks — per-rank thresholds cannot
/// prune (a rank not holding the winner can never prune).
const SHARED_THRESHOLD: i64 = 50;

/// The precise definition. A row counts as a mismatch unless the
/// certified logits give the same (maximum, lowest id) as the full
/// head on that rank, or the full head's maximum on that rank is
/// below the shared threshold.
fn row_is_mismatch(certified: (i64, u32), full: (i64, u32), full_max: i64) -> bool {
    let same_pick = certified == full;
    !(same_pick || full_max < SHARED_THRESHOLD)
}

struct CertCensus {
    rows_checked: AtomicU64,
    mismatches: AtomicU64,
    fallback_steps: AtomicU64,
    groups_read: AtomicU64,
}

/// `force_fallback_every`: the adversarial arm that proves the fallback
/// branch runs (every Nth step takes the full mask). `bound_scale`: the
/// adversarial arm that proves the no-prune regime (scale the bound so
/// almost no group can be pruned).
fn decode_step_checked(
    census: &CertCensus,
    groups: &[Group],
    force_fallback_every: Option<usize>,
    bound_scale: f64,
    step: usize,
) -> Output {
    let forced = matches!(force_fallback_every, Some(n) if step % n == 0);
    if forced {
        census.fallback_steps.fetch_add(1, Ordering::Relaxed);
    }
    let mut out_groups = Vec::new();
    for g in groups {
        // the bound prunes the group only when its scaled upper bound
        // stays under the best lower bound; the full head reads the
        // survivors — and under check mode EVERY read row is verified
        // against the certified output IN THE SAME STEP.
        let prunable = !forced && g.upper_bound() * bound_scale < best_lower_of(g);
        if prunable {
            out_groups.push(g.certified_output());
            continue;
        }
        census.groups_read.fetch_add(1, Ordering::Relaxed);
        let full = g.full_head_output();
        let cert = g.certified_output();
        for (row, full_row) in full.rows.iter().enumerate() {
            census.rows_checked.fetch_add(1, Ordering::Relaxed);
            if row_is_mismatch(cert.rows[row], *full_row, full_row.0) {
                census.mismatches.fetch_add(1, Ordering::Relaxed);
            }
        }
        out_groups.push(full);
    }
    Output::from_groups(out_groups)
}
".to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (tip == pin at authoring; MIT; upstream blaesus/dsgx10), docs/stack4.md:87-98 (the check-mode proof table + the definition at :93-95) and :124-126 (the shipped switches — DSGB10_CERT_CHECK=1 runs the check, DSGB10_CERT_FORCE=N forces the full mask every Nth step, DSGB10_CERT_BSCALE=f loosens the bound: the two adversarial arms are CONFIGURATION, not one-off runs). THE MECHANISM, verbatim: 'Proof (engine in check mode: the full head runs beside the certified one in the same step, on every rank)' + the definition 'A row counts as a mismatch unless the certified logits give the same (maximum, lowest id) as the full head on that rank, or the full head's maximum on that rank is below the shared threshold (then that rank cannot hold the winner under either head)' + the census '1,344,416 real step rows at the serving bound, zero mismatches on every rank; the largest |coarse - exact| was 8.3 % of the bound'. THE TWO ADVERSARIAL ARMS are the content — verbatim from the proof table: 'fallback forced every 3rd step … 10,336 steps ran all 2,020 groups' (proves the fallback branch runs and is correct) and 'bound scaled x4 (almost nothing can be pruned) … 2,018 of 2,020 groups read on average' (proves the no-prune regime is correct and its cost is known); a happy-path-only check never sees either branch, and the fallback IS the correctness story — the bound's whole value is how rarely it fires. THE DEFINITION is load-bearing too: without the below-threshold disjunct every under-threshold row is a false mismatch and the census drowns in noise nobody reads. ⛔ ADJUDICATIONS (both directions): vs canary-admission-per-config-bit-equal-stock (gpu_98), bit-equality-oracle-keyed-on-reference-dispatch (gpu_99) and kernel-contract-planted-gate-controls (gpu_101) — those own bit-equality ORACLES keyed on reference dispatch and PLANTED CONTROLS for kernel contracts; THIS owns the mismatch DEFINITION (the two-disjunct rule with the shared threshold) + the FORCED-WORST-CASE arms for a BOUND-gated path (the gate's skip branch is the thing under test, and the skip branch is forced by configuration on a stride, not planted in data); compose: a planted control (gpu_101) inside this rule's forced arm is the full posture. vs the entries_70 spread family — the check census is a COUNT axis (integer mismatches), not a timing claim; no band applies. REGIME: any certified/approximate fast path with a rigorous bound (certified argmax heads, screen-and-rescore, bound-gated prefilters, cheap-routing layers) — especially multi-rank/multi-worker, where the definition must hold per rank against a threshold shared across the fleet. ANTI-REGIME: exact paths with no approximation (nothing to bound — plain parity tests); sampled/logprob outputs where '(maximum, lowest id)' is not the contract (their own anti-regime: the certified head is greedy-only); holdout accuracy suites (320/320 texts, qeval 71/71 — sample-level instruments beside the row-level census, not substitutes); a check that ONLY runs the happy path is the defect, never a lighter regime. Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R7 (the batch's GOAT lead): below-spread gains are proven by
        //    paired held-out runs + per-step decomposition, not medians ──
        PerfRule {
            rule_id: "gain-within-bench-swing-paired-held-out-plus-per-step-time-is-the-proof".to_string(),
            name: "when a lever's gain sits below the bench's run-to-run spread, the multi-run median is continuity not evidence — the proof of each lever is (1) paired same-text runs over a held-out prompt set, (2) GPU ms per step bucketed by row count from the engine's own step log, and (3) the exact baseline being the SAME BINARY minus the flags; the medians are shown for continuity only".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// A lever's proof is a 9-run median against a 9-run median of
// the base build — on a bench whose runs swing 74-95 tok/s
// between runs of the SAME build. A +1% lever is inside the
// spread; the median comparison reads noise.
fn prove_lever(build_a: &Bench, build_b: &Bench) -> Verdict {
    let med_a = median_of(9, || build_a.run_chats()); // e.g. 84.6
    let med_b = median_of(9, || build_b.run_chats()); // e.g. 85.1
    if med_b > med_a {
        // "+1% lever proven" — but the same build re-run ranges
        // 74..95: the medians swing by several tok/s BETWEEN RUNS
        // OF THE SAME BUILD. The 0.5 gap is noise wearing a tie.
        Verdict::Proven((med_b - med_a) / med_a)
    } else {
        Verdict::Refuted
    }
}
"#.to_string(),
            fixed_code: r"// FIXED: three legs. (1) PAIRED same-text runs — the same held-out
// prompts, same session, base and lever run back to back, so the
// session offset cancels in the pair; (2) the DECOMPOSITION leg —
// GPU ms per step bucketed by row count from the engine's own step
// log, where step time at a fixed row count is a low-noise read and
// the lever's true effect is localized to its buckets; (3) the exact
// baseline is the SAME BINARY minus the lever's flags. The medians
// are reported FOR CONTINUITY, never as the verdict.
fn prove_lever(base: &Build, lever_flags: &[&str]) -> Verdict {
    // leg 3: same binary, flags off = the baseline (work-removed discipline)
    let base_flags: &[&str] = &[];

    // leg 1: paired same-text runs over the held-out set
    let mut rel: Vec<f64> = Vec::new();
    for prompts in held_out_sets() {
        let a = base.run_with(base_flags, prompts);
        let b = base.run_with(lever_flags, prompts);
        rel.push((b.tokens_per_s - a.tokens_per_s) / a.tokens_per_s);
    }
    let paired_gain = mean(&rel); // the session offset cancelled in every pair

    // leg 2: per-step GPU ms by row-count bucket, from the step log
    let buckets_base = gpu_ms_by_rows(base.step_log(base_flags));
    let buckets_lever = gpu_ms_by_rows(base.step_log(lever_flags));
    // the lever moves specific buckets (e.g. 1-row 19.41 -> 18.99 ms);
    // buckets it does not touch are the controls
    let localized = bucket_shifts(&buckets_base, &buckets_lever);

    Verdict::from_parts(paired_gain, localized) // medians: printed, never decisive
}
".to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT), docs/stack4.md:172-173 verbatim: 'The 9- and 27-run medians of our bench swing by several tok/s between runs of the same build (chat runs range 74-95), so the paired 320-prompt runs and the per-step GPU times are the proof of each lever; the medians are shown for continuity.' + :16-32 the paired table (exact 86.19 -> +lever1 87.17 -> +lever2 88.17 -> +lever3 89.45 chat tok/s on the same 320 held-out prompts, texts 320/320 equal to the exact build — per-lever gains +1.1/+1.1/+1.5%) and the per-step GPU-ms-by-rows table (1-row 19.41 -> 18.99 ms fully stacked; the shift is bucket-localized); CHANGES.md:305-306 'Proof per lever: paired runs on the same 320 held-out prompts (same texts), GPU ms per step by rows, and the 54-prompt identity set on a test port.'; root README.md the single-user rows are '9-run medians … for continuity' posture. THE QUANTIFIED RATIO: the lever's gain (+1.1-1.5%) against a run range of 74-95 (a ~±10% swing, spread ~10x the gain) — the decision rule is CONDITIONAL on that ratio: below it, medians decide nothing. THE THREE LEGS: paired same-text runs (the session offset — machine state, cache residency, thermal posture — cancels in the pair; held-out prompts keep the read out of any tuning set); per-step GPU ms bucketed by row count from the engine's own step log (step time at a fixed row count is a low-noise read that localizes WHICH work moved, with untouched buckets as controls); the exact baseline is the same binary minus the flags (no rebuilt 'baseline'). GOAT CELL (T4, pre-registered in b427_perf_fixtures.rs): a seeded noisy-bench simulator at the source's own spread ratio with an injected TRUE +1% on arm B — the unpaired 9-run median vs the paired same-session read over 200 seeds, counting correct verdicts; the falsifier: if the paired read does not beat the median, the rule demotes to suggestion-only decision content. ⛔ ADJUDICATIONS (both directions): vs benchmark-spread-above-threshold-reports-unstable (entries_70) — owns the spread->UNSTABLE VERDICT (warm-up CV settles, spread over threshold reports UNSTABLE, never a flaky headline); THIS owns what to measure INSTEAD once the spread is known — the paired + decomposed proof posture at a quantified spread/gain ratio (entries_70 names the disease, THIS names the treatment); compose. vs score-is-lower-median-pair-never-mean (entries_96) — owns score SELECTION across pairs (the anti-cherry-pick aggregation of an already-paired comparison); THIS owns the comparison DESIGN that makes pairs informative (same texts, same session, held-out set); compose — pair by session and text (THIS), aggregate by lower median pair (entries_96). vs specdec-composite-decomposed-into-per-step-time-and-accepted-length (entries_84) — owns the tok/s DECOMPOSITION into step time x tokens-per-step; the bucket arm is that law's fixed-row-count instance (a positive instance, cited never re-landed — the bucketing adds the localization axis, not the decomposition). vs sequential_ab_timing (the house tests/common discipline) — owns in-process interleaved A/B; the paired leg is its cross-process serving posture (whole-run sessions, the interleaving replaced by same-session pairing). REGIME: any lever-level claim whose gain is single-digit % on a bench with run-to-run drift (serving tok/s, frame time, compile time, epoch time); the pre-registration of the ratio (spread vs claimed gain) belongs in the plan before the runs. ANTI-REGIME: gains above the spread (the median is a fine instrument — the rule is conditional on the ratio, not a median ban); microbenchmarks inside one process with interleaved arms (the sequential-AB class owns that posture); A/B frameworks that already pair by construction (this rule is their decision rule, not a new harness); the medians are still PRINTED ('shown for continuity') — the law is they are not the verdict. Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R8: an epistemic class per published number ──
        PerfRule {
            rule_id: "measured-simulated-modelled-label-per-number".to_string(),
            name: "every published number carries its epistemic class — measured, simulated, or modelled — and unmeasured quantities never sit unlabelled beside measured ones, because the reader prices them the same ('Every number below is measured unless it says simulated or modelled.')".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r"// A results table mixing measurements with forward-looking
// estimates — none labelled. The reader cannot tell which
// numbers the hardware produced and which a spreadsheet did.
//
//   | idea                                    | result   |
//   |-----------------------------------------|----------|
//   | oracle re-ranker                        | +1.0%    |   <- simulated; unimplementable as written
//   | trained re-ranker                       | +0.17%   |   <- simulated
//   | certified head, 6-7 bit code            | -0.07 ms |   <- modelled; needs a new kernel
//   | shipped stack (three levers)            | +3.8%    |   <- measured
//
// The +1.0% oracle row reads exactly like the +3.8% shipped
// row. A roadmap priced off this table buys three kernels for
// gains a simulation promised and no measurement ever made.
".to_string(),
            fixed_code: r#"// FIXED: an epistemic class per figure, carried ON the row, and
// the class is decision-relevant: simulated = an upper bound from
// a model of the mechanism (often unimplementable as written —
// an oracle is not shippable); modelled = an estimate whose
// dependency is stated ('needs a new kernel'); measured = the
// only class a shipping decision may cite alone.
enum Class {
    Measured,
    /// an upper bound computed from a model of the mechanism
    Simulated,
    /// an estimate whose dependency is stated on the row
    Modelled { dependency: &'static str },
}

struct Row {
    idea: &'static str,
    result: f64,
    class: Class,
}

fn render(rows: &[Row]) -> String {
    let mut out = String::from("Every number below is measured unless it says simulated or modelled.\n");
    for r in rows {
        let tag = match r.class {
            Class::Measured => String::new(),
            Class::Simulated => " (simulated)".to_string(),
            Class::Modelled { dependency } => format!(" (modelled; {dependency})"),
        };
        out.push_str(&format!("| {} | {:.2}%{} |\n", r.idea, r.result, tag));
    }
    out
}
"#.to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT), docs/stack4.md:6 verbatim: 'Every number below is measured unless it says simulated or modelled.' + the in-table instances at :105 ('an oracle that always picks the right one of the two is worth +1.0 % (simulated); a trained re-ranker +0.17 % (simulated)') and :107 ('7 % fewer head bytes than the FP8 copy: about 0.07 ms (modelled); needs a new kernel'). THE LAW: an epistemic class per figure, and the classes are DECISION-RELEVANT, not decoration — a simulated gain is an upper bound on what an implementation could reach (their oracle is unimplementable as written; a trained re-ranker realizes a fraction), a modelled cost is an estimate whose missing dependency rides the label ('needs a new kernel' — the number is purchasable only at the kernel's price), and measured is the only class a shipping decision may cite alone. Unlabelled, the reader prices them all the same and a roadmap buys kernels for numbers a simulation promised. The default-with-exception form matters: stating 'measured unless labelled' makes the LABEL the burden of proof for every non-measurement — the inverse (label only the measured rows) lets an unlabelled estimate masquerade as the default. ⛔ ADJUDICATIONS (both directions): vs pava-monotone-cost-pooling-modeled-cost-not-clock (entries_19) — owns cost-MODEL construction (pool monotone costs instead of clocks, a modelling METHOD); THIS owns the LABEL law (which class a number belongs to and that the class is stated) — a modelled number correctly built by entries_19's method still needs this label, and a mislabelled measurement is not fixed by better modelling; compose. vs acceptance-band-derived-from-own-axis-measured-cv (entries_100) — owns band derivation from a measured CV; the label law is the minimum precondition (the axis must be measured to derive a band from it — an acceptance band on a simulated number is doubly unpriced). vs specdec-composite-decomposed-into-per-step-time-and-accepted-length (entries_84) — owns the decomposition axis; the label law rides beside any decomposition (each component inherits its own class). REGIME: any results table or report mixing forward-looking estimates with measured rows — roadmap tables, 'measured and not built' ledgers, modelled cost notes, ablation proposals, league projection rows. ANTI-REGIME: single-number claims (nothing to mislabel against); formally derived constants (a proof citation is its own class — math is neither measured nor simulated); the rule never forbids estimates — only UNLABELLED ones beside measured ones. Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R9: external figures at a pinned commit, best per row, dated,
        //    bracketed ours–kit–ours ──
        PerfRule {
            rule_id: "external-figures-read-at-pinned-commit-best-per-row-dated".to_string(),
            name: "external benchmark figures are read at a pinned commit (cloned, or fetched from the project's APIs — never from summaries), the competitor row is the best PER ROW across versions with a date and qualifiers (never one version's column labelled 'best published'), and any cross-kit comparison is bracketed ours–kit–ours with the drift ratio O2/O1 reported and investigated above 1% before publishing".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// A league table assembled from blog summaries; the competitor
// row is one version's column called "best published"; our own
// number measured once, months before the comparison.
fn league_table() -> String {
    // figure taken from a THIRD PARTY's summary of a table that
    // itself credits another project's numbers — no pin, no date,
    // no config label
    let competitor = 78.8; // "best published", from a summary

    // ours, measured once on a different day under unknown drift
    let ours = measure_once();

    format!("ours {ours} vs best published {competitor}")
}
"#.to_string(),
            fixed_code: r#"// FIXED: the class is carried ON the row; comparisons refuse
// across classes; the mechanism is stated (repeated prompts
// collapse the MoE expert union and share draft blocks — the
// per-layer distinct-expert count drops from ~53.6 to ~37.4 at
// one text's overlap).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PromptClass {
    /// a different prompt per user
    Distinct,
    /// prompts repeated — shares experts and drafts (~15% higher)
    Copies,
}

struct ThroughputRow {
    engine: &'static str,
    users: u32,
    tok_s: f64,
    class: PromptClass,
}

fn compare_within_class(a: &ThroughputRow, b: &ThroughputRow) -> Result<f64, String> {
    if a.class != b.class {
        return Err(format!(
            "cross-class comparison: {} ran {:?}, {} ran {:?} — a workload \
             substitution, not an engine delta; compare within class or \
             carry the ~15% class note on the row",
            a.engine, a.class, b.engine, b.class
        ));
    }
    Ok(b.tok_s / a.tok_s)
}
"#.to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT) — the source's own competitor review is the law's worked instance. Clause (a), docs/review/external.md:9-10 verbatim: 'Every external figure was read from its source at a pinned commit (cloned, or fetched from the GitHub and Hugging Face APIs), not from summaries.' THE CAUGHT MISATTRIBUTION is the motivating instance (:168): 'ZackO2o's cross-project table credits nero- with 81.8 / 76.7 / 78.8 tok/s; nero-'s archive (RESULTS-SGLANG-ARCHIVE.md) says those are Qwen3.8 TP2 numbers' — a summary-of-a-summary carried another project's config under the wrong name; only reading the source archive at its pin caught it. Clause (b), external.md:375-377, the M4 finding verbatim: 'M4. \"Best published\" is not the best per row, and has no date' — 'the best published figures on four GB10s at native precision are, row by row': the per-row maxima sit in DIFFERENT versions (a v2.2 figure beats the v2.4 column on some rows), and each carries its date and qualifiers; a single-version column under-reports the competitor and carries no date. Clause (c), docs/review/method.md:182-183 the bracket design (O1: ours before; the kit's session(s); O2: ours after — 'Ours–kit–ours shows drift as O2 − O1') and :438 verbatim: 'Ours is the mean of O1 and O2. Report O2/O1; above 1 % look for the cause before publishing.' — our own stability is measured AROUND the foreign number, so the published comparison carries its own drift budget. ⛔ ADJUDICATIONS (both directions): vs bench-ratio-omitted-when-baseline-not-live (entries_105) — owns the LIVE-BASELINE axis (the denominator measured on THIS box in THIS run; a missing baseline omits the ratio); THIS owns external-figure PROVENANCE (pinned commit, dated, config-labeled) plus the drift BRACKET that prices our own stability around the foreign number — compose: the live baseline is our arm, the bracket prices the whole comparison's day-scale drift, the pin+config label prices the foreign arm's identity. vs control-reproduces-on-baseline-trims-overclaim (entries_05) — owns the control-reproduces trim of an overclaim; the bracket is its temporal form (reproduce OUR side twice, report the spread) with the publishing gate (above 1%, investigate first). vs aa-control-prices-the-fidelity-instrument (entries_70 family) — the O1/O2 pair IS an A/A control on our side; this rule is its cross-kit application. REGIME: league tables, competitor comparisons, any published cross-project number, perf-rematch watch rows; the pin+date+config triple belongs on every foreign figure BEFORE it enters a table. ANTI-REGIME: our own A/B within one run (the interleaved class owns that posture — no bracket needed when both arms share the session); figures consumed from a live API WITH the pin recorded (the clause is the RECORDING, not the fetching); a single-vendor version sweep (the best-per-row law still applies — the column is one version's, the row maxima are the league's). Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R10: a multi-user figure names its prompt class ──
        PerfRule {
            rule_id: "copies-share-experts-and-drafts-distinct-is-the-honest-row".to_string(),
            name: "a multi-user throughput figure names its prompt class — distinct (a different prompt per user) or copies (repeated prompts, which share the MoE expert union and the draft blocks, ~15% higher) — and competitors compare only within class, because a copies-vs-distinct comparison is a workload substitution, not a win".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// A multi-user comparison table with no prompt class: our row
// ran a different prompt per user; the competitor row ran the
// same prompt repeated. The class gap is ~15% — larger than the
// "win" being claimed.
//
//   | engine        | users | tok/s | prompt class |
//   |---------------|-------|-------|--------------|
//   | ours          | 16    | 368   | (unstated)   |   <- distinct
//   | upstream      | 16    | 434   | (unstated)   |   <- copies
//
// Conclusion printed: "upstream serves 18% more" — a workload
// substitution priced as an engine deficit.
"#.to_string(),
            fixed_code: r#"// FIXED: the class is carried ON the row; comparisons refuse
// across classes; the mechanism is stated (repeated prompts
// collapse the MoE expert union and share draft blocks — the
// per-layer distinct-expert count drops from ~53.6 to ~37.4 at
// one text's overlap).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PromptClass {
    /// a different prompt per user
    Distinct,
    /// prompts repeated — shares experts and drafts (~15% higher)
    Copies,
}

struct ThroughputRow {
    engine: &'static str,
    users: u32,
    tok_s: f64,
    class: PromptClass,
}

fn compare_within_class(a: &ThroughputRow, b: &ThroughputRow) -> Result<f64, String> {
    if a.class != b.class {
        return Err(format!(
            "cross-class comparison: {} ran {:?}, {} ran {:?} — a workload \
             substitution, not an engine delta; compare within class or \
             carry the ~15% class note on the row",
            a.engine, a.class, b.engine, b.class
        ));
    }
    Ok(b.tok_s / a.tok_s)
}
"#.to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT), root README.md:47-48 verbatim: 'distinct = a different prompt per user; copies = prompts repeated (shares experts and drafts, ~15% higher). Upstream and sglang were only measured with copies.' — the class note and the competitor disclosure sit ON the headline table, not in a footnote. THE MECHANISM, docs/batching.md:555-556 verbatim: 'A session's drafts share its experts: 53.6 distinct per layer when sessions share none, and 37.4 at one text's overlap' — repeated prompts collapse the MoE expert union (every session routes to the same experts, so one expert set serves many users) and share draft blocks (overlapping text reuses drafter work); a copies figure is a DIFFERENT WORKLOAD with materially more throughput at the same hardware. THE MEASURED GAP: CHANGES.md:27 the distinct row ('16 users, a different prompt each': 368-376 -> about 413) vs CHANGES.md:333 the same-build pair ('16 users re-measured on the live build: 368 distinct, 436 copies (medians of 3)') — an ~18% class gap on identical hardware, larger than most claimed engine deltas. THE LAW: the class is carried ON the row; comparisons refuse across classes; where a competitor was measured in only one class (upstream and sglang: copies only), the mismatch is disclosed beside the row, never averaged away. ⛔ ADJUDICATIONS (both directions): vs unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately (entries_84) — owns the unique-prompts/PREFIX-CACHE axis (unique prompts defeat the cache; cold and warm reported separately); THIS owns the MoE-UNION + draft-sharing mechanism and the cross-class COMPARISON law (the row-labeling and refusal half) — the prefix-cache face of prompt repetition is entries_84's, the expert-union face and the comparison gate are THIS; compose (a copies number has BOTH a cache face and a union face). vs benchmark-input-distribution-realism (entries_94) — owns input DISTRIBUTION realism (zeros and random are both lies about cache behavior); the copies question is not synthetic-vs-real — both classes are real workloads — it is WHICH class a number describes; compose (a real copies number mis-used as a distinct comparison is entries_94-legal and still wrong). REGIME: any multi-tenant throughput figure on architectures with per-token expert selection (MoE) or content-addressed draft/prefix sharing; the class belongs on the row at capture time, not at publication time. ANTI-REGIME: single-user figures (one prompt — the class is trivially fixed); batch-size or concurrency claims (a different axis — rows per step, not prompt identity); the rule never says copies are invalid — only UNLABELED, or cross-compared without the class note; dense models without expert routing have no union face (their prompt-class gap rides entries_84's cache axis alone). Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R12: a digest that folds nothing equals its seed and passes
        //    forever — refuse loud ──
        PerfRule {
            rule_id: "vacuous-digest-equals-its-seed-refuse-loud".to_string(),
            name: "a reference digest equal to its hasher's seed/offset basis folded NOTHING and passes forever — refuse loud when a digest equals the fold's init constant and when a run processed zero rows, and scope-label regression hashes: they show a change left the output alone, never that the output is correct".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// A golden reference hash whose run can legitimately fold zero
// rows: the mode hashes only the rows after the prompt offset —
// and one golden command's offset consumes the whole file. The
// fold never runs; the digest IS the init constant; it "passes"
// forever.
const CHOSEN_HASH_INIT: u64 = 0x1465_0fb0_739d_0383;

fn reference_digest(mode: Mode, prompt_offset: usize, file: &File) -> u64 {
    let mut h = CHOSEN_HASH_INIT;
    if mode == Mode::TeacherForced {
        for row in file.rows_after(prompt_offset) {
            // the golden command sets limit == prompt_offset:
            // rows_after() is EMPTY — the loop never runs
            h = fold_row(h, row);
        }
    }
    h // == CHOSEN_HASH_INIT, "whatever the logits"
}

fn check_build(actual: u64, golden: u64) -> bool {
    actual == golden // both may be the seed: the check is vacuous
}
"#.to_string(),
            fixed_code: r#"// FIXED: refuse loud on the two vacuity witnesses — a digest
// equal to the fold's init constant (nothing was folded), and a
// run that processed zero rows (the digest would equal the seed
// whatever the data). The passing check also carries its scope:
// a regression hash shows a change left the output alone, never
// that the output is correct.
const CHOSEN_HASH_INIT: u64 = 0x1465_0fb0_739d_0383;

fn checked_digest(rows_folded: usize, payload: &[u8]) -> Result<u64, DigestError> {
    if rows_folded == 0 {
        return Err(DigestError::ZeroRows);
    }
    let d = fold(CHOSEN_HASH_INIT, payload);
    if d == CHOSEN_HASH_INIT {
        // generic: even with rows counted, a fold that returns its
        // init constant folded nothing the reader can price
        return Err(DigestError::SeedValued);
    }
    Ok(d)
}

#[derive(Debug)]
enum DigestError {
    /// the run folded nothing; the digest would equal the seed
    ZeroRows,
    /// the digest equals the hasher's init constant
    SeedValued,
}

fn check_build(actual: Result<u64, DigestError>, golden: u64) -> Result<bool, DigestError> {
    // scope note carried at the call site: this is a REGRESSION
    // check — "a change left the output alone" — never evidence
    // that the output is correct
    Ok(actual? == golden)
}
"#.to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class). FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT), docs/review/correctness.md:120-127 the M1 finding verbatim: 'the long28k CED reference hash checked nothing. One of the 7 hashes every change is held to, 14650fb0739d0383, comes from --mode tf ... --prompt 28489 --limit 28489' — the mode 'hashes only the rows after --prompt (main.cpp, the teacher-forced loop), and this run has none, so it prints the hash's seed (main.cpp:719, chosen_hash = 1469598103934665603ull, which is 0x14650fb0739d0383) whatever the logits' + 'The docs cite it as bit-level evidence about ten times' — one golden hash of seven checked nothing for the whole life of the doc set, while its reproduction ('the same command in exact mode ... prints the same hash although its first token's log-probability differs') proved it insensitive. THE FIX is upstream-landed (b47c510, docs/experiments.md:328: 'Adopted: 8f08a153d415c5c8 replaces the seed' — the run now 'hashes the prompt's sample (token, logit and logprob bits, top 20 logprobs' ids and bits)'). THE SCOPE NOTE, correctness.md:242-244 verbatim: 'The engine's own hashes and A/B comparisons only show that a change left the output alone; they are regression checks, not evidence of correctness.' THE LAW: (a) a digest equal to its hasher's seed/offset basis must REFUSE loud — it prices zero evidence whatever the data; (b) a run that processed zero rows must refuse BEFORE the digest is recorded; (c) regression hashes carry their scope — same-output-after-change, never correct-output. ⛔ ADJUDICATIONS (both directions): vs lane-split-self-consistency-bar-straddle-blind (entries_93) — owns the symmetric-corruption-cancels-in-self-consistency axis (a structural split whose pieces cross a converter blind every self-consistency bar); THIS owns the SEED-VALUED digest — a single-artifact vacancy (the fold never ran), not a cross-piece blindness; distinct sinks (converter boundary vs fold initialization). vs aa-control-prices-the-fidelity-instrument (entries_70 family) and the house green-zero class (cfg-gated empty binaries reporting ok. 0 passed) — those own instruments that cannot fail at the HARNESS/compile layer; THIS owns the same vacancy at the ARTIFACT layer (a reference value that carries no read of the data it certifies); the research note's dedup row records exactly this: the corpus has no seed-valued-digest rule — this is the new instance. vs work-removed-build-is-not-a-valid-lower-bound (entries_84) — owns absent-work in a TIMED arm; THIS owns absent-work in an INTEGRITY artifact; compose (both are the loud-zero family's members at different layers). REGIME: golden/reference hashes over runs that can legitimately fold zero rows (mode-gated loops, offset == limit, empty prompts, skipped sections); any fold whose init constant is reachable as a final value. ANTI-REGIME: the equality check alone is NOT the rule — a digest whose init value is a legitimate member of the folded domain needs the CONJUNCTION (digest == init AND a zero-row/zero-byte witness) or it false-refuses honest collisions; digests over always-nonempty inputs (the vacancy is unreachable — though the zero-row refusal is cheap insurance at the boundary). Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R13: pad the io tail to the device's interrupt threshold ──
        PerfRule {
            rule_id: "io-completion-batch-padded-to-interrupt-threshold".to_string(),
            name: "with nothing left to issue, round the reads outstanding up to a multiple of read_batch so the LAST completion trips the device's interrupt-coalescing count threshold instead of its timer (pad reads discarded via a user_data flag bit and counted), gated OFF while a deep queue keeps completions coming on its own".to_string(),
            category: "io".to_string(),
            buggy_code: r"// A decode-shaped reader over a coalescing device: the burst
// ends with 1-7 reads outstanding. The device signals a
// completion once 8 are pending OR its timer fires — so a tail
// of 1-7 waits the full timer latency, every step.
fn submit_reads(queue: &mut Queue, needed: &[Key]) {
    for k in needed {
        queue.read(k); // the burst: e.g. 5 reads issued
    }
    // nothing left to issue — the 5 outstanding reads wait for
    // the TIMER (the count threshold needs 8). The step pays the
    // timer latency on every decode step that reads.
    wait_all(queue);
}
".to_string(),
            fixed_code: r"// FIXED: with nothing more to issue, round the reads outstanding
// up to a multiple of read_batch — the last completion trips the
// SSD's interrupt threshold instead of its timer. Pad reads are
// discarded at completion via a user_data flag bit and counted.
// The gate: while misses are still pending (deep prefill queues)
// completions keep coming and no padding is needed.
const READ_BATCH: u32 = 8;
const K_PAD: u64 = K_DISCARD | 2; // user_data bit: a pad read

fn submit_reads(queue: &mut Queue, needed: &[Key]) {
    for k in needed {
        queue.read(k);
    }
    // With nothing more to issue, round the reads outstanding up
    // to a multiple of read_batch, so the last completion trips
    // the SSD's interrupt threshold instead of its timer.
    if READ_BATCH > 1 && queue.nothing_left_to_issue() {
        while queue.unseen() % READ_BATCH != 0 {
            queue.read_discardable(K_PAD); // counted separately (pads)
        }
    }
    wait_all(queue);
}
".to_string(),
            context_notes: Some("NotBenchable bench_spec None — a measurement-honesty rule (the validator prices code deltas, not bench-methodology defects; the dial9/136 annotation class); io lane — the completion-coalescing pad prices no code delta the validator can host. FULL PROVENANCE: Picom-code/pico-dgx-ds4 @ 922dddb1a35e521148022ee11264632da1f0ddc0 (MIT), engine/engram/store.cpp:367-369 the comment verbatim: 'With nothing more to issue, round the reads outstanding up to a multiple of read_batch, so the last completion trips the SSD's interrupt threshold instead of its timer. While misses are still pending (deep prefill queues) completions keep coming and no padding is needed.' + :34/:371 the discard plumbing ('constexpr uint64_t kPad = kDiscard | 2; // rounding a batch up to read_batch'; the pad issued through discard_read(kPad) with its own st_pads counter — pad reads are flagged in user_data and dropped at completion, never conflated with real reads); docs/engram.md:120-124 the drive behavior verbatim: 'The drive signals a completion once 8 are pending or 200 µs have passed, so 1–7 reads take 200 µs (p50 199.8 µs at queue depths 1, 2 and 4)' + the measured result 'On rank 3, steps that read went from p50 237 / p99 256 µs to 86 / 169 µs' + the gate's own census 'Deep prefill queues need no padding: storecheck issued 684 padding reads against 642k real ones' (0.1% — the pad is decode-tail traffic, not a standing tax). THE MECHANISM: interrupt coalescing (count threshold OR timer); a burst's tail of 1–7 outstanding reads pays the full timer; rounding the tail UP to the batch multiple makes the last pad completion push pending past the threshold, so the whole tail completes on the interrupt instead of the timer. THE GATE is load-bearing: padding fires only when the pending queue is EMPTY (nothing more to issue) — under a deep queue completions keep arriving and pad reads would be dead traffic. ⛔ KEEPALIVE ROUTING (the face lane, not this rule): the same store's worker spin (spin_us default 1 s; GB10 idle exit 231/433 µs) and its APST keepalive read (one random record per 5 ms idle; 1.1–9.5 ms state-exit costs) are KEEPALIVE — metal-keepalive-residency-unwire (entries_21) and spin-keep-awake-checked-capability-degrade-loud (entries_74) own that family; they are this source's SECOND instances and route to the face lane (F11 in Research 454), never re-owned here. THIS rule owns ONLY the pad-to-interrupt-threshold atom (grep: interrupt threshold / completion coalescing / pad-to-batch — zero corpus owners). REGIME: device/queue pairs with completion coalescing (count threshold + timer): NVMe queues, NIC interrupt moderation, batched completion rings — anywhere the LAST completion of a burst decides whether the waiter pays the timer, and the workload's bursts end just under the threshold. ANTI-REGIME: deep queues (completions keep coming; padding is dead traffic — the 684-vs-642k census is the gate's own evidence and the code comment's own condition); read_batch <= 1 (a threshold of one — nothing to round to); latency-critical single reads where an extra discarded read costs more than the timer it avoids (pad only when a burst's tail is already there); devices without count-based coalescing (timer-only — rounding changes nothing). Batch 427 / Plan 420 / Research 454.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
