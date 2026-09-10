//! Plan 342 T3.7 — emit a fix corpus whose label is decided by **running the
//! tests**, not by the compile gate and not by provenance.
//!
//! ## Why this exists
//!
//! riir-train's Plan 342 needs rows on which a learned verifier could possibly
//! earn its cost, and Bench 511 showed its corpus has none: every cut produced
//! a label that was already derivable for free. Two confounds, both fatal:
//!
//! - **`label == clippy_pass`** (T3.3d) — the label *was* the compile gate's
//!   output, so the gate scores AUC 1.0 by construction.
//! - **`label == provenance`** (Bench 511 gate A5) — every `Yes` was a real
//!   corpus row and every `No` a synthesised corruption, 51/51, so "looks
//!   corrupted" reads the label for free.
//!
//! The requirement that follows is a conjunction:
//!
//! > A row can support a verifier evaluation only if its label is derivable
//! > **neither from the compile gate nor from its provenance.**
//!
//! This harness satisfies both clauses **by construction**, which is the whole
//! point of its design:
//!
//! 1. **Not the compile gate** — every emitted row records `compile_ok` and
//!    `tests_ok` separately, and the label is `tests_ok`. The rows that matter
//!    are `compile_ok = true, tests_ok = false`: the edit builds, so the gate
//!    says Yes, and the tests say No. The gate is *wrong* on those rows, which
//!    is exactly the signal Plan 342 has never had a single instance of.
//! 2. **Not provenance** — both classes come from **one** generator: the real
//!    riir-clippy proposers, applied to real workspace source. There is no
//!    corruption generator, so nothing distinguishes a `Yes` row from a `No`
//!    row except what the edit does to the tests.
//!
//! ## Method
//!
//! Targets are Rust files that compile and pass their own tests **standalone**
//! under `rustc --test` — real production modules whose only dependency is
//! `std`. That keeps the label loop sub-second per candidate, so isolating one
//! edit at a time is affordable.
//!
//! Per target: establish a green baseline (a target whose tests are not
//! already green cannot attribute a later failure to an edit, so it is
//! excluded), then for each lint collect the proposer's edits, apply them,
//! compile, run, record.
//!
//! **`--mode` decides what a compile failure means** and is not a tuning knob
//! — see [`Mode`]. `per-lint` (default) applies all of one lint's edits for a
//! file together, which is the unit `--fix` actually uses; `single` isolates
//! one edit for per-site diagnosis but *fabricates* failures for the
//! multi-edit lints.
//!
//! ```text
//! cargo run --release --example plan342_t37_test_labeled_corpus -- \
//!     --targets targets.txt --out t37_rows.jsonl --mode per-lint
//! ```
//!
//! ⚠ Targets must be **Windows** paths. Git-Bash `/e/git/...` forms are not
//! translated when read from a file (only in argv), so every target silently
//! skips with "cannot find the path specified".
//!
//! Output is deliberately *neutral* JSONL — `(lint, span, fix, compile_ok,
//! tests_ok)` — not GenRM prompt text. Rendering rows into a training corpus is
//! riir-train's concern; producing verified edit outcomes is this repo's.

use riir_clippy::domains::clippy_lints::proposers;
use riir_clippy::edit_emitter::{apply_edits, SourceEdit};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Every lint with a proposer. Not just `FIXABLE_LINTS`: the type-blind ones
/// (`needless_borrow`, `useless_conversion`, `unnecessary_mut_passed`,
/// `redundant_closure`, …) are the *most likely* to compile while changing
/// behaviour, which is the cell this harness exists to populate.
const ALL_LINTS: &[&str] = &[
    "needless_return",
    "redundant_closure",
    "needless_lifetimes",
    "redundant_field_names",
    "useless_conversion",
    "needless_borrow",
    "unnecessary_mut_passed",
    "manual_map",
    "unnecessary_unwrap",
    "needless_range_loop",
    "semicolon_if_nothing_returned",
    "collapsible_if",
    "collapsible_else_if",
    "single_match",
    "single_match_else",
    "redundant_pattern_matching",
    "question_mark",
    "range_plus_one",
    "uninlined_format_args",
    "identity_op",
    "needless_bool",
    "needless_bool_assign",
    "match_bool",
    "needless_match",
    "redundant_static_lifetimes",
    "double_parens",
    "unnecessary_lazy_evaluations",
    "map_unwrap_or",
    "filter_next",
    "iter_nth_zero",
    "iter_next_loop",
    "comparison_to_empty",
    "while_let_on_iterator",
    "single_char_pattern",
    "deref_addrof",
    "identity_conversion",
    "manual_filter_map",
    "option_as_ref_deref",
    "unnecessary_cast",
    "useless_format",
];

