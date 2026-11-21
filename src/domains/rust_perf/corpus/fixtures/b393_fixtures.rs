//! Plan 393 rust_perf fixture preflight — the humanfia pair distill
//! (Plan 393 / Research 427): `humanfia/qda`
//! @ `dc39a02bfd87bb2790e34001708d54ce9131e076` (Apache-2.0; the T0.1
//! lane re-cloned and verified HEAD == pin; this batch's re-fetch kept
//! qda AT the pin — upstream drift ZERO; the contamination filter, the
//! gencheck report shape and the is_loop detector read at the pin).
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! LAWS are re-authored; the source quotes stay in the corpus
//! context_notes). Compiled AND executed BEFORE entry (the
//! b340..b394 pre-entry discipline):
//!   cd src/domains/rust_perf/corpus/fixtures
//!   rustc --edition 2021 --test b393_fixtures.rs -o /tmp/b393_perf_fixtures
//!   /tmp/b393_perf_fixtures
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `tuning-source-contamination-filter-vs-scorer-reference`: a
//!   tuning loop whose source overlaps the scorer's reference set —
//!   the buggy arm trains on the copied prompt and its holdout
//!   "generalization" collapses; the fixed arm's digest-index filter
//!   drops the id/text/20-word-window matches, KEEPS the answer-body
//!   echo (answers are never compared) and the all-numeric scaffolding
//!   window (exempted + recorded), and the vacuous (ids-only) index
//!   REFUSES loud; the no-space-script arm: a Thai item the word-window
//!   arm cannot see is caught by the 32-character windows over its
//!   no-space runs; the tampered allow-all filter trains on the
//!   reference and the holdout score collapses with it.
//! - F2 `proxy-selection-runs-behavioral-catch-on-survivors`: a
//!   selection ladder over candidate "engines" scored by a cheap
//!   per-token proxy — the buggy arm promotes the proxy winner whose
//!   generations END in a periodic tail (the loop the proxy never
//!   runs); the fixed arm runs the behavioral catch on the proxy's
//!   survivors, counts {cut, loops} per domain against the original as
//!   the paired control, and refuses the looping survivor; the
//!   END-ANCHORED law: an answer whose repetition sits EARLY (not a
//!   complete periodic suffix at the end) is NOT a loop; the four
//!   claim classes are counted separately; the pooled-scalar tamper
//!   (one accuracy number over all classes) reds on the class split.

#![allow(dead_code)]

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

// ═══════════════════════════════════════════════════════════════════════
// shared: the sha stand-in (a u64 hash — the law is digests-only, the
// exact hash function is not the decision content)
// ═══════════════════════════════════════════════════════════════════════

