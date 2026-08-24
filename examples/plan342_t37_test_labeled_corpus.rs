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
use riir_clippy::edit_emitter::{SourceEdit, apply_edits};
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
const RUN_TIMEOUT: Duration = Duration::from_secs(30);
const COMPILE_TIMEOUT: Duration = Duration::from_secs(120);

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

fn compile(src: &Path, exe: &Path) -> Outcome {
    let child = Command::new("rustc")
        .args(["--test", "--edition", "2024", "-A", "warnings"])
        .arg(src)
        .arg("-o")
        .arg(exe)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    match child {
        Ok(c) => wait_bounded(c, COMPILE_TIMEOUT),
        Err(_) => Outcome::Fail,
    }
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

fn main() {
    let mut targets_file = String::new();
    let mut out_path = String::from("t37_rows.jsonl");
    let mut workdir: Option<String> = None;
    let mut only_lints: Option<Vec<String>> = None;
    let mut mode = Mode::PerLint;

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

    let (mut n_rows, mut n_skipped, mut n_edits) = (0usize, 0usize, 0usize);
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
                let edited = apply_edits(&source, group.iter().map(dup).collect());
                if edited == source {
                    continue; // the emitter dropped them (overlap / bad range)
                }
                let cand_src = wd.join(format!("t{ti}_{lint}_{ei}.rs"));
                let cand_exe = wd.join(format!("t{ti}_{lint}_{ei}.exe"));
                if std::fs::write(&cand_src, &edited).is_err() {
                    continue;
                }
                let compile_out = compile(&cand_src, &cand_exe);
                let test_out = match compile_out {
                    Outcome::Pass => run_tests(&cand_exe),
                    _ => Outcome::Fail,
                };

                match (compile_out, test_out) {
                    (Outcome::Pass, Outcome::Pass) => cc_pass_t_pass += 1,
                    (Outcome::Pass, Outcome::Fail) => cc_pass_t_fail += 1,
                    (Outcome::Pass, Outcome::Timeout) => cc_timeout += 1,
                    _ => cc_fail += 1,
                }

                // Span, line-expanded on BOTH sides so the pair is comparable.
                let (lo, hi) = line_span(&source, e.start, e.end);
                let buggy = &source[lo..hi];
                let new_end = e.start + e.replacement.len();
                let (nlo, nhi) = line_span(&edited, e.start, new_end);
                let fixed = &edited[nlo.min(edited.len())..nhi.min(edited.len())];

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