/// How a lint's edits are applied.
///
/// **This is not a tuning knob — it changes what a compile failure means.**
/// Some proposers emit a *multi-edit atomic group* per site:
/// `uninlined_format_args` rewrites `"{:?}"` → `"{x:?}"` **and** deletes the
/// now-unused trailing `, x` as two separate `SourceEdit`s. Either one alone
/// cannot compile ("argument never used" / "no arguments were given"), so
/// isolating single edits reports 0/48 compile-pass for a lint that is
/// perfectly correct when applied as riir-clippy actually applies it.
/// Measured: that is exactly what happened on the first sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// One `SourceEdit` at a time. Maximum attribution, but it FABRICATES
    /// compile failures for multi-edit lints. Kept for per-site diagnosis.
    Single,
    /// All of one lint's edits for a file, applied together — the unit
    /// `propose_fix` / `--fix` actually use. The correct mode for corpus
    /// generation, because it is the edit a real consumer would see.
    PerLint,
}

/// A test binary that hangs would stall the sweep; a behaviour-changing edit
/// can genuinely produce an infinite loop (a mutated loop bound, a `while let`
/// whose receiver no longer advances). Treated as a distinct outcome, not as a
/// test failure — conflating them would credit the harness with a "No" it did
/// not actually observe.
/// Only the RUN needs a deadline. `compile_traced` uses `Command::output()`,
/// which waits for rustc to finish — correct here because a compile terminates
/// on its own where a mutated test binary may not.
const RUN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Pass,
    Fail,
    Timeout,
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(2);
}

/// Wait with a deadline, killing the child on expiry.
fn wait_bounded(mut child: std::process::Child, limit: Duration) -> Outcome {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                return match st.success() {
                    true => Outcome::Pass,
                    false => Outcome::Fail,
                };
            }
            Ok(None) => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Outcome::Timeout;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return Outcome::Fail,
        }
    }
}

/// Compile, returning the outcome **and rustc's diagnostics**.
///
/// The trace is the whole point of Plan 342 T4.1: it is the *rationale* a
/// GenRM-CoT verifier is graded against ("compiler-guided grading", the
/// stronger analogue of the paper's reference-guided grading). Discarding it —
/// which this harness did until now — makes a CoT corpus unbuildable, because
/// the one thing that actually explains WHY an edit is bad is gone.
///
/// Uses `output()` rather than the deadline-bounded spawn: rustc terminates on
/// its own (a *test binary* can loop forever, a compile cannot), and capturing
/// stderr through a pipe needs the reader to drain it anyway.
fn compile_traced(src: &Path, exe: &Path) -> (Outcome, String) {
    let out = Command::new("rustc")
        .args(["--test", "--edition", "2024", "-A", "warnings"])
        .arg(src)
        .arg("-o")
        .arg(exe)
        .stdout(Stdio::null())
        .output();
    match out {
        Ok(o) => {
            let trace = String::from_utf8_lossy(&o.stderr).trim().to_string();
            let outcome = match o.status.success() {
                true => Outcome::Pass,
                false => Outcome::Fail,
            };
            (outcome, trace)
        }
        Err(e) => (Outcome::Fail, format!("rustc spawn failed: {e}")),
    }
}

fn compile(src: &Path, exe: &Path) -> Outcome {
    compile_traced(src, exe).0
}

fn run_tests(exe: &Path) -> Outcome {
    let child = Command::new(exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(c) => wait_bounded(c, RUN_TIMEOUT),
        Err(_) => Outcome::Fail,
    }
}