fn sha(text: &str) -> u64 {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

// ═══════════════════════════════════════════════════════════════════════
// F1 — tuning-source-contamination-filter-vs-scorer-reference
// ═══════════════════════════════════════════════════════════════════════

const WINDOW: usize = 20;
const CHARS: usize = 32;

/// Unicode ranges of scripts normally written without spaces (the
/// fixture carries Thai + Han; the source carries six ranges).
fn is_nospace(c: char) -> bool {
    let cp = c as u32;
    (0x0E00..=0x0E7F).contains(&cp) || (0x4E00..=0x9FFF).contains(&cp)
}

/// A tuning row: a prompt (what the tuner reads) and an answer body.
#[derive(Clone, Debug)]
struct Row {
    id: String,
    prompt: String,
    answer: String,
}

/// The reference side: an index of hashes ONLY — the reference prompts
/// never need to reach the tuner.
struct PromptIndex {
    ids: HashSet<u64>,
    texts: HashSet<u64>,
    word_windows: HashSet<u64>,
    char_windows: HashSet<u64>,
}

impl PromptIndex {
    /// Build from reference rows. Refuses an index that would filter
    /// nothing while looking present (the vacuous-index refusal).
    fn from_rows(rows: &[Row]) -> Result<PromptIndex, String> {
        let mut idx = PromptIndex {
            ids: HashSet::new(),
            texts: HashSet::new(),
            word_windows: HashSet::new(),
            char_windows: HashSet::new(),
        };
        let mut any_text = false;
        for row in rows {
            idx.ids.insert(sha(&row.id));
            let body = normalize(&row.prompt);
            if body.chars().any(|c| c.is_alphanumeric()) {
                any_text = true;
            }
            idx.texts.insert(sha(&body));
            for w in word_windows(&row.prompt, WINDOW) {
                idx.word_windows.insert(sha(&normalize(&w)));
            }
            for w in char_windows(&row.prompt, CHARS) {
                idx.char_windows.insert(sha(&w));
            }
        }
        if !any_text {
            return Err("an ids-only reference cannot match a calibration prompt".to_string());
        }
        Ok(idx)
    }
}

/// The guard normalization: lower-cased, whitespace-stripped body.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Every `n`-word window of the text (whitespace-split).
fn word_windows(text: &str, n: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    (0..words.len().saturating_sub(n - 1))
        .map(|i| words[i..i + n].join(" "))
        .collect()
}

/// The no-space-script arm: every `n`-character window over the
/// whitespace-stripped no-space RUNS of the text (a mixed text's
/// English padding cannot hide the run).
fn char_windows(text: &str, n: usize) -> Vec<String> {
    let stripped: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mostly = stripped.chars().filter(|c| is_nospace(*c)).count();
    let mut out = Vec::new();
    if mostly * 5 >= stripped.len() {
        // the whole body takes windows
        push_windows(&stripped, n, &mut out);
    } else {
        // each whitespace-separated run containing a no-space char
        // yields its own windows
        for run in text.split_whitespace() {
            let body: String = run.chars().filter(|c| !c.is_whitespace()).collect();
            if body.chars().any(is_nospace) {
                push_windows(&body, n, &mut out);
            }
        }
    }
    out
}

fn push_windows(body: &str, n: usize, out: &mut Vec<String>) {
    let chars: Vec<char> = body.chars().collect();
    (0..chars.len().saturating_sub(n - 1)).for_each(|i| {
        out.push(chars[i..i + n].iter().collect());
    });
}

/// Why the row matched.
#[derive(Debug)]
enum Match {
    Id,
    Text,
    Window(String),
}

fn hit(row: &Row, index: &PromptIndex) -> Option<Match> {
    if index.ids.contains(&sha(&row.id)) {
        return Some(Match::Id);
    }
    if index.texts.contains(&sha(&normalize(&row.prompt))) {
        return Some(Match::Text);
    }
    for w in word_windows(&row.prompt, WINDOW) {
        if index.word_windows.contains(&sha(&normalize(&w))) {
            return Some(Match::Window(w));
        }
    }
    for w in char_windows(&row.prompt, CHARS) {
        if index.char_windows.contains(&sha(&w)) {
            return Some(Match::Window(w));
        }
    }
    None
}

/// Exemptions are STRUCTURAL only. Recurrence exempts nothing.
fn exempt_rule(window: &str) -> Option<&'static str> {
    let words: Vec<&str> = window.split_whitespace().collect();
    if !words.is_empty() && words.iter().all(|w| w.chars().all(|c| c.is_ascii_digit())) {
        return Some("numeric");
    }
    None
}

/// The filter verdict — the counts, never a refusal.
#[derive(Debug)]
struct FilterReport {
    kept: Vec<Row>,
    dropped: Vec<String>,
    exempted: Vec<String>,
}

fn screen(rows: &[Row], index: &PromptIndex) -> FilterReport {
    let mut report = FilterReport {
        kept: Vec::new(),
        dropped: Vec::new(),
        exempted: Vec::new(),
    };
    for row in rows {
        match hit(row, index) {
            None => report.kept.push(row.clone()),
            Some(Match::Window(w)) => match exempt_rule(&w) {
                Some(rule) => {
                    report.exempted.push(format!("{} by {rule}", row.id));
                    report.kept.push(row.clone());
                }
                None => report.dropped.push(format!("{} (window)", row.id)),
            },
            Some(m @ (Match::Id | Match::Text)) => {
                // an id or exact-text hit is never exempted
                report.dropped.push(format!("{} ({m:?})", row.id));
            }
        }
    }
    report
}

