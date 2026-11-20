// Batch 374 — the laya 0.5.0-surface re-distill (the post-pin Rust port +
// serving tier)
// (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634,
// Apache-2.0; the pin is 2 commits past the v0.5.0 tag — Research 425's
// "0.4.35" label was the stale spelling, corrected at T0; clone `.raw/laya`
// read at the pin 2026-11-20, T0 two reader subagents — 25 direct quotes
// byte-verified, 8 record corrections carried into the corpus notes below).
//
// rust_perf 369 → 373: R1 `batch-mixed-length-pads-to-longest-group-by-length`
// (complexity, BENCHABLE — the batch's GOAT lead, the padded-work axis) +
// R2 `confidence-threshold-bucketed-by-answer-shape` (measurement-honesty,
// suggestion-only; carries the R3 tri-state gate face folded in-notes) +
// R5 `long-doc-window-aggregation-picks-deciding-window` (complexity,
// suggestion-only) + R6 `budget-room-from-constructed-artifact-not-
// configured-cap` (measurement-honesty, suggestion-only). The batch's sec
// half (R8 + R9 rules, R4 + R10 rider faces) lands in
// `src/domains/sec/corpus.rs` in the same commit.
//
// Honest T0 drops recorded here: R7 `unicode-classification-parity-across-
// language-ports` (C+/B−, thin — the standalone fixture cannot pull the
// `unicode-general-category` crate so the parity MECHANISM is
// unrepresentable without a hand-rolled category table, and entries_29's
// fixture already carries `char::is_alphabetic` as a second instance);
// the TileLang/graph/tokenizer-reuse/LRU KILLs recorded as dedup evidence
// in Research 425's own table (positive instances of the graph-fallback-
// coverage-ladder / capture-hygiene / bucket-ladder families + B361 kin +
// instinct-A7 kin, never re-graded).
//
// Fixtures `corpus/fixtures/b374_fixtures.rs` F1–F4 are ORIGINAL plain-Rust
// reproductions (zero repo source pasted), standalone-compiled AND executed
// BEFORE entry (11/11 green). NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: mixed-length batched passes pad to the pass max — group by length ──
pub fn r1_batch_mixed_length_pads_to_longest_group_by_length() -> PerfRule {
    PerfRule {
        rule_id: "batch-mixed-length-pads-to-longest-group-by-length".to_string(),
        name: "a batched pass pads EVERY row of the pass to the pass's LONGEST row, so mixed-length workloads pay the per-pass max on every row — a single pass has nothing to reorder, but a bounded-window stable sort by row length (within a small chunk-window lookahead, never a global sort) puts similarly-sized rows in the same chunk so each pass pads to a SHORTER maximum, and results are restored to input order; the per-row results stay bit-identical (the pad is inert), only the padded work shrinks".to_string(),
        category: "complexity".to_string(),
        buggy_code: r#"// NAIVE: rows are chunked in arrival order. Every pass pads all
// of its rows to the pass's LONGEST row — a 12-token row co-chunked
// with a 380-token row folds 380 columns of zero pad. On a
// mixed-length workload most passes carry both modes, so nearly
// every row pays a max it does not own.
fn batched_fold(rows: &[Row], chunk: usize, sink: &mut u64) -> Vec<u64> {
    let mut results = vec![0u64; rows.len()];
    for (start, pass) in rows.chunks(chunk).enumerate() {
        let start = start * chunk;
        let pass_max = pass.iter().map(|r| r.len).max().unwrap_or(0);
        for (i, r) in pass.iter().enumerate() {
            results[start + i] = fold_padded(r, pass_max, sink); // pads to pass_max
        }
    }
    results
}"#.to_string(),
        fixed_code: r#"// FIXED: within each window of 8 chunks, stably sort the rows by
// length, cut the sorted window into chunks, fold, and write each
// result back to its ORIGINAL index. Same work per row, shorter pad
// maxima, input order preserved — a single-pass caller still does
// nothing (one chunk has nothing to reorder; say so, don't pretend).
const SORT_WINDOW_BATCHES: usize = 8;

fn batched_fold_grouped(rows: &[Row], chunk: usize, sink: &mut u64) -> Vec<u64> {
    let mut results = vec![0u64; rows.len()];
    let window = chunk * SORT_WINDOW_BATCHES;
    let mut start = 0;
    while start < rows.len() {
        let end = (start + window).min(rows.len());
        let mut idx: Vec<usize> = (start..end).collect();
        idx.sort_by_key(|&i| rows[i].len); // stable: ties keep input order
        for group in idx.chunks(chunk) {
            let pass_max = group.iter().map(|&i| rows[i].len).max().unwrap_or(0);
            for &i in group {
                results[i] = fold_padded(&rows[i], pass_max, sink); // shorter max
            }
        }
        start = end;
    }
    results
}"#.to_string(),
        context_notes: Some("Grade B+ — the batch's GOAT lead, BENCHABLE (the padded-work axis is exact: work ∝ Σ_pass rows × pass_max vs Σ rows × len; the pad is inert so per-row results are bit-identical BY CONSTRUCTION — the only claim is the work axis, and the FP-drift caveat is the source's own, carried). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0 — 2 commits past the v0.5.0 tag; clone .raw/laya read at the pin, T0 2026-11-20, quotes byte-verified). THE LAW, laya-rust/src/batch_prediction.rs:33-36 at the pin verbatim: 'Group similarly sized encoded states within windows of eight batches, so each batch pads to a shorter maximum. Takes effect only with a `batch_size` greater than one and smaller than the number of states; otherwise it does nothing. Results keep input order. Changing the batch shapes can slightly change floating-point predictions near decision thresholds.' — and the mechanics, :126-128 verbatim: 'Without a reordering `sort_by_length` the windows are the chunks themselves. With one, each window spans eight chunks, its states are stably sorted by `row_len`, and the sorted states are cut into chunks.' The window bound is itself decision content, :17-19 verbatim: 'How many batches of states the `sort_by_length` lookahead spans. Bounds the tokenized lookahead independently of the input size, as Python does.' / 'const SORT_WINDOW_BATCHES: usize = 8;' — a BOUNDED lookahead, never a global sort (memory-bounded tokenization + bounded reordering scope). THE HONESTY RIDER (README:508-509, verbatim): a single pass 'says so on stderr rather than printing a same-speed result' — the rule does nothing at batch_size ≤ 1 and SAYS SO. THE FP CAVEAT is the source's own: batch-shape changes move near-threshold predictions — the rule serves throughput only where decisions are not at thresholds (the fixture keeps per-row outputs identical by reordering only). PRE-REGISTERED BARS: G1 per-row results bit-identical across arms (reorder-only; stable sort, input-order restore) — the fixture pins it; G2 `--release` ParetoDominant on ≥2 of 3 sizes [1K, 16K, 256K] rows with a MIXED-length distribution (bimodal + long-tail), never-slower on the third, and a UNIFORM-length control arm must Tie (the anti-regime is part of the fixture); G3 no regression; G4 alloc census stable. THE WORK MODEL IS REAL PADDED WORK (fill a padded buffer of pass-max length + fold it), never a cost model — a simulated cost is the falsifier that voids the cell. REGIME: mixed-length batched work of any kind (encoding, scoring, padded dispatch) at batch_size > 1; the uniform-length or single-pass workload is the Tie anti-regime. COMPOSITION VERDICT (plan 390 T1's named question — laya-apple's fixed-length-bucket ANE capture, Research 232's lane intel): the two compose as a TWO-REGIME PAIR, never a merge — R1 owns the DYNAMIC-pad regime (each pass pads to its own max, so a sort cuts the max) where the bucket-capture regime pads to FIXED shapes the sort cannot cut; there the same length signal moves to bucket ASSIGNMENT (smallest fitting bucket) and bucket derivation. Sort-then-bucket (reordering rows ACROSS bucket boundaries after assignment) would break shape stability for capture; sort-within-bucket is a padding no-op in the fixed-shape regime — neither spelling is the composition; the regimes stay separate and the length signal serves each in its own form. DISTINCT-FROM (T1 re-greps at HEAD, 3 vocabularies: sort.by.length/length.bucket/bucket.by.length/group.by.length = ONE kernel hit; pad.to.longest/longest.row/padding.waste = 3 kernel files, none the law; batch.window/lookahead/reorder = generic hits only): `persistent-kernel-length-bucketed-mechanism-ladder` (kernel_opt, B110) buckets rows by length to PICK A KERNEL MECHANISM per row with derived thresholds — THIS reorders ROWS to cut PADDING WASTE before chunked dispatch; different axis, compose not merge; `sorted-input-previous-row-fast-path` (entries_04) is the SORTED pole of run-locality (at pre-sorted keys the hash structure goes entirely) — THIS SORTS to cut pad waste, the input was unsorted by premise; `bounded-batch-buffer-pool` (entries_06) + `channel-send-batching-unpark` (entries_02) recycle buffers and batch wakes — neither reorders; `batch-hash-probe-prefetch-two-pass` (entries_77) is two-pass hashing over fixed-width keys — no pad axis. Fixture: b374_fixtures.rs F1 (the bit-identity pin + the pad-census cut + the uniform Tie + the tampered no-restore arm reds). Plan 390 / Research 425.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(batch_mixed_length_group_by_length_bench_spec()),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn batch_mixed_length_group_by_length_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
pub struct Input {
    lens: Vec<usize>, // per-row token counts (the mixed-length distribution)
    seed: u64,
    sink: u64,
}

pub fn make_input(n: usize) -> Input {
    // Deterministic bimodal + long-tail: every 16th row long, the rest short.
    let mut s = 0x243f_6a88_85a3_08d3u64;
    let mut lens = Vec::with_capacity(n);
    for i in 0..n {
        s = s.wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let len = if i % 16 == 0 {
            200 + (s >> 40) as usize % 200
        } else {
            4 + (s >> 45) as usize % 8
        };
        lens.push(len);
    }
    Input { lens, seed: s, sink: 0 }
}

fn fold_padded(len: usize, pass_max: usize, sink: &mut u64) -> u64 {
    // REAL padded work: touch every column of the pass-width buffer,
    // fold the row's own non-zero values (the pad is inert).
    let mut acc = 0u64;
    let mut h = len as u64 | 1;
    for i in 0..pass_max {
        if i < len {
            h = h.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            acc = acc.wrapping_add(h >> 33);
        }
        *sink += 1; // the padded column is touched regardless
    }
    acc
}

pub fn run_buggy(input: &Input) {
    let rows = &input.lens;
    let chunk = 32;
    let mut results = vec![0u64; rows.len()];
    let mut sink = 0u64;
    for (start, pass) in rows.chunks(chunk).enumerate() {
        let start = start * chunk;
        let pass_max = pass.iter().copied().max().unwrap_or(0);
        for (i, &len) in pass.iter().enumerate() {
            results[start + i] = fold_padded(len, pass_max, &mut sink);
        }
    }
    std::hint::black_box((results, sink));
}

pub fn run_fixed(input: &Input) {
    let rows = &input.lens;
    let chunk = 32;
    const SORT_WINDOW_BATCHES: usize = 8;
    let mut results = vec![0u64; rows.len()];
    let mut sink = 0u64;
    let window = chunk * SORT_WINDOW_BATCHES;
    let mut start = 0;
    while start < rows.len() {
        let end = (start + window).min(rows.len());
        let mut idx: Vec<usize> = (start..end).collect();
        idx.sort_by_key(|&i| rows[i]);
        for group in idx.chunks(chunk) {
            let pass_max = group.iter().copied().map(|i| rows[i]).max().unwrap_or(0);
            for &i in group {
                results[i] = fold_padded(rows[i], pass_max, &mut sink);
            }
        }
        start = end;
    }
    std::hint::black_box((results, sink));
}
"#
        .to_string(),
    }
}

// ── R2: one flat confidence cut cannot serve every answer shape ──
pub fn r2_confidence_threshold_bucketed_by_answer_shape() -> PerfRule {
    PerfRule {
        rule_id: "confidence-threshold-bucketed-by-answer-shape".to_string(),
        name: "an abstention/confidence gate thresholded on ONE field cannot serve every answer SHAPE — the same field orders OPPOSITE between checkpoints across option counts (a 3-option mean-max-p far below a 20-option's on one model, the reverse on another), so one flat cut abstains at a different accuracy target per shape: bucket the threshold by qtype × option-band (`choice:3-5`, `choice:11+`, … + one `default`), and echo the threshold THAT answer's bucket applied so logs re-split by gate".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: one global abstention threshold for every answer shape.
// The same confidence field runs at a DIFFERENT mean per shape (a
// 20-option argmax is far more peaked than a 3-option one), so this
// gate is simultaneously too loose on peaked shapes and too tight on
// flat ones — and nothing in the log says which regime an answer
// was gated under.
fn gate(confidence: f64, min_confidence: f64) -> bool {
    confidence >= min_confidence // one cut, every shape
}"#.to_string(),
        fixed_code: r#"// FIXED: bucket the threshold by qtype × option-band, fall through
// to a default, and ECHO the applied threshold with the verdict so
// logs re-split by the gate that actually ran.
fn bucket_key(qtype: &str, option_count: usize) -> String {
    let band = match option_count {
        0..=2 => "2",
        3..=5 => "3-5",
        6..=10 => "6-10",
        _ => "11+",
    };
    format!("{qtype}:{band}")
}

fn gate(
    confidence: f64,
    qtype: &str,
    option_count: usize,
    thresholds: &HashMap<String, f64>,
) -> GateOutcome {
    let applied = thresholds
        .get(&bucket_key(qtype, option_count))
        .or_else(|| thresholds.get("default"))
        .copied()
        .unwrap_or(0.0);
    GateOutcome {
        passed: confidence >= applied,
        abstention_threshold: applied, // the echo
    }
}"#.to_string(),
        context_notes: Some("Grade B — suggestion-only, honestly NotBenchable-by-domain (the win metric is an accuracy-at-abstention-target axis a wall/alloc validator cannot host — the metric-mismatch class, `bench_spec: None`). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0; clone .raw/laya read at the pin). THE MEASURED PREMISE — the source's own committed calibration study, recomputed at T0 from research/results/t4_colab_benchmark.json: checkpoint `laya` mean calibrated max-p on 3-option XNLI = 0.7677 vs 20-option MASSIVE = 0.9659; checkpoint `laya-multilingual` the OTHER WAY (0.9152 / 0.8055) — the SAME field orders OPPOSITE between checkpoints across option counts, so one flat cut abstains at a different accuracy target per shape AND per model. THE LAW (the Rust port's bucket table, laya-rust/src/confidence.rs:22-26 at the pin verbatim): 'The qtype half of a bucket key, in `common.temp_bucket`'s spelling.' / 'pub const BUCKET_QTYPES: [&str; 3] = [\"choice\", \"score\", \"noul\"];' / 'pub const BUCKET_SIZES: [&str; 4] = [\"2\", \"3-5\", \"6-10\", \"11+\"];' / 'pub const DEFAULT_BUCKET_KEY: &str = \"default\";' — twelve bucket keys (3 × 4) + the one non-bucket default, every bucket the map does not name falling through to it (never a refusal — the gate keeps ANSWERING). THE ECHO is the log axis, README:655-657 verbatim: 'comes back with `abstention` and `abstention_threshold` fields naming whether it fired' — the threshold ECHOED is the one THAT answer's bucket applied, so logs re-split by gate. THE MECHANISM is the same never-pool discipline as the per-group NLL/KL gates (kernel_opt Batch 339's TriAttention calibration lane corroborates: per-length-group gates, never pooled) — HERE the pool axis is answer SHAPE and the pooled artifact is a single threshold. REGIME: any confidence/abstention gate consumed across heterogeneous answer shapes (option-count spans, class-count spans, per-tenant score scales); thresholds fit per bucket at calibration time. ANTI-REGIMES: one shape only (a flat cut IS the right shape); shapes with no measured ordering difference (bucketing adds fitting burden for nothing — fit per shape only where the ordering actually splits); thresholds with no calibration data (the default fall-through answers, it never refuses). FACE (count-neutral, folded — R3 the tri-state gate outcome): the gate states are EXACTLY `passed` / `abstained` / `unevaluated` (confidence.rs:29-33, the const table), with TWO deliberate absences carried in the same law: (1) 'There is deliberately no member for \"no gate was configured\": with no `min_confidence` an answer reports nothing at all' (:35-36 verbatim — the gate is `None`, never a fabricated pass); (2) a non-finite confidence lands in `unevaluated` — the source's own test name pins it: 'an_answer_with_no_usable_confidence_is_unevaluated_not_flagged' (:401) — reporting it as a pass and reporting it as an abstain are BOTH wrong, so it is its own state. DISTINCT-FROM (T1 re-greps: confidence.bucket/threshold.bucket/abstention.threshold/option.band/option.count — one incidental file, no owner; never.pool/pooled.threshold — no threshold-pool owner): kernel_opt's per-length-group NLL/KL calibration gates (Batch 339) own the METRIC-reporting pool axis — THIS owns the THRESHOLD pool axis (a gate that decides, not a number that reports); `capacity-exceeded-silent-fallback-not-refusal` (sec) owns the lying-cap refusal — THIS deliberately answers-through-default (the fall-through IS the law, never a silent truncate). Fixture: b374_fixtures.rs F2 (the ordering-flip premise + the per-shape targets hit / flat cut misses in opposite directions + the echo + the default fall-through + the R3 face arms). Plan 390 / Research 425.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R5: long-doc window aggregation picks the deciding window ──
pub fn r5_long_doc_window_aggregation_picks_deciding_window() -> PerfRule {
    PerfRule {
        rule_id: "long-doc-window-aggregation-picks-deciding-window".to_string(),
        name: "a long document scored per window aggregates by PICKING the deciding window, never by averaging: a noul/statement lane takes the strongest window (any supporting window ⇒ true — one decisive span out-votes nineteen neutral ones), choice/score lanes take the MOST-CONFIDENT window (the mean lets the neutral majority out-vote the one window that saw the deciding span); the picked window's probability is the answer's — carried WITH per-window attribution (index, span, support count), never laundered into a fake whole-document number".to_string(),
        category: "complexity".to_string(),
        buggy_code: r#"// NAIVE: average the per-window scores. A 20-window document with
// ONE window over the deciding span: the mean read is 'false' at
// ~0.56 confidence — the neutral majority out-voted the window that
// actually saw the answer, and the number LOOKS like a whole-document
// probability when it is no such thing.
fn aggregate(windows: &[WindowScore]) -> (bool, f64) {
    let n = windows.len() as f64;
    let mean_conf = windows.iter().map(|w| w.confidence).sum::<f64>() / n;
    let mean_support = windows.iter().filter(|w| w.supports).count() as f64 / n;
    (mean_support > 0.5, mean_conf)
}"#.to_string(),
        fixed_code: r#"// FIXED: pick the DECIDING window and carry its attribution. noul:
// the strongest window (any supporting window ⇒ true — the max
// evidence read decides, first window on ties). choice/score: the
// most-confident window. The answer's probability IS that window's —
// disclosed as such, never a calibrated whole-document number.
fn aggregate(windows: &[WindowScore]) -> Decision {
    let best = windows
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.confidence.total_cmp(&b.1.confidence))
        .map(|(i, w)| (i, w))
        .expect("at least one window");
    Decision {
        supports: best.1.supports,
        confidence: best.1.confidence, // the DECIDING window's, not a document mean
        window: WindowAttribution {
            index: best.1.index,
            span: best.1.span, // token_start/token_end — where the answer came from
            count: windows.len(),
        },
    }
}"#.to_string(),
        context_notes: Some("Grade B− — suggestion-only, honestly NotBenchable-by-domain (decision-correctness class: the harm is a FLIPPED answer, not a wall/alloc delta — the metric-mismatch class, `bench_spec: None`). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0; clone .raw/laya read at the pin). THE LAW, README:1156-1158 at the pin verbatim: '`noul` takes the strongest window (the statement holds if any window supports it).' / '`choice` / `score` take the most-confident window — averaging over a long, mostly-neutral document lets the neutral majority out-vote the one window that saw the deciding span.' — and the implementation, laya-rust/src/long_prediction.rs:316-318 verbatim: 'Collapse per-window results into one: for each question the window with the greatest evidence (P(true) for noul, `answer_confidence` for choice and score) wins, the first window on ties.' THE HONESTY RIDER is the second half of the law, README:1189-1191 verbatim: 'The returned probability is the deciding window's, **not a calibrated number for the whole document** — a `noul` max drifts up with the window count even with no signal, and `choice` can land on a confidently-neutral window when nothing is decisive. Each answer carries `answer[\"window\"]` (the deciding window's `index`, `token_start`/`token_end`, and `count`) so you can check the span the answer actually came from' — attribution (index + span + count) rides WITH the pick; the max-with-window-count drift is DISCLOSED, never laundered. MECHANISM: per-window scoring over a long input is a disjunction for noul (one decisive span proves the statement) and an argmax for choice/score (the best-evidence read decides); a mean is the wrong aggregator for BOTH because windows are evidence, not voters. REGIME: long inputs scored per window/chunk and folded into one answer (long-doc classification, multi-span verification, paginated review). ANTI-REGIMES: lanes whose windows genuinely VOTE (the aggregation semantics ARE a mean — e.g. an averaged regression target); single-window inputs (the pick degenerates to the only window — no change); consumers needing a calibrated whole-document number (the pick's confidence is window-conditional — the source's own disclosure is the law, re-calibrate downstream instead of relabeling). DISTINCT-FROM (T1 re-greps: deciding.window/most.confident.window/window.aggregat/strongest.window/long.document — zero corpus owners; the kernel 'window' hits are attention-window mechanics, a different domain): `compaction-tail-boundary-prefers-complete-turn` (entries_06) picks a truncation BOUNDARY for a summary — THIS picks an ANSWER among scored candidates; `topk`/argmax family rules own the selection primitive — THIS owns the AGGREGATION semantics over windowed evidence (that a max must REPLACE a mean, and what attribution must ride with it). Fixture: b374_fixtures.rs F3 (the 19-neutral-vs-1-decisive flip pin + the attribution + the most-confident-not-first arm). Plan 390 / Research 425.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R6: the state room is what the built head leaves, not what the cap claims ──
pub fn r6_budget_room_from_constructed_artifact_not_configured_cap() -> PerfRule {
    PerfRule {
        rule_id: "budget-room-from-constructed-artifact-not-configured-cap".to_string(),
        name: "a budget derived from a CONFIGURED cap (`max_len - head_max_len`) is not the room the artifact actually leaves: per-option token floors make the built head grow BEYOND its cap past ~cap/4 options (77 options → head 319; 100 options → head 411 > the 320 cap), so a state sized to the cap-based figure is TRUNCATED, not merely conservative — and for a question well under the cap the same figure UNDERSTATES the room and discards state the model would have read: size from the CONSTRUCTED head for the question actually asked".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the state budget from the CONFIGURED cap.
// `head_max_len` is what the config SAYS the head may cost — not
// what a given question's head DOES cost. Two failure directions
// at once: the small question wastes room; the many-option question
// truncates state mid-budget.
fn state_budget() -> usize {
    MAX_LEN - HEAD_MAX_LEN // the cap-based figure: 512 - 320 = 192
}

fn build_question_state(state: &[Token]) -> Vec<Token> {
    let room = state_budget();
    state[..room.min(state.len())].to_vec() // truncates on big questions
}"#.to_string(),
        fixed_code: r#"// FIXED: size from the head that was BUILT for THIS question —
// per-option floors summed, then the room is what remains. The cap
// stays as an admission bound; the room figure is the construction's.
fn head_len_for(options: usize, instruction: usize) -> usize {
    let per_option = 4usize; // the option token FLOOR — options are never free
    (4 + instruction + options * per_option + 2).max(HEAD_MAX_LEN.min(HEAD_MAX_LEN))
        .max(8 + options * per_option) // the head keeps its own floor
}

fn state_room_for(options: usize, instruction: usize) -> usize {
    MAX_LEN.saturating_sub(head_len_for(options, instruction))
}

fn build_question_state(state: &[Token], options: usize, instruction: usize) -> Vec<Token> {
    let room = state_room_for(options, instruction);
    state[..room.min(state.len())].to_vec() // the room the head actually left
}"#.to_string(),
        context_notes: Some("Grade B− — suggestion-only, honestly NotBenchable (size-axis class: the harm is a TRUNCATED state / wasted budget, absent from a wall/alloc validator — the metric-mismatch class, `bench_spec: None`). SOURCE (NandhaKishorM/laya @ 416dd9ed20da2d43fc40e23712e9adceefabf634, Apache-2.0; clone .raw/laya read at the pin). THE LAW, README:1907-1911 at the pin verbatim: '`max_len - head_max_len` is **not a safe figure in either direction**, so size a state to `max_len - head_len - 1` for the question you are actually asking' / 'For a question that does not fill the cap it **understates** the room … conservative, but it discards state the model would have read.' / 'For a high-cardinality question it **overstates** it. `per = max(4, (head_max_len - 16) // k)` floors at 4 tokens per option and `head_ids` keeps a floor of 8, so past roughly `head_max_len / 4` options the head grows *beyond* the cap. A state sized to the cap-based figure is then **truncated**, not merely conservative' — with the source's own measured table (README:1913-1915): `laya` 77 options → head 319, real room 192 vs the cap-based figure 320; 100 options → head 411, real room 100 vs 320; `laya-multilingual` 100 options → head 411, real room 612 vs 768. THE REGIME BRIDGE is the source's own next sentence (README:1917 verbatim): 'The crossover is at `k ≈ head_max_len / 4` and the gap widens from there; `MAX_CHOICE_OPTIONS = 100` in `laya/serve.py` puts the overstating regime within reach over HTTP' — the amplification guard bounds what callers can send, and the constructed-room figure is what the server sizes from. THE GENUS is len_derived (a budget derived from a DECLARED figure vs from what was BUILT) — katgpt-rs's `len_derived_binding_audit` is the workspace AUDIT instrument for the kernel-binding face (a dimension derived from a buffer's declared size); THIS is the budget-face of the same genus in plain Rust: any `X_max - Y_cap` room/space/reserve figure where Y's actual construction carries per-unit floors. Adjudicate in-rule both directions: the audit instrument operates on kernel BINDINGS (a shape derived from a buffer length — silent OOB reads); THIS operates on BUDGET arithmetic (a room derived from a cap — silent truncation or silent waste); neither subsumes the other, the shared law is 'derive from the constructed artifact, not the declared configuration'. REGIME: any budget/room/reserve arithmetic fed from a config cap where the consuming side's size carries per-unit floors (prompt/state assembly, frame budgets, buffer reservations, quota math). ANTI-REGIMES: the cap IS the construction (a buffer truly clamped to the cap — the figure is exact); few-unit shapes where the two figures agree (the fixture's anti-regime arm pins the agreement); budgets re-derived per call with no stored claim (the truncation risk needs the figure to CROSS a boundary). DISTINCT-FROM (T1 re-greps: state.room/window.budget/cap.based/constructed.artifact — entries_45's hits are fixture internals, zero owners; the len_derived sweep family is audit tooling, not corpus): `capacity-exceeded-silent-fallback-not-refusal` (sec) owns the LYING-CAP refusal face (a cap that silently drops) — THIS owns the room ARITHMETIC fed by an honest cap that the construction can exceed; `work-buffer-sized-by-execution-width-not-plan-tasks` (annotated) sizes from execution width — the same 'built not declared' pole on the kernel-buffer axis, compose. Fixture: b374_fixtures.rs F4 (the 90-option truncation pin + the few-options agreement + the tampered floor-0 estimate). Plan 390 / Research 425.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b374_rules() -> Vec<PerfRule> {
    vec![
        r1_batch_mixed_length_pads_to_longest_group_by_length(),
        r2_confidence_threshold_bucketed_by_answer_shape(),
        r5_long_doc_window_aggregation_picks_deciding_window(),
        r6_budget_room_from_constructed_artifact_not_configured_cap(),
    ]
}