/// Expand a byte range to whole lines so the emitted span is a readable unit
/// of code rather than a bare token. A single-token span (`1` from `x * 1`) is
/// useless as a verifier prompt.
fn line_span(source: &str, start: usize, end: usize) -> (usize, usize) {
    let lo = source[..start.min(source.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let hi = source[end.min(source.len())..]
        .find('\n')
        .map_or(source.len(), |i| end + i);
    (lo, hi)
}

/// The L4 drafter's prompt, byte-identical to `data/clippy_l4_sft_train.jsonl`.
///
/// **Load-bearing that this is one function.** T3.8 emits a corpus with it and
/// later rejoins model drafts by prompt text, because `--dump-drafts` does not
/// carry through extra columns — the prompt IS the join key. Two
/// near-identical renderings would silently drop every row.
fn l4_prompt(lint: &str, buggy: &str) -> String {
    format!(
        "Fix the clippy lint `{lint}` in this Rust code. Output ONLY the fixed \
         code, no explanation:\n\n{buggy}"
    )
}

/// What the sweep does with each site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Task {
    /// T3.7: apply the PROPOSER's edit, compile, test.
    MeasureProposer,
    /// T3.8 step 1 (CPU): emit an L4 eval corpus over these sites, so the
    /// model is asked to fix *buildable, testable* code rather than the
    /// existing corpus's in-crate excerpts.
    EmitL4Corpus,
    /// T3.8 step 3 (CPU): apply the MODEL's draft at the same sites and run
    /// the same compile+test loop. Only the edit's author changes, so the
    /// proposer cross-tab is a valid control.
    MeasureDrafts,
}

fn main() {
    let mut targets_file = String::new();
    let mut out_path = String::from("t37_rows.jsonl");
    let mut workdir: Option<String> = None;
    let mut only_lints: Option<Vec<String>> = None;
    let mut mode = Mode::PerLint;
    let mut task = Task::MeasureProposer;
    let mut emit_corpus: Option<String> = None;
    let mut drafts_path: Option<String> = None;

    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        let need = |i: usize| -> String {
            args.get(i)
                .cloned()
                .unwrap_or_else(|| die(&format!("{} needs a value", args[i - 1])))
        };
        match args[i].as_str() {
            "--targets" => {
                i += 1;
                targets_file = need(i);
            }
            "--out" => {
                i += 1;
                out_path = need(i);
            }
            "--workdir" => {
                i += 1;
                workdir = Some(need(i));
            }
            "--lints" => {
                i += 1;
                let v = need(i);
                if v != "all" {
                    only_lints = Some(v.split(',').map(|s| s.trim().to_string()).collect());
                }
            }
            "--mode" => {
                i += 1;
                mode = match need(i).as_str() {
                    "single" => Mode::Single,
                    "per-lint" => Mode::PerLint,
                    other => die(&format!("--mode must be single|per-lint, got {other}")),
                };
            }
            "--emit-l4-corpus" => {
                i += 1;
                emit_corpus = Some(need(i));
                task = Task::EmitL4Corpus;
            }
            "--drafts" => {
                i += 1;
                drafts_path = Some(need(i));
                task = Task::MeasureDrafts;
            }
            other => die(&format!("unknown arg: {other}")),
        }
        i += 1;
    }
    if targets_file.is_empty() {
        die("--targets <file> is required (one .rs path per line)");
    }

    let wd = PathBuf::from(workdir.unwrap_or_else(|| {
        std::env::temp_dir()
            .join("p342_t37")
            .to_string_lossy()
            .to_string()
    }));
    std::fs::create_dir_all(&wd).unwrap_or_else(|e| die(&format!("mkdir {}: {e}", wd.display())));

    let list = std::fs::read_to_string(&targets_file)
        .unwrap_or_else(|e| die(&format!("read {targets_file}: {e}")));
    let targets: Vec<&str> = list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        // tolerate the "path|N passed" form the discovery step emits
        .map(|l| l.split('|').next().unwrap_or(l).trim())
        .collect();

    let lints: Vec<&str> = match &only_lints {
        None => ALL_LINTS.to_vec(),
        Some(v) => ALL_LINTS
            .iter()
            .copied()
            .filter(|l| v.iter().any(|w| w == l))
            .collect(),
    };

    println!("Plan 342 T3.7 — test-labeled fix corpus");
    println!("  targets : {} from {targets_file}", targets.len());
    println!("  lints   : {}", lints.len());
    println!("  workdir : {}", wd.display());
    println!();

    let mut out = std::fs::File::create(&out_path)
        .map(std::io::BufWriter::new)
        .unwrap_or_else(|e| die(&format!("create {out_path}: {e}")));

    let mut corpus_out = emit_corpus.as_ref().map(|p| {
        std::fs::File::create(p)
            .map(std::io::BufWriter::new)
            .unwrap_or_else(|e| die(&format!("create {p}: {e}")))
    });

    // T3.8 step 3: prompt -> model draft. The prompt is the join key because
    // `--dump-drafts` emits only (lint, prompt, reference, draft, …) and drops
    // any provenance columns the corpus carried.
    let mut drafts: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(dp) = &drafts_path {
        let text = std::fs::read_to_string(dp).unwrap_or_else(|e| die(&format!("read {dp}: {e}")));
        let mut dup_keys = 0usize;
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let (Some(p), Some(d)) = (v["prompt"].as_str(), v["draft"].as_str()) else {
                continue;
            };
            if drafts.insert(p.to_string(), d.to_string()).is_some() {
                dup_keys += 1;
            }
        }
        // A duplicated prompt means the join is ambiguous and some site would
        // silently get another site's draft. Refuse rather than mis-attribute.
        if dup_keys > 0 {
            die(&format!(
                "{dup_keys} duplicate prompts in {dp}: the prompt join is ambiguous, so \
                 drafts would be attributed to the wrong site. Aborting."
            ));
        }
        println!("  drafts loaded: {} unique prompts\n", drafts.len());
    }

    let (mut n_rows, mut n_skipped, mut n_edits) = (0usize, 0usize, 0usize);
    let (mut n_no_draft, mut n_noop_draft) = (0usize, 0usize);
    let mut n_empty_draft = 0usize;
    // The cross-tab this harness exists to fill in.
    let (mut cc_pass_t_pass, mut cc_pass_t_fail, mut cc_fail, mut cc_timeout) =
        (0usize, 0usize, 0usize, 0usize);

    for (ti, tgt) in targets.iter().enumerate() {
        let source = match std::fs::read_to_string(tgt) {
            Ok(s) => s,
            Err(e) => {
                println!("  SKIP {tgt}: read: {e}");
                n_skipped += 1;
                continue;
            }
        };
        let base_src = wd.join(format!("t{ti}_base.rs"));
        let base_exe = wd.join(format!("t{ti}_base.exe"));
        if std::fs::write(&base_src, &source).is_err() {
            n_skipped += 1;
            continue;
        }
        // A target whose baseline is not green cannot attribute a later
        // failure to an edit, so it is excluded rather than measured.
        if compile(&base_src, &base_exe) != Outcome::Pass || run_tests(&base_exe) != Outcome::Pass {
            println!("  SKIP {tgt}: baseline not green");
            n_skipped += 1;
            continue;
        }
        let Ok(parsed) = syn::parse_file(&source) else {
            println!("  SKIP {tgt}: syn parse failed");
            n_skipped += 1;
            continue;
        };

        let mut per_target = 0usize;
        for lint in &lints {
            let edits = proposers::collect_edits(lint, &source, &parsed);
            if edits.is_empty() {
                continue;
            }
            let dup = |e: &SourceEdit| SourceEdit {
                start: e.start,
                end: e.end,
                replacement: e.replacement.clone(),
            };
            // A candidate is the ATOMIC unit being judged. In `PerLint` that is
            // all of this lint's edits together (what `--fix` applies); in
            // `Single` it is one edit, which fabricates failures for the
            // multi-edit lints and is therefore diagnosis-only.
            let candidates: Vec<Vec<SourceEdit>> = match mode {
                Mode::PerLint => vec![edits.iter().map(|re| dup(&re.edit)).collect()],
                Mode::Single => edits.iter().map(|re| vec![dup(&re.edit)]).collect(),
            };
            for (ei, group) in candidates.iter().enumerate() {
                n_edits += 1;
                let e = dup(&group[0]);
                let group_len = group.len();
                let proposed = apply_edits(&source, group.iter().map(dup).collect());
                if proposed == source {
                    continue; // the emitter dropped them (overlap / bad range)
                }
                // The prompt/apply window, shared by all three tasks so the
                // proposer arm is a valid control for the model arm.
                let (w_lo, w_hi) = group.iter().fold((usize::MAX, 0usize), |(a, b), g| {
                    let (x, y) = line_span(&source, g.start, g.end);
                    (a.min(x), b.max(y))
                });
                let prompt = l4_prompt(lint, &source[w_lo..w_hi]);

                // ---- T3.8 step 1: emit the L4 eval corpus and move on ----
                if task == Task::EmitL4Corpus {
                    let p_delta: isize = group
                        .iter()
                        .map(|g| g.replacement.len() as isize - (g.end as isize - g.start as isize))
                        .sum();
                    let p_hi = (w_hi as isize + p_delta).clamp(0, proposed.len() as isize) as usize;
                    let reference = &proposed[w_lo.min(proposed.len())..p_hi.max(w_lo)];
                    let row = serde_json::json!({
                        "prompt": prompt,
                        // The proposer's own output is the reference, so
                        // `exact_match` stays meaningful and the model is
                        // scored against a fix that is known to compile here.
                        "response": reference,
                        "lint": lint,
                        // Provenance for the rejoin. `--dump-drafts` drops
                        // unknown columns, so these are for OUR index only and
                        // the prompt remains the actual join key.
                        "file": tgt,
                        "start": w_lo,
                        "end": w_hi,
                        "n_edits": group_len,
                    });
                    if let Some(w) = corpus_out.as_mut() {
                        let _ = writeln!(w, "{row}");
                    }
                    n_rows += 1;
                    per_target += 1;
                    continue;
                }

                // ---- choose whose edit is under test ----
                let edited = match task {
                    Task::MeasureDrafts => {
                        let Some(draft) = drafts.get(&prompt) else {
                            n_no_draft += 1;
                            continue;
                        };
                        // An EMPTY draft is a non-attempt, not a failed fix.
                        // `--dump-drafts` writes `draft` as
                        // `extract_code_block(generated)`, which returns "" when
                        // the model emits no usable fenced block (verified: the
                        // extractor has an explicit `return ""` for a fence with
                        // no newline). Splicing "" over the buggy span DELETES
                        // that span, which then fails to compile and would be
                        // scored as "the model produced a fix that does not
                        // build" — inflating the compile-FAIL cell with rows
                        // where nothing was proposed. That matters here: an
                        // undertrained drafter (Plan 336 measured 0/116
                        // exact-match) is exactly the regime that emits
                        // malformed output, so this could be a large fraction
                        // and would shrink the effective n behind the gold cell
                        // while the report still claimed the full count.
                        if draft.trim().is_empty() {
                            n_empty_draft += 1;
                            continue;
                        }
                        // Splice the model's span back where the buggy span was.
                        let mut s = String::with_capacity(source.len() + draft.len());
                        s.push_str(&source[..w_lo]);
                        s.push_str(draft);
                        s.push_str(&source[w_hi..]);
                        s
                    }
                    _ => proposed.clone(),
                };
                if edited == source {
                    n_noop_draft += 1;
                    continue;
                }
                let cand_src = wd.join(format!("t{ti}_{lint}_{ei}.rs"));
                let cand_exe = wd.join(format!("t{ti}_{lint}_{ei}.exe"));
                if std::fs::write(&cand_src, &edited).is_err() {
                    continue;
                }
                let (compile_out, mut trace) = compile_traced(&cand_src, &cand_exe);
                let test_out = match compile_out {
                    Outcome::Pass => run_tests(&cand_exe),
                    _ => Outcome::Fail,
                };
                // Cap the rationale: rustc can emit hundreds of lines and a
                // GenRM-CoT target is a short chain, not a transcript. Keep the
                // head, which carries the error code and primary span.
                if trace.len() > 1200 {
                    trace.truncate(1200);
                    trace.push_str("\n… [truncated]");
                }

                match (compile_out, test_out) {
                    (Outcome::Pass, Outcome::Pass) => cc_pass_t_pass += 1,
                    (Outcome::Pass, Outcome::Fail) => cc_pass_t_fail += 1,
                    (Outcome::Pass, Outcome::Timeout) => cc_timeout += 1,
                    _ => cc_fail += 1,
                }

                // Reuse the same window the prompt was cut from, so `buggy` and
                // `candidate_fix` describe the identical region in both arms.
                let (lo, hi) = (w_lo, w_hi);
                let buggy = &source[lo..hi];
                // Every edit lies inside [lo, hi), so the edited region is that
                // window shifted by the net length delta — exact, no re-scan.
                let delta: isize = match task {
                    Task::MeasureDrafts => edited.len() as isize - source.len() as isize,
                    _ => group
                        .iter()
                        .map(|g| g.replacement.len() as isize - (g.end as isize - g.start as isize))
                        .sum(),
                };
                let nhi = (hi as isize + delta).clamp(0, edited.len() as isize) as usize;
                let fixed = &edited[lo.min(edited.len())..nhi.max(lo.min(edited.len()))];

                let compile_ok = compile_out == Outcome::Pass;
                let tests_ok = test_out == Outcome::Pass;
                let row = serde_json::json!({
                    "lint": lint,
                    "file": tgt,
                    "start": e.start,
                    "end": e.end,
                    "buggy_span": buggy,
                    "candidate_fix": fixed,
                    // Load-bearing for interpretation: a multi-site group's
                    // span covers only the FIRST site, so a consumer wanting
                    // clean single-site prompts must filter n_edits == 1.
                    "n_edits": group_len,
                    "apply_mode": match mode {
                        Mode::PerLint => "per-lint",
                        Mode::Single => "single",
                    },
                    // Which author's edit this row measures. The whole point of
                    // T3.8 is that these two arms share target, window, and
                    // label mechanism, so `arm` is the ONLY difference.
                    "arm": match task {
                        Task::MeasureDrafts => "model_draft",
                        _ => "proposer",
                    },
                    "compile_ok": compile_ok,
                    "tests_ok": tests_ok,
                    "run_outcome": match test_out {
                        Outcome::Pass => "pass",
                        Outcome::Fail => "fail",
                        Outcome::Timeout => "timeout",
                    },
                    // The label is EARNED by the test run. This is the field
                    // Plan 342 has never had a gate-independent instance of.
                    "label": if tests_ok { "Yes" } else { "No" },
                    // Recorded so a consumer can prove the label is not the
                    // gate's output rather than taking it on trust.
                    "gate_would_say": if compile_ok { "Yes" } else { "No" },
                    "gate_is_wrong": compile_ok != tests_ok,
                    // T4.1: the RATIONALE for compiler-guided grading. Empty on
                    // a clean compile, which is itself the signal.
                    "compiler_trace": trace,
                });
                let _ = writeln!(out, "{row}");
                n_rows += 1;
                per_target += 1;
                let _ = std::fs::remove_file(&cand_src);
                let _ = std::fs::remove_file(&cand_exe);
            }
        }
        println!(
            "  {:<62} {:>3} rows",
            tgt.rsplit(['/', '\\']).next().unwrap_or(tgt),
            per_target
        );
        let _ = std::fs::remove_file(&base_src);
        let _ = std::fs::remove_file(&base_exe);
    }
    let _ = out.flush();

    if let Some(w) = corpus_out.as_mut() {
        let _ = w.flush();
    }
    if task == Task::EmitL4Corpus {
        println!(
            "\n  emitted {n_rows} L4 prompts -> {}",
            emit_corpus.as_deref().unwrap_or("?")
        );
        println!("  targets skipped : {n_skipped}");
        println!(
            "\n  Next: run bonsai_clippy_l4_eval --test <that file> --dump-drafts <drafts>,\n\
             \x20 then re-run this harness with --drafts <drafts> to get the model arm."
        );
        return;
    }

    println!("\n  edits attempted : {n_edits}");
    println!("  rows emitted    : {n_rows}");
    println!("  targets skipped : {n_skipped}");
    println!("\n  outcome cross-tab");
    println!("    compile PASS / tests PASS : {cc_pass_t_pass:>4}   (label Yes; gate agrees)");
    println!(
        "    compile PASS / tests FAIL : {cc_pass_t_fail:>4}   <- THE GOLD CELL (gate says Yes, truth is No)"
    );
    println!(
        "    compile PASS / tests HANG : {cc_timeout:>4}   (recorded distinctly, not as a fail)"
    );
    println!("    compile FAIL              : {cc_fail:>4}   (the gate already handles these)");
    if task == Task::MeasureDrafts {
        println!(
            "\n  model-arm bookkeeping (excluded, never counted as an outcome)\n\
             \x20   sites with no draft       : {n_no_draft}\n\
             \x20   drafts identical to input : {n_noop_draft}   (a no-op draft is not a fix \
             attempt, so scoring it would flatter the model)\n\
             \x20   drafts EMPTY after extract: {n_empty_draft}   (no usable fenced block — a \
             non-attempt, NOT a fix that fails to build)"
        );
    }

    println!("\n  VERDICT");
    let gate_independent = cc_pass_t_fail;
    println!(
        "    gate-independent rows : {gate_independent}  {}",
        match gate_independent {
            0 =>
                "FAIL — no row where the compile gate is wrong, so this generator \
                  cannot break the label==gate confound",
            _ => "PASS — rows exist where the gate says Yes and the tests say No",
        }
    );
    let two_class = cc_pass_t_pass > 0 && cc_pass_t_fail > 0;
    println!(
        "    both classes, one generator : {}  {}",
        two_class,
        match two_class {
            true => "PASS — provenance cannot predict the label (A5 broken by construction)",
            false => "FAIL — single-class output; provenance would still confound",
        }
    );
    println!("\n  wrote {n_rows} rows -> {out_path}");
}