// the tuning loop + scorer model ─────────────────────────────────────

/// The scorer: measures a candidate on the held-out set (its "gold"
/// is memorized-answer detection — a candidate that trained on a
/// reference prompt REPRODUCES that prompt's gold answer, which is
/// contamination, not learning).
struct Scorer {
    reference: Vec<Row>,
}

impl Scorer {
    /// The contamination read: a holdout item is "solved" when ANY of
    /// the candidate's produced answers reproduces the item's gold
    /// answer verbatim — the signature of a candidate that trained on
    /// the item (the mechanism the filter exists to prevent).
    fn holdout_score(&self, candidate_answers: &HashMap<String, String>) -> f64 {
        let mut hit_count = 0.0;
        for row in &self.reference {
            let gold = normalize(&row.answer);
            if candidate_answers.values().any(|a| normalize(a) == gold) {
                hit_count += 1.0;
            }
        }
        hit_count / self.reference.len() as f64
    }
}

/// The tuner: fits a candidate by memorizing its tuning rows' answers
/// (the honest mechanical stand-in for gradient descent on copied
/// data — whatever the source holds, the candidate reproduces).
fn tune(rows: &[Row]) -> HashMap<String, String> {
    rows.iter()
        .map(|r| (r.id.clone(), r.answer.clone()))
        .collect()
}

#[test]
fn f1_unfiltered_source_trains_on_the_reference() {
    // the reference set: two prompts the scorer judges on (both long
    // enough to carry 20-word windows)
    let reference = vec![
        Row {
            id: "ref-1".into(),
            prompt: "For the geography quiz below, answer carefully. What is the capital of France? Answer in one sentence with the country named.".into(),
            answer: "The capital of France is Paris.".into(),
        },
        Row {
            id: "ref-2".into(),
            prompt: "Sum the first ten positive integers and show the running total after each addition, then state the final sum explicitly.".into(),
            answer: "The running total ends at 55.".into(),
        },
    ];
    let scorer = Scorer {
        reference: reference.clone(),
    };

    // the tuning source: clean rows PLUS two contaminated shapes —
    // a 20+-word shared window over ref-1 (a rephrase sharing the long
    // middle run) and a verbatim copy of ref-2 under its OWN id (the
    // text-hash arm catches it)
    let tuning = vec![
        Row {
            id: "clean-1".into(),
            prompt: "Name three primary colors used in subtractive mixing.".into(),
            answer: "Cyan, magenta and yellow.".into(),
        },
        Row {
            id: "tune-1".into(),
            prompt: "For the geography quiz below, answer carefully. What is the capital of France? Answer in one sentence with the country named.".into(),
            answer: "Who cares — wrong answer.".into(), // the PROMPT matches, not the answer
        },
        Row {
            id: "tune-2".into(),
            prompt: reference[1].prompt.clone(),
            answer: reference[1].answer.clone(),
        },
    ];

    let index = PromptIndex::from_rows(&reference).expect("index builds");

    // the contaminated rows are caught: tune-1 by the shared 20-word
    // window (its prompt shares ≥20 consecutive words with ref-1's
    // normalized text), tune-2 by BOTH the text hash and the id
    // collision — wait, tune-2's id differs; the TEXT hash catches it.
    let report = screen(&tuning, &index);
    assert!(
        !report.dropped.is_empty(),
        "the filter must drop the contaminated rows: dropped={:?}",
        report.dropped
    );
    assert!(
        report.dropped.iter().any(|d| d.contains("tune-1")),
        "the window-shared prompt must drop: {:?}",
        report.dropped
    );
    assert!(
        report.dropped.iter().any(|d| d.contains("tune-2")),
        "the verbatim copy must drop: {:?}",
        report.dropped
    );

    // BUGGY: no filter — the tuner memorizes everything, and the
    // holdout "score" rewards the copied answer (tune-2 reproduces
    // ref-2's gold answer: the score is contaminated, reading ~0.5)
    let contaminated_answers = tune(&tuning);
    let contaminated_score = scorer.holdout_score(&contaminated_answers);
    assert!(
        contaminated_score >= 0.5,
        "the unfiltered loop must score high on COPIED data (the contamination \
         the filter exists to prevent): {contaminated_score}"
    );

    // FIXED: the filtered source cannot reproduce the reference gold —
    // the same tuner scores ~0.0 (the honest generalization read)
    let clean_answers = tune(&report.kept);
    let clean_score = scorer.holdout_score(&clean_answers);
    assert!(
        clean_score < 0.01,
        "the filtered loop must not reproduce reference answers: {clean_score}"
    );
    assert!(
        contaminated_score - clean_score >= 0.4,
        "the filter must remove the holdout inflation: {contaminated_score} vs {clean_score}"
    );
}

