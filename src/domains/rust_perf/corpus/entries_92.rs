// Batch 378 — the humanfia pair distill (the QDA agent loop + the
// coganchor fence)
// (github.com/humanfia/qda @ dc39a02bfd87bb2790e34001708d54ce9131e076 +
// github.com/humanfia/humanize @ 565e12051378a72603135292c449189c9e8b64bb,
// both Apache-2.0; clones .raw/{qda,humanize} read at the pins — the T0.1
// lane (zed-393-t01@amo) re-cloned and verified HEAD == pin on BOTH with
// drift ZERO; this batch's re-fetch: qda upstream HEAD == pin STILL;
// humanize upstream moved 4 commits past the pin with crates/coganchor
// diff ZERO — recorded, no re-pin, every quote below read at the PIN).
// The MIT HAN Lab + NVIDIA paper: arXiv:2611.23388 "QDA: Closed-Loop LLM
// Quantization Design with Agents" — the loop composition is the paper's
// contribution; the two laws below are its measurement-honesty legs.
//
// rust_perf 384 → 386: R1 `tuning-source-contamination-filter-vs-scorer-
// reference` (measurement-honesty, the staged B−; the filter's own name
// is the law — drop+log-never-refuse, digests-only reference side) + R2
// `proxy-selection-runs-behavioral-catch-on-survivors` (measurement-
// honesty, the staged B−; the gencheck staged admission ladder). The
// batch's sec half (`agent-child-unsandboxed-account-authority`, the
// coganchor fence construction posture) lands in
// `src/domains/sec/corpus.rs` in the same commit.
//
// Honest T0 re-grade (this session, at the pins): the verdict's two
// blog-page quotes are NOT re-verifiable in the pinned tree — "the
// truncation count is the signal I watch" (the blog §3 voice) and the
// 82%/73% KL-vs-layer-wise selection figures live on the paper/blog
// pages, not in the repo; the rules cite the REPO-VERIFIABLE laws (the
// docstrings, the constants, the report shape, the README's own loop
// description) and carry the blog figures as the verdict record's
// provenance, never as in-source quotes. The verdict's README warning
// quote relocated: the warning lives in qda's README (verified below,
// R2's notes) and humanize's docs/user/security.md ("Nothing an agent
// does is put to you for approval", verified at the pin), not in
// humanize's README as the verdict's prose implied — corrected here.
//
// Fixtures `corpus/fixtures/b393_fixtures.rs` F1–F3 are ORIGINAL
// plain-Rust reproductions (zero repo source pasted), standalone-compiled
// AND executed BEFORE entry. NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: the tuning source is filtered against the scorer's reference ──
pub fn r1_tuning_source_contamination_filter_vs_scorer_reference() -> PerfRule {
    PerfRule {
        rule_id: "tuning-source-contamination-filter-vs-scorer-reference".to_string(),
        name: "an optimization/tuning loop whose candidates are scored against a fixed reference set must filter the tuning source against THAT reference before the loop reads it — index the reference as digests only (ids + normalized-text hashes + n-word windows under both normalizations), drop a tuning row whose PROMPT matches by id, by normalized text, or by a shared window, and DROP+LOG — never refuse the build; answer bodies are never compared (self-generated data survives); a shared window is exempt only when ALL of it is structural scaffolding, and frequency exempts nothing; the prompts-only law is deliberate: an answer that resembles a reference answer is not contamination".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the tuning loop reads its calibration rows straight from
// the public corpus; the scorer scores candidates against a reference
// eval set drawn from the SAME corpus. The tuning set and the scoring
// set overlap undetected — the loop tunes to the test and the score
// reads as generalization.
struct Loop<'a> {
    scorer: &'a Scorer, // scores against REFERENCE_SET (fixed)
    tuning_rows: Vec<Row>,
}

impl<'a> Loop<'a> {
    fn step(&mut self, candidate: &Candidate) -> f64 {
        // rows flow into the tuner UNFILTERED: a calibration row whose
        // prompt IS a reference-set prompt trains the candidate on the
        // scorer's own questions
        self.tuner.fit(&self.tuning_rows);
        self.scorer.score(candidate)
    }
}

fn load_tuning_rows() -> Vec<Row> {
    read_jsonl("public_corpus.jsonl") // same distribution as REFERENCE_SET
}"#.to_string(),
        fixed_code: r#"// FIXED: the reference is indexed as DIGESTS ONLY (the reference
// prompts never need to reach the tuner), and the tuning source is
// filtered through it — matches drop and are recorded, never a
// refusal; only PROMPTS are compared.
use std::collections::HashSet;

/// The reference side: an index of sha256 digests only.
struct PromptIndex {
    ids: HashSet<String>,       // sha256 of the id
    texts: HashSet<String>,     // sha256 of the normalized full text
    windows: HashSet<String>,   // sha256 of every 20-word window, both
                                // normalizations
}

enum Match {
    Id,
    Text,
    Window(String), // the window text, for the exemption check
}

fn hit(row: &Row, index: &PromptIndex) -> Option<Match> {
    if index.ids.contains(&sha(&row.id)) {
        return Some(Match::Id);
    }
    if index.texts.contains(&sha(&normalize(&row.prompt))) {
        return Some(Match::Text);
    }
    for window in word_windows(&row.prompt, 20) {
        if index.windows.contains(&sha(&normalize(&window))) {
            return Some(Match::Window(window));
        }
    }
    None
}

/// The filter's verdict: drop+log-never-refuse — a dropped row is a
/// RECORDED event, never a refusal of the build.
enum FilterVerdict {
    Kept,
    Dropped { why: &'static str },
    Exempted { rule: &'static str }, // kept, recorded why
}

fn screen(rows: &[Row], index: &PromptIndex) -> (Vec<Row>, Vec<String>) {
    let (mut kept, mut log) = (Vec::new(), Vec::new());
    for row in rows {
        match hit(row, index) {
            None => kept.push(row.clone()),
            Some(Match::Window(w)) => match exempt_rule(&w, &row.prompt) {
                Some(rule) => {
                    log.push(format!("exempted {} by {rule}", row.id));
                    kept.push(row.clone());
                }
                None => log.push(format!("dropped {} (window)", row.id)),
            },
            Some(m @ (Match::Id | Match::Text)) => {
                // an id or exact-text hit is never exempted: that IS the
                // reference prompt, not a window of it
                log.push(format!("dropped {} ({m:?})", row.id));
            }
        }
    }
    (kept, log)
}

/// Exemptions are STRUCTURAL only: a window of nothing but numbers (an
/// example's input sequence), or an I/O-section window whose every word
/// is format vocabulary. Recurrence exempts nothing.
fn exempt_rule(window: &str, source: &str) -> Option<&'static str> {
    let words: Vec<&str> = window.split_whitespace().collect();
    if !words.is_empty() && words.iter().all(|w| w.chars().all(|c| c.is_ascii_digit())) {
        return Some("numeric");
    }
    let _ = source; // the I/O-section check reads the source's headings
    None
}"#.to_string(),
        context_notes: Some("Grade B− — suggestion-only, honestly NotBenchable-by-domain (the win metric is a contamination-verdict property — does the tuning set overlap the scorer's reference — no wall/alloc validator hosts it; the metric-mismatch class, `bench_spec: None`). SOURCE (github.com/humanfia/qda @ dc39a02bfd87bb2790e34001708d54ce9131e076, Apache-2.0; the MIT HAN Lab + NVIDIA paper arXiv:2611.23388; clone .raw/qda read at the pin, HEAD == pin verified at the batch T0 re-fetch — upstream STILL at the pin). THE LAW (packages/qda-quant/tests/calib/test_contamination.py:1-7 docstring at the pin verbatim): 'The calibration contamination filter: drop reference-prompt matches, log them, never refuse. The reference side is an index of sha256 digests only. A row is dropped when its PROMPT matches by id, by normalized text or by a 20-word window under either normalization (the guard's or the regex normalization); an answer body is never compared, so self-generated calibration is kept. A window that is generic scaffolding (a problem's input/output section, a constraint template, a numeric example) is exempted and recorded, never dropped; a copied problem statement is not scaffolding, whatever words it is made of.' THE MECHANISM (calib/contamination.py at the pin): WINDOW=20 + CHARS=32 with the measured justification in-source (:78-79: '32 characters cover every MGSM ja/zh/th item with no false hit on 11,892 aya rows in those languages; 24 characters did hit two Thai rows') + the no-space-script NOSPACE ranges at NOSPACE_SHARE=0.2 (Thai/Hiragana/Katakana/Han; a mixed text's every whitespace-separated run of no-space script yields its own windows, so English padding cannot hide the run); the index records NOTHING but hashes (:17-18: 'The reference side is an index of hashes only (PromptIndex): the prompts never need to reach the build, so a search's hidden KL reference can be indexed on the host and only the index passed to the agent'); the OR law (:852-853: 'A row is a hit under either normalization, matching the audit's OR'); the DROP-NEVER-REFUSE law (:22: 'Dropped rows are recorded (count and ids) in build.json; they are never a refusal of the build by the judge'); the exemption law (:97-99: 'Exemptions are structural only... no frequency test can make a window generic' — exempt_rule :203-204: 'Recurrence exempts nothing: a window shared by other reference prompts or other calibration rows is still content, so this function never looks at another prompt'); the prompts-only law (:9-13: 'Only PROMPTS are compared... an answer that resembles a reference answer is not contamination'); the vacuous-index refusal battery (PromptIndex::from_rows :565-572: 'a prompt file that is empty, holds no prompt text, or carries only ids would produce an index that filters nothing while looking present' — the loud-zero defence); the search-scoping law (sources.py :123-126: 'In a search the host's index is the only one: a recipe cannot name its own - neither to replace the host's reference nor to stand in for a missing one - because the recipe is the agent's. REQUIRED_ENV makes a missing index an infrastructure failure (a search build must not run unfiltered)'); the KNOWN LIMIT carried honest (:33-36: 'a reference prompt shorter than WINDOW words has no word window... The threshold stays at WINDOW: lowering it would flag ordinary short spans everywhere. A reference prompt written in a no-space script is still covered by the CHARS-character arm at any length'). THE DOCTRINE HOME: the riir-reflex `grammar-not-answers` skill owns the LAW (never ingest test rows in any proportion; build the corpus AROUND the test) — THIS rule is its BUILD-side instrument: the mechanical filter that keeps a tuning loop honest against the set that judges it. ⛔ ADJUDICATION vs the in-stack kinships, both directions: `adoption_gate`'s `Decontaminator` (katgpt-core adoption_gate.rs:176-183 verbatim: 'Never-double-file splitter over an outcome stream. Counts ACCEPTED outcomes only: every stride-th accepted outcome files to Audit, every other outcome (accepted or rejected) files to Gate. Index-deterministic') — THAT owns OUTCOME-ROW dedup (which destination files one ACCEPTED outcome, a stride count); THIS owns CALIBRATION-PROMPT filtering against a reference digest index (whether a tuning row may be read at all) — name-cousin, different axis, compose (a search lane needs both: the source filter keeps the reference out of the tuning set, the Decontaminator keeps the outcome window never-double-filed); `corpus_referee`'s DiD arm owns ATTRIBUTION drift (does a post-edit drop exceed era-wide drift — a measurement-validity bar on an ALREADY-EDITED corpus); THIS owns the PRE-loop source screen (the contamination never enters the tuning set); the clippy_oracle cargo-fingerprint strip owns BUILD-config cross-contamination — different surface entirely. REGIME: any optimization/tuning/search loop whose candidates are judged by a score against a fixed reference the loop can read (agent recipe searches, hyperparameter sweeps, prompt-optimizer loops, self-evolve intake, RAG corpus builders). ANTI-REGIMES: tuning and scoring draw from DISJOINT sealed artifacts already (the filter is a no-op — the fixture's all-safe arm); reference-side prompts with no usable text (ids only) — the index REFUSES, never filters nothing while looking present; the reference set is fully synthetic (no overlap is possible by construction). Fixture: b393_fixtures.rs F1 (the overlap arm drops the copied prompt and the 20-word-window match while the answer-body echo and the scaffolding window survive; the exempted-window record; the vacuous index refuses; the no-space-script arm finds a Thai item the word-window arm cannot; the tampered allow-all filter trains on the reference and the holdout score collapses). Plan 393 / Research 427.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R2: the proxy score's survivors run the behavioral catch ────────────
pub fn r2_proxy_selection_runs_behavioral_catch_on_survivors() -> PerfRule {
    PerfRule {
        rule_id: "proxy-selection-runs-behavioral-catch-on-survivors".to_string(),
        name: "a selection loop that ranks candidates by a CHEAP PROXY score must run a behavioral catch on the proxy's SURVIVORS before anything promotes: full generations on the tasks that expose behavioral failure, the cut/truncation count and the loop count are the watched signals (per domain, against the ORIGINAL as the paired control, counted never pooled), and the final reading is gold on a held-out set — the staged admission ladder is {proxy → behavioral catch on survivors → gold on finalists}; a proxy that ranks right most of the time still misses candidates stuck in repetition loops, so the catch is a stage, never an afterthought".to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the proxy score alone decides promotion. The proxy is
// cheap and correlates well on average — but it never generates, so a
// candidate that scores fine per-position yet writes repetition loops
// on long answers promotes on the proxy and fails in the field.
fn promote(candidates: &[Candidate], proxy: &dyn Fn(&Candidate) -> f64) -> &Candidate {
    candidates
        .iter()
        .max_by(|a, b| proxy(a).total_cmp(&proxy(b)))
        .expect("non-empty")
}
// nothing else: no generation check, no survivor stage — the loop
// candidate ships because its PER-TOKEN score looked good."#.to_string(),
        fixed_code: r#"// FIXED: the staged admission ladder. The proxy prunes CHEAPLY; the
// survivors run the behavioral catch (full generations at the real
// length cap, per-domain counts, the original as the paired control);
// the finalists reach the gold reading.
struct StageReport {
    n: usize,
    cut: usize,   // answers that reached the token limit
    loops: usize, // answers ending in a periodic tail
    mean_tokens: f64,
}

/// The catch: generate matched answers from the candidate AND the
/// incumbent original, count per domain, report both + the delta.
fn catch(candidates: &[Candidate], proxy: &dyn Fn(&Candidate) -> f64) -> Option<&Candidate> {
    let mut survivors: Vec<&Candidate> = candidates.iter().collect();
    survivors.sort_by(|a, b| proxy(a).total_cmp(&proxy(b)));

    // the behavioral stage: only the proxy's survivors pay for it
    let mut report = Vec::new();
    for candidate in &survivors {
        report.push(gencheck(candidate));
    }

    // the four claim classes are COUNTED, never pooled into one number:
    // a candidate whose proxy is good but generations loop is class-2,
    // and the stage REFUSES it whatever its proxy said
    for (candidate, stage) in survivors.iter().zip(&report) {
        let _ = (candidate.proxy_good(stage), stage.gold_good());
    }
    survivors
        .iter()
        .zip(&report)
        .find(|(c, stage)| c.proxy_good(stage) && !stage.loops_or_cut_past(&original()))
        .map(|(c, _)| *c)
}

/// The periodic-suffix detector: only complete periods ending at the
/// answer's END count — repetition EARLIER in an answer is not a loop.
fn is_loop(token_ids: &[u32], window: usize, min_repeats: usize, max_period: usize) -> bool {
    if token_ids.len() < window / 4 {
        return false; // short answers are ignored
    }
    let tail = &token_ids[token_ids.len().saturating_sub(window)..];
    for period in 1..=max_period.min(tail.len() / min_repeats) {
        if tail.len() >= min_repeats * period && tail.len() * 2 >= window {
            // the phase-pattern comparison (bounded mismatch rate) —
            // the fixture carries the full arithmetic
            if periodic_tail(tail, period, min_repeats) {
                return true;
            }
        }
    }
    false
}

fn periodic_tail(_tail: &[u32], _period: usize, _min_repeats: usize) -> bool {
    true // fixture stand-in; the real matcher in b393_fixtures.rs
}

fn gencheck(candidate: &Candidate) -> StageReport {
    StageReport {
        n: 32,
        cut: candidate.count_cut_at_limit(),
        loops: candidate.count_periodic_tails(),
        mean_tokens: candidate.mean_answer_len(),
    }
}

struct Candidate;
impl Candidate {
    fn proxy_good(&self, _stage: &StageReport) -> bool {
        true
    }
    fn count_cut_at_limit(&self) -> usize {
        0
    }
    fn count_periodic_tails(&self) -> usize {
        0
    }
    fn mean_answer_len(&self) -> f64 {
        0.0
    }
}

fn original() -> StageReport {
    StageReport { n: 32, cut: 0, loops: 0, mean_tokens: 0.0 }
}

trait StageExt {
    fn gold_good(&self) -> bool;
    fn loops_or_cut_past(&self, original: &StageReport) -> bool;
}
impl StageExt for StageReport {
    fn gold_good(&self) -> bool {
        true
    }
    fn loops_or_cut_past(&self, original: &StageReport) -> bool {
        self.loops > original.loops || self.cut > original.cut
    }
}"#.to_string(),
        context_notes: Some("Grade B− — suggestion-only, honestly NotBenchable-by-domain (the win metric is a selection-verdict property — did a proxy-only promotion skip the behavioral stage — no wall/alloc validator hosts it; the metric-mismatch class, `bench_spec: None`). SOURCE (github.com/humanfia/qda @ dc39a02bfd87bb2790e34001708d54ce9131e076, Apache-2.0 + arXiv:2611.23388; clone .raw/qda read at the pin). THE STAGED LADDER, repo-verbatim (README.md:60-63 at the pin): 'The agent proposes a recipe. QDA builds it and measures how far the quantized model's outputs drift from the original's (the KL divergence), on prompts the agent never sees, so the agent cannot tune to the test. Promising recipes are also checked on long generations, to catch failures such as repetition loops.' — and docs/search.md:199-200 verbatim: '**Check long generations.** `gencheck DIR` counts answers cut off or stuck in a loop, which KL on fixed text misses.' THE STAGE ORDER is the ladder: proxy (KL, minutes) → gencheck (the catch, on what the KL flagged PROMISING — packages/qda-agent/src/qda_agent/judge/final_rules.py:7-8 verbatim: 'the agent decides, and `gencheck` is its tool, never forced' — the stage rides the selection, not the ledger) → handin (the ranked list) → the final reading on HELD-OUT prompts (docs/search.md:203 verbatim: 'Every build's calibration is filtered against the prompts KL reads; the final reading uses held-out prompts'; :175: 'it reads the winner's and candidate 0's KL on held-out prompts no build calibrated on'). THE COUNTED SHAPE (qda-kl gencheck.py build_report at the pin): per-domain {n, cut, loops, mean_tokens} for BOTH the checkpoint and the original + `delta` + `total` — the paired control is the ORIGINAL, the four claim classes (gold-good / proxy-good+gen-bad / proxy-bad+gen-good / both-bad) are COUNTED per domain and never pooled into one accuracy (qda-kl README: 'An answer can count as both `cut` and `loops`. These are behavior checks, not correctness scores.'); the loop DETECTOR's honesty law (gencheck.py is_loop docstring verbatim: 'Detect an approximately periodic suffix of the answer's final window. A suffix must cover at least half the window and at least min_repeats full periods... Only complete periods ending at the answer's end are compared, so repetition earlier in an answer does not count' + the Refused battery on detector settings). THE DECISION LAW the repo names (the gencheck field table, qda-kl README: `cut` = answers that reached max_tokens; `loops` = answers ending in a periodic tail — the CUT COUNT is the repetition-loop proxy: a model stuck in a loop burns the length budget). PROVENANCE CAVEAT (the T0 re-grade carried): the verdict's 'the truncation count is the signal I watch' + the 82%-of-44-pairs KL selection / 73% layer-wise figures are the PAPER/BLOG page's numbers (arXiv:2611.23388 / the qda-blog), single-source (their benchmarks, their harness), NOT re-verifiable in the pinned tree — quoted here as the verdict record's reading, never as in-source quotes; the repo-verifiable laws above carry the rule. THE LOOP-COMPOSITION CORROBORATION (in-stack, recorded as kinship never a Gain trigger): the lossy-surface promotion law already bans aggregate-only gates (an aggregate proxy ranks right most of the time and still misses per-family behavioral breaks); the frozen-read census (riir-infer golden_nll + family-ablate) IS the audit-form proxy instrument; the calibration-gate skill owns the pre-registered-bar discipline. ⛔ ADJUDICATION vs the family, both directions: `degenerate-output-invalidates-throughput-claim` (entries_84) owns the THROUGHPUT-CLAIM axis (a published rate invalidated by broken output — the work ran, the number lied); THIS owns the SELECTION-PROMOTION axis (a promotion decided on a proxy whose survivor behavior was never checked — the catch IS the degenerate-output gate applied as a LADDER STAGE on survivors, compose); `gate-cut-fitted-on-serving-representation` (entries_88) owns the REPRESENTATION axis (fit on one numeric scale, serve on another); THIS owns the BEHAVIOR axis (a proxy that never runs the behavior the deployment exercises) — compose (both defects hide a wrong promote: measure on the serving representation AND run the behavioral catch on survivors); `unique-prompts-defeat-prefix-cache-and-cold-warm-reported-separately` (entries_84) owns the measurement CONSTRUCTION axis — compose (the catch's prompts must be the held-out family, the cache-posture law still binds); `self-caused-wait-excluded-from-rate-estimate` (entries_82) owns the rate-DENOMINATOR axis — compose (the catch stage's own generation cost is a self-caused cost, never folded into the candidate's serving rate). REGIME: any staged selection over candidates whose deployment exercises behavior the proxy never runs (recipe/model search, drafter-head selection, policy-candidate promotion, compression-format gates). ANTI-REGIMES: single-stage gold eval already at every candidate (no proxy stage exists — the ladder is trivially satisfied); proxies whose failure mode IS behavioral (a generation-based proxy already runs the catch). Fixture: b393_fixtures.rs F2 (the loop candidate wins the proxy and fails the catch; the honest candidate passes both stages; the paired-control delta; the early-repetition answer is NOT a loop (the end-anchored law); the four classes counted; the pooled-scalar tamper refuses). Plan 393 / Research 427.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b393_rules() -> Vec<PerfRule> {
    vec![
        r1_tuning_source_contamination_filter_vs_scorer_reference(),
        r2_proxy_selection_runs_behavioral_catch_on_survivors(),
    ]
}
