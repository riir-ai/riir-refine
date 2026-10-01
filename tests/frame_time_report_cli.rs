//! Plan 134 T2.1 — `--frame-time-report`'s exit contracts, pinned AT THE
//! PROCESS BOUNDARY (the `coverage_report_cli.rs` pattern): the lib fixtures
//! in `src/frame_time_report.rs` cover the parse/verdict arithmetic; these
//! arms cover what only the real binary can — the exit codes on a blind
//! scan, an unparsable sidecar, and the A/B evidence guards.
//!
//! Contract table:
//!
//! | invocation | exit | because |
//! |---|---|---|
//! | report over sidecars with budgets | 0 | a real scan with verdicts |
//! | report over an empty dir | 1 | the frontier-report law — never a green zero |
//! | report over a malformed sidecar | 1 | named UNPARSABLE row; partial ≠ pass |
//! | report with no budget anywhere | 0 | measurement without a verdict is still a measurement (no budget column lie) |
//! | A/B improved | 0 | paired, warmup-equal, delta beyond threshold |
//! | A/B warmup mismatch | 1 | a pair captured under different warmups is not evidence |
//! | A/B ambiguous pairing | 1 | the tool refuses to invent pairings |
#![cfg(all(
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints",
    feature = "kat_service"
))] // the bin's required-features — CARGO_BIN_EXE_cargo-refine exists only under them

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static TMP_COUNTER: AtomicU32 = AtomicU32::new(0);

fn heal(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .args(args)
        .env("RIIR_HEAL_KAT_METER", "0")
        .env("RIIR_HEAL_KAT_GATE", "0")
        .output()
        .expect("spawn cargo-refine");
    (
        out.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fft_cli_{}_{}_{}",
        tag,
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).expect("mkdir");
    dir
}

fn sidecar_text(effect: &str, p50: f64, warmup: (u32, u64)) -> String {
    format!(
        r#"{{
  "schema": "riir_shader_frame_time_v1",
  "effect": "{effect}",
  "capture": "/tmp/x/{effect}.png",
  "warmup_frames": {},
  "warmup_min_ms": {},
  "frame_budget_ms": 16.7,
  "n_samples": 100,
  "mean_ms": {p50:.3},
  "p50_ms": {p50:.3},
  "p90_ms": 60.0,
  "min_ms": 1.0,
  "max_ms": 60.0,
  "mean_fps": 60.0,
  "samples_ms": [1.0, 2.0]
}}
"#,
        warmup.0, warmup.1
    )
}

fn write_sidecar(dir: &std::path::Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).expect("write sidecar");
    path
}

#[test]
fn budget_report_happy_path_exits_zero_with_verdicts() {
    let dir = scratch("happy");
    write_sidecar(&dir, "gradient.frames.json", &sidecar_text("gradient", 10.0, (45, 750)));
    write_sidecar(&dir, "fft-ocean.frames.json", &sidecar_text("fft-ocean", 20.0, (45, 750)));
    let (code, text) = heal(&["--frame-time-report", dir.to_str().expect("utf8")]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("frame-time budget report"), "{text}");
    assert!(text.contains("OK"), "{text}");
    assert!(text.contains("OVER"), "{text}");
    assert!(text.contains("--domain shader_perf"), "heal hint missing: {text}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn empty_scan_refuses_exit_one() {
    let dir = scratch("empty");
    let (code, text) = heal(&["--frame-time-report", dir.to_str().expect("utf8")]);
    assert_eq!(code, Some(1), "output:\n{text}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn unparsable_sidecar_refuses_exit_one_and_names_the_file() {
    let dir = scratch("bad");
    write_sidecar(&dir, "gradient.frames.json", &sidecar_text("gradient", 10.0, (45, 750)));
    write_sidecar(&dir, "broken.frames.json", "{ not json at all");
    let (code, text) = heal(&["--frame-time-report", dir.to_str().expect("utf8")]);
    assert_eq!(code, Some(1), "output:\n{text}");
    assert!(text.contains("UNPARSABLE"), "{text}");
    assert!(text.contains("broken.frames.json"), "{text}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn no_budget_anywhere_still_reports_exit_zero_without_verdicts() {
    let dir = scratch("nobudget");
    let text = sidecar_text("gradient", 10.0, (45, 750)).replace("16.7", "null");
    write_sidecar(&dir, "gradient.frames.json", &text);
    let (code, out) = heal(&["--frame-time-report", dir.to_str().expect("utf8")]);
    assert_eq!(code, Some(0), "output:\n{out}");
    assert!(out.contains("n/a"), "{out}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn frame_budget_flag_overrides_the_sidecar_echo() {
    let dir = scratch("override");
    write_sidecar(&dir, "gradient.frames.json", &sidecar_text("gradient", 10.0, (45, 750)));
    // 10.0 ms p50 against a 5 ms CLI budget → OVER.
    let (code, text) = heal(&[
        "--frame-time-report",
        "--frame-budget",
        "5",
        dir.to_str().expect("utf8"),
    ]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("OVER"), "{text}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn ab_diff_reports_improved_across_a_real_pair() {
    let pre = scratch("ab_pre");
    let post = scratch("ab_post");
    write_sidecar(&pre, "carrier.frames.json", &sidecar_text("carrier", 10.0, (45, 750)));
    write_sidecar(&post, "carrier.frames.json", &sidecar_text("carrier", 8.0, (45, 750)));
    let (code, text) = heal(&[
        "--frame-time-report",
        pre.to_str().expect("utf8"),
        "--vs",
        post.to_str().expect("utf8"),
    ]);
    assert_eq!(code, Some(0), "output:\n{text}");
    assert!(text.contains("IMPROVED"), "{text}");
    assert!(text.contains("frame-time A/B diff"), "{text}");
    fs::remove_dir_all(&pre).ok();
    fs::remove_dir_all(&post).ok();
}

#[test]
fn ab_diff_refuses_a_warmup_mismatch() {
    let pre = scratch("warm_pre");
    let post = scratch("warm_post");
    write_sidecar(&pre, "c.frames.json", &sidecar_text("c", 10.0, (45, 750)));
    write_sidecar(&post, "c.frames.json", &sidecar_text("c", 10.0, (10, 100)));
    let (code, text) = heal(&[
        "--frame-time-report",
        pre.to_str().expect("utf8"),
        "--vs",
        post.to_str().expect("utf8"),
    ]);
    assert_eq!(code, Some(1), "output:\n{text}");
    assert!(text.contains("warmup-parity"), "{text}");
    fs::remove_dir_all(&pre).ok();
    fs::remove_dir_all(&post).ok();
}

#[test]
fn ab_diff_refuses_ambiguous_pairings() {
    let pre = scratch("amb_pre");
    let post = scratch("amb_post");
    write_sidecar(&pre, "c.frames.json", &sidecar_text("c", 10.0, (45, 750)));
    write_sidecar(&post, "c.frames.json", &sidecar_text("c", 10.0, (45, 750)));
    write_sidecar(&post, "c.b02.frames.json", &sidecar_text("c", 9.0, (45, 750)));
    let (code, text) = heal(&[
        "--frame-time-report",
        pre.to_str().expect("utf8"),
        "--vs",
        post.to_str().expect("utf8"),
    ]);
    assert_eq!(code, Some(1), "output:\n{text}");
    assert!(text.contains("ambiguous-pairing"), "{text}");
    fs::remove_dir_all(&pre).ok();
    fs::remove_dir_all(&post).ok();
}