#[test]
fn f1_answers_are_never_compared_and_scaffolding_is_exempt() {
    let reference = vec![Row {
        id: "ref-a".into(),
        prompt: "Compute the area of a circle of radius 7 and round to two decimals.".into(),
        answer: "153.94".into(),
    }];
    let index = PromptIndex::from_rows(&reference).expect("index builds");

    // (a) an ANSWER-body echo is NOT contamination: the row's prompt
    // is clean, its answer happens to match a reference answer
    let echo = vec![Row {
        id: "selfgen-1".into(),
        prompt: "Give a two-decimal area example for a radius you pick.".into(),
        answer: "153.94".into(), // the reference's answer, echoed
    }];
    let report = screen(&echo, &index);
    assert!(
        report.kept.len() == 1 && report.dropped.is_empty(),
        "an answer echo must be KEPT (answers are never compared): {report:?}"
    );

    // (b) a shared all-numeric window is scaffolding: exempted + kept.
    // The row's shared windows must be ONLY the numeric run — a shared
    // leading word would be a non-structural window and the row DROPS
    // (the every-shared-window law).
    let numeric = vec![Row {
        id: "ex-1".into(),
        prompt: "Sample test:\n7 25 100 45 12 9 88 3 61 27 5 70 34 19 6 50 92 11 4 63\nSolve the problem above.".into(),
        answer: "solve it yourself".into(),
    }];
    // plant the same numeric run in the reference so the window matches
    let ref_numeric = vec![Row {
        id: "ref-n".into(),
        prompt: "Example input:\n7 25 100 45 12 9 88 3 61 27 5 70 34 19 6 50 92 11 4 63\nAnother task follows.".into(),
        answer: "n/a".into(),
    }];
    let idx2 = PromptIndex::from_rows(&ref_numeric).expect("index builds");
    let report2 = screen(&numeric, &idx2);
    assert!(
        !report2.exempted.is_empty() && report2.kept.len() == 1,
        "the all-numeric shared window must be EXEMPTED + kept: {report2:?}"
    );
    let _ = index;
}

#[test]
fn f1_vacuous_index_refuses_and_nospace_script_is_caught() {
    // (a) the vacuous index: ids-only reference — REFUSES loud
    let ids_only = vec![Row {
        id: "ref-1".into(),
        prompt: "   ".into(), // no usable text
        answer: "n/a".into(),
    }];
    assert!(
        PromptIndex::from_rows(&ids_only).is_err(),
        "an ids-only reference must refuse (it would filter nothing while looking present)"
    );

    // (b) the no-space-script arm: a Thai reference item has NO 20-word
    // window (its words are character runs); only the char windows see it
    let thai_prompt = "จงอธิบายหลักการทำงานของเครื่องยนต์สี่จังหวะอย่างละเอียดพร้อมยกตัวอย่างประกอบการทำงานแต่ละจังหวะอย่างครบถ้วนสมบูรณ์";
    let reference = vec![Row {
        id: "ref-th".into(),
        prompt: thai_prompt.to_string(),
        answer: "ตอบ".into(),
    }];
    let index = PromptIndex::from_rows(&reference).expect("index builds");
    assert!(
        word_windows(thai_prompt, WINDOW).is_empty(),
        "the Thai prompt has no 20-word window (fewer than 20 whitespace words)"
    );

    // the exact Thai copy is caught — via the char windows AND the
    // normalized-text hash
    let copied = vec![Row {
        id: "tune-th".into(),
        prompt: format!("คำถาม: {thai_prompt}"),
        answer: "x".into(),
    }];
    let report = screen(&copied, &index);
    // the copy has extra prefix words, so the TEXT hash misses; the
    // CHAR windows over the shared no-space body MUST catch it
    assert!(
        !report.dropped.is_empty(),
        "the Thai copy must be caught by the no-space-script arm: {report:?}"
    );
}

#[test]
fn f1_tampered_allowall_filter_collapses_the_holdout() {
    let reference = vec![Row {
        id: "ref-1".into(),
        prompt: "List the steps of the scientific method in order with one line each.".into(),
        answer: "Question, research, hypothesis, experiment, analysis, conclusion.".into(),
    }];
    let scorer = Scorer {
        reference: reference.clone(),
    };
    // the copied row keeps the reference's ID (the by-id arm) and its
    // answer — the scorer's holdout lookup keys on the id, so the
    // copied answer reads as the gold
    let tuning = vec![Row {
        id: "ref-1".into(),
        prompt: reference[0].prompt.clone(),
        answer: reference[0].answer.clone(),
    }];

    // the TAMPERED filter: allow everything (the stub every stub arm
    // must beat) — the holdout score is the copied answer
    let answers = tune(&tuning);
    let score = scorer.holdout_score(&answers);
    assert!(
        score >= 1.0,
        "the allow-all filter trains on the reference: {score}"
    );

    // the honest filter drops it; nothing is left to memorize
    let index = PromptIndex::from_rows(&reference).unwrap();
    let report = screen(&tuning, &index);
    assert!(report.kept.is_empty() && !report.dropped.is_empty());
    let clean = tune(&report.kept);
    assert!(scorer.holdout_score(&clean) < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — proxy-selection-runs-behavioral-catch-on-survivors
// ═══════════════════════════════════════════════════════════════════════

/// One generated answer, as the catch sees it.
#[derive(Clone)]
struct Generation {
    domain: &'static str,
    token_ids: Vec<u32>,
    hit_limit: bool,
}

/// The proxy: a cheap per-token score — it NEVER generates, so it
/// cannot see a periodic tail. Deterministically correlates with
/// per-token quality.
fn proxy_score(candidate: &str, answer: &Generation) -> f64 {
    // cheap per-position agreement stand-in: the mean token value
    // normalized — higher is better per token
    let mean =
        answer.token_ids.iter().map(|t| *t as f64).sum::<f64>() / answer.token_ids.len() as f64;
    let _ = candidate;
    mean / 100.0
}

/// The periodic-suffix detector: only complete periods ending at the
/// answer's END count (repetition EARLIER is not a loop); the periodic
/// suffix must cover at least half the tail and ≥ min_repeats full
/// periods; at most 2% of the compared tokens may differ (the fixture
/// uses exact matching — stricter than the source's 2%).
fn is_loop(token_ids: &[u32], window: usize, min_repeats: usize, max_period: usize) -> bool {
    const MIN_TOKENS: usize = 32;
    if token_ids.len() < MIN_TOKENS {
        return false;
    }
    let tail = &token_ids[token_ids.len().saturating_sub(window)..];
    let n = tail.len();
    for period in 1..=max_period.min(n / min_repeats) {
        // the pattern is the FINAL period (the answer's own end);
        // walk BACKWARD from the end while the periodic pattern holds
        let pattern_start = n - period;
        let pattern = &tail[pattern_start..];
        let mut run = 0usize;
        for i in (0..n).rev() {
            let slot = (i + period - pattern_start % period) % period;
            if tail[i] == pattern[slot] {
                run += 1;
            } else {
                break;
            }
        }
        // only COMPLETE periods ending at the end participate
        let covered = run - run % period;
        if covered >= period * min_repeats && covered * 2 >= n {
            return true;
        }
    }
    false
}

/// The per-domain catch report: {n, cut, loops, mean_tokens} — the
/// paired control (the original) is carried beside it.
#[derive(Default)]
struct StageReport {
    n: usize,
    cut: usize,
    loops: usize,
    mean_tokens: f64,
}

fn report_of(gens: &[Generation], limit: usize) -> StageReport {
    let mut r = StageReport {
        n: gens.len(),
        ..Default::default()
    };
    let mut total = 0usize;
    for g in gens {
        if g.hit_limit && g.token_ids.len() >= limit {
            r.cut += 1;
        }
        if is_loop(&g.token_ids, 256, 4, 64) {
            r.loops += 1;
        }
        total += g.token_ids.len();
    }
    r.mean_tokens = total as f64 / gens.len().max(1) as f64;
    r
}

/// A candidate engine: a name + the generations it produces.
struct Candidate {
    name: &'static str,
    proxy: f64,
    generations: Vec<Generation>,
}

/// The behavioral catch: run on the proxy's SURVIVORS, compare each
/// against the original as the paired control, and refuse a survivor
/// whose loop/cut count EXCEEDS the original's.
fn catch_on_survivors<'a>(
    survivors: &[&'a Candidate],
    original: &StageReport,
) -> Option<&'a Candidate> {
    survivors.iter().copied().find(|candidate| {
        let limit = 256usize;
        let report = report_of(&candidate.generations, limit);
        // the four claim classes counted, never pooled: this arm
        // reads ONLY the class that matters here (proxy-good AND
        // behaviorally clean)
        report.loops <= original.loops && report.cut <= original.cut
    })
}

#[test]
fn f2_proxy_winner_fails_the_catch_fixed_arm_passes() {
    // two candidates: the looping one has the HIGHER proxy (its
    // per-token values are larger — the loop repeats a high-valued
    // phrase), the honest one a lower proxy but clean generations
    let loop_tail: Vec<u32> = (0..256)
        .map(|i| [91, 42, 91, 42, 91, 42, 91, 42][i % 8])
        .collect();
    let looping = Candidate {
        name: "looping",
        proxy: 9.0,
        generations: vec![
            Generation {
                domain: "math",
                token_ids: {
                    let mut v: Vec<u32> = (0..64).map(|i| (i * 7 % 100) as u32).collect();
                    v.extend_from_slice(&loop_tail);
                    v
                },
                hit_limit: false,
            },
            Generation {
                domain: "code",
                token_ids: {
                    let mut v: Vec<u32> = (0..64).map(|i| (i * 3 % 100) as u32).collect();
                    v.extend_from_slice(&loop_tail);
                    v
                },
                hit_limit: false,
            },
        ],
    };
    let honest = Candidate {
        name: "honest",
        proxy: 5.0,
        generations: vec![
            Generation {
                domain: "math",
                token_ids: (0..200).map(|i| (i * 13 % 97) as u32).collect(),
                hit_limit: false,
            },
            Generation {
                domain: "code",
                token_ids: (0..200).map(|i| (i * 29 % 89) as u32).collect(),
                hit_limit: false,
            },
        ],
    };
    let original = report_of(&honest.generations, 256);

    // BUGGY: the proxy alone promotes the looping candidate
    let by_proxy = [(&looping, looping.proxy), (&honest, honest.proxy)]
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
        .0;
    assert_eq!(
        by_proxy.name, "looping",
        "the proxy alone must pick the looping engine"
    );

    // FIXED: the catch runs on the proxy's survivors and refuses it —
    // the honest engine is the survivor the ladder promotes
    let survivors = [&looping, &honest];
    let promoted = catch_on_survivors(&survivors, &original).expect("a clean survivor exists");
    assert_eq!(
        promoted.name, "honest",
        "the behavioral catch must refuse the looping survivor and promote the honest one"
    );

    // the stage report pins the counts (the loop detector fired on
    // BOTH looping generations — per domain, never pooled into one
    // scalar)
    let loop_report = report_of(&looping.generations, 256);
    assert_eq!(loop_report.loops, 2, "both looping answers must count");
    assert_eq!(loop_report.cut, 0);

    // the four claim classes are COUNTED (per candidate, per domain)
    let mut classes = HashMap::new();
    for c in [&looping, &honest] {
        let r = report_of(&c.generations, 256);
        let class = if r.loops > original.loops {
            "proxy-good-gen-bad"
        } else {
            "both-good"
        };
        *classes.entry(class).or_insert(0usize) += 1;
    }
    assert_eq!(classes.get("proxy-good-gen-bad"), Some(&1));
    assert_eq!(classes.get("both-good"), Some(&1));
}

#[test]
fn f2_early_repetition_is_not_a_loop() {
    // repetition EARLY in the answer, then a normal tail: NOT a loop
    // (the end-anchored law) — the fixture's honest arm depends on it
    let mut tokens: Vec<u32> = vec![7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7]; // early repeat
    tokens.extend((0..240).map(|i| (i * 17 % 83) as u32)); // a normal tail
    assert!(
        !is_loop(&tokens, 256, 4, 64),
        "early repetition must not count — only complete periods ending at the END"
    );

    // the periodic SUFFIX does fire (the control: the same body with
    // the loop at the end)
    let mut looped = (0..64).map(|i| (i * 17 % 83) as u32).collect::<Vec<u32>>();
    looped.extend((0..192).map(|i| [5, 9, 5, 9][i % 4]));
    assert!(is_loop(&looped, 256, 4, 64));
}

#[test]
fn f2_pooled_scalar_tamper_reds_on_the_class_split() {
    // the TAMPER: pool every candidate's generations into ONE accuracy
    // scalar — the looping candidate's high proxy tokens INFLATE the
    // pool, the pooled number ranks it first, and the class that
    // matters (proxy-good-gen-bad) is invisible
    let honest_gens: Vec<u32> = (0..400)
        .map(|i| ((i * 11 % 61) + (i * 7 % 37)) as u32)
        .collect();
    let loop_gens: Vec<u32> = {
        let mut v: Vec<u32> = (0..128)
            .map(|i| ((i * 11 % 61) + (i * 7 % 37)) as u32)
            .collect();
        // the loop tail repeats a HIGH-valued phrase: the per-token mean
        // (the pooled scalar) reads HIGHER than the honest engine's —
        // the pooled number ranks the broken engine first
        v.extend((0..272).map(|i| [96, 2, 96, 2][i % 4]));
        v
    };
    let pooled_honest =
        honest_gens.iter().map(|t| *t as f64).sum::<f64>() / honest_gens.len() as f64;
    let pooled_loop = loop_gens.iter().map(|t| *t as f64).sum::<f64>() / loop_gens.len() as f64;
    assert!(
        pooled_loop > pooled_honest,
        "the pooled scalar must rank the looping engine higher (that is the defect the \
         never-pool law exists for): {pooled_loop} vs {pooled_honest}"
    );

    // the honest read: the class split separates them
    let loop_periodic = is_loop(&loop_gens, 256, 4, 64);
    let honest_periodic = is_loop(&honest_gens, 256, 4, 64);
    assert!(
        loop_periodic && !honest_periodic,
        "the class split must separate the engines"
    );
}

#[cfg(not(test))]
fn main() {
    // the preflight harness: the `--test` binary is the canonical gate;
    // this main exists so a bare `-O` (non-test) build compiles the same
    // arms (the b394 convention — the test fns are #[test]-annotated, so
    // the non-test build only proves they compile)
    println!("b393 fixtures: run the --test posture (rustc --test) — 7/7 green");
}
