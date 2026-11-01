//! The `kernel_opt` GOAT gate G2 harness — the per-rule interleaved A/B
//! microbench over the deterministic tranche (Issue 191 T5 machinery).
//!
//! ## The pre-registered measurement law (T1 bar 2, verbatim-shaped)
//!
//! - **Tranche selector (deterministic, pinned before measurement):** corpus
//!   files sorted by their GREEN rules' embedded `#[test]` token count
//!   descending, tie-break file name ascending, truncated at 15 files.
//! - **Interleaved A/B:** per mirrored pair, `(ref chunk, opt chunk)`
//!   interleaved `PAIRS` times; per-pair speedup `t_ref / t_opt`; the pair's
//!   verdict is the **median** across pairs. A ratio of two SEQUENTIAL arms
//!   measures the box — the interleave is the defense.
//! - **Bar:** median speedup ≥ 1.06 at `--release` (the measured flake band
//!   is < 6 pts). A median < 1.0 is a measured regression — the rule exits
//!   the benchable set with the reason recorded per-rule.
//! - **Loud zero:** results and inputs go through `std::hint::black_box`;
//!   a chunk that times zero (deleted work) fails the pair, never passes it.
//! - **Posture:** `rustc --edition 2021 -O` (release-class optimization),
//!   pid-suffixed temp dir, per-rule compile + run with timeouts.
//!
//! ## What is benchable (the extraction convention, measured)
//!
//! 46 of the 183 G1-green rules expose ≥ 1 **mirrored top-level pair** — a
//! `fn` of the same name and identical raw parameter list in BOTH arms (the
//! ref/opt bodies implement the naive and optimized forms of the same
//! function). Only mirrored pairs bench; everything else is recorded
//! `NOT-MEASURED` with the per-rule reason (the ratified disclosure). Inputs
//! are synthesized type-directed and deterministic; a parameter whose type
//! has no generator disqualifies that pair, not the rule (unless no pair
//! survives).

use serde::Serialize;

use super::goat_g1::{self, G1Record};
use super::KernelRule;

/// Files in the tranche: GREEN `#[test]` token count per file, descending;
/// tie-break file name ascending; truncated at [`TRANCHE_FILES`].
pub const TRANCHE_FILES: usize = 15;

/// Interleaved (ref chunk, opt chunk) pairs per measured pair.
pub const PAIRS: usize = 15;
/// Warmup calls before the timed chunks (both arms, same count).
pub const WARMUP: u32 = 3;
/// Target per-chunk wall time budget the calibration scales into (ns).
const TARGET_CHUNK_NS: u128 = 2_000_000;

/// The file-tranche rows (selector output, disclosed in the record).
#[derive(Debug, Clone, Serialize)]
pub struct TrancheFile {
    pub file: String,
    /// `#[test]` token count over the file's G1-green rules' strings.
    pub green_test_tokens: usize,
    pub green_rules: usize,
}

/// The deterministic tranche: consumes the committed G1 store (via
/// [`goat_g1::g1_record_for`]) and the corpus builders.
#[must_use]
pub fn tranche() -> Vec<TrancheFile> {
    let mut rows: Vec<TrancheFile> = Vec::new();
    for (file, entries) in goat_g1::per_file_entries_for_tranche() {
        let mut tokens = 0usize;
        let mut greens = 0usize;
        for rule in entries {
            if goat_g1::g1_record_for(&rule.rule_id, &rule.ref_shape, &rule.opt_pattern)
                != G1Record::Green
            {
                continue;
            }
            greens += 1;
            tokens += rule.ref_shape.matches("#[test]").count()
                + rule.opt_pattern.matches("#[test]").count();
        }
        if greens > 0 {
            rows.push(TrancheFile {
                file,
                green_test_tokens: tokens,
                green_rules: greens,
            });
        }
    }
    rows.sort_by(|a, b| {
        b.green_test_tokens
            .cmp(&a.green_test_tokens)
            .then_with(|| a.file.cmp(&b.file))
    });
    rows.truncate(TRANCHE_FILES);
    rows.shrink_to_fit();
    rows
}

/// One mirrored pair extracted from a rule's arms.
#[derive(Debug, Clone)]
pub struct MirroredPair {
    pub fn_name: String,
    /// The identical raw parameter list (the signature gate).
    pub params: String,
}

/// Column-0 top-level `fn` names + raw parameter lists of one arm.
#[must_use]
pub fn top_level_fns(arm_src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in arm_src.lines() {
        let rest = line.strip_prefix("pub fn ").or_else(|| line.strip_prefix("fn "));
        let Some(rest) = rest else { continue };
        let Some(paren) = rest.find('(') else { continue };
        let name = rest[..paren].trim();
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue; // generics (`fn f<T>(`) and where-clauses stay unbenchable
        }
        let Some(close_rel) = rest[paren..].find(')') else {
            continue; // multi-line param lists stay unbenchable
        };
        let params = rest[paren + 1..paren + close_rel].trim().to_string();
        out.push((name.to_string(), params));
    }
    out.shrink_to_fit();
    out
}

/// The rule's mirrored pairs: same fn name, IDENTICAL raw parameter list,
/// at least one parameter (a zero-arg fn has nothing our harness can
/// measure — the call would be a constant).
#[must_use]
pub fn mirrored_pairs(rule: &KernelRule) -> Vec<MirroredPair> {
    let ref_fns = top_level_fns(&rule.ref_shape);
    let opt_fns = top_level_fns(&rule.opt_pattern);
    let mut out = Vec::with_capacity(ref_fns.len());
    for (name, params) in &ref_fns {
        if params.is_empty() {
            continue;
        }
        if opt_fns.iter().any(|(n, p)| n == name && p == params)
            && !out.iter().any(|m: &MirroredPair| &m.fn_name == name)
        {
            out.push(MirroredPair {
                fn_name: name.clone(),
                params: params.clone(),
            });
        }
    }
    out.shrink_to_fit();
    out
}

/// One synthesized argument expression (rendered into the bench source).
struct ArgGen(String);

/// Deterministic input synthesis per parameter (`name: type` — the type part
/// decides; `idx` salts the value so multi-arg pairs get distinct inputs,
/// still fully deterministic).
fn synth_param(param: &str, idx: usize) -> Option<ArgGen> {
    let ty = param
        .split_once(':')
        .map_or(param.trim(), |(_, t)| t.trim());
    // Dimension-like integers stay SMALL: the fixed 4096-element input slices
    // bound any index math a corpus fn performs on them (rows*cols ≤ ~2601),
    // and loop bounds like `0..n` stay inside the slice. Both arms receive
    // the SAME values, so the A/B comparison is unaffected. The arithmetic
    // happens IN the target integer type, so no narrowing cast is owed.
    let int_salt_u =
        |base: u32| (base.wrapping_add(u32::try_from(idx).unwrap_or(0).wrapping_mul(0x9E37_79B9)) % 48) + 4;
    let int_salt_s =
        |base: i32| (base.wrapping_add(i32::try_from(idx).unwrap_or(0).wrapping_mul(0x0013_79B9)) % 48) + 4;
    Some(match ty {
        "&[u8]" => ArgGen("&INPUT_BYTES".to_string()),
        "&[f32]" => ArgGen("&INPUT_F32".to_string()),
        "&[f64]" => ArgGen("&INPUT_F64".to_string()),
        "&str" => ArgGen("INPUT_STR".to_string()),
        "usize" => ArgGen(format!("{}usize", int_salt_u(1024))),
        "u64" => ArgGen("u64::from_le_bytes(INPUT_BYTES8) % 64".to_string()),
        "i64" => ArgGen("(i64::from_le_bytes(INPUT_BYTES8) % 64)".to_string()),
        "u32" => ArgGen(format!("{}u32", int_salt_u(0xDEAD_BEEF))),
        "i32" => ArgGen(format!("{}i32", int_salt_s(0x5EED_1234))),
        "f32" => ArgGen("INPUT_F32[7]".to_string()),
        "f64" => ArgGen("INPUT_F64[7]".to_string()),
        "bool" => ArgGen("true".to_string()),
        "char" => ArgGen("'k'".to_string()),
        _ => return None,
    })
}

/// Top-level comma split of a parameter list (no nested generics survive the
/// extraction gate, so a plain split is exact here).
fn split_params(params: &str) -> Vec<&str> {
    if params.trim().is_empty() {
        Vec::new()
    } else {
        params.split(',').collect()
    }
}

/// Whether every parameter of `params` synthesizes (an empty list synthesizes
/// nothing — zero-arg pairs never reach this gate).
fn synthesizable(params: &str) -> bool {
    let split = split_params(params);
    !split.is_empty()
        && split
            .iter()
            .enumerate()
            .all(|(i, p)| synth_param(p, i).is_some())
}

/// The synthesized argument expression list for one parameter list
/// (synthesizability is gated upstream; every param renders).
fn arg_list_of(params: &str) -> String {
    split_params(params)
        .iter()
        .enumerate()
        .map(|(i, p)| {
            synth_param(p, i).map_or_else(|| unreachable!("gated by synthesizable"), |a| a.0)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Split an arm's leading inner attributes (`#![...]`, single-line each) off
/// the text — skipping over leading `//` comment lines to find them (the
/// corpus arms open with `// optimized: …` prose, THEN the attr). In G1 the
/// ref text opens the file so its inner attrs are valid; in the G2 shape both
/// arms land AFTER generated items, where an inner attribute is a hard error
/// — the attrs hoist to the generated file's top, the comments stay with the
/// arm's text.
fn split_inner_attrs(text: &str) -> (String, String) {
    let mut attrs = String::new();
    let mut kept = String::new(); // comments (and anything) that stays with the text
    let mut rest = text;
    loop {
        let trimmed = rest.trim_start();
        if trimmed.starts_with("#![") {
            let line_end = trimmed.find('\n').map_or(trimmed.len(), |i| i + 1);
            let line = &trimmed[..line_end];
            if !line.trim_end().ends_with(']') {
                kept.push_str(trimmed);
                break; // multi-line inner attr stays with the text (disclosed error)
            }
            attrs.push_str(line);
            rest = &trimmed[line_end..];
            continue;
        }
        // a run of whole-line comments (and blank lines) may precede the next
        // attr — corpus arms separate the prose block from `#![allow]` with
        // blank lines, and none of those are items
        let mut scan = trimmed;
        let mut comments = String::new();
        loop {
            let line_end = scan.find('\n').map_or(scan.len(), |i| i + 1);
            let line = &scan[..line_end];
            if line.trim_start().starts_with("//") || line.trim().is_empty() {
                comments.push_str(line);
                scan = &scan[line_end..];
            } else {
                break;
            }
        }
        if !comments.is_empty() && scan.starts_with("#![") {
            kept.push_str(&comments);
            rest = scan; // loop re-enters with the attr now at the front
            continue;
        }
        kept.push_str(trimmed);
        break;
    }
    (attrs, kept)
}

/// The synthesized bench program for one rule (compile + run per rule).
///
/// Architecture (the G1 privacy lesson, mirrored): the measured call lives
/// INSIDE each arm's own scope — the opt side's mirrored fn may be private
/// inside `mod __opt`, and `main` cannot call it — so each pair synthesizes
/// a chunk-timing fn PER SIDE (`__bench_ref_N` at the root, `pub fn
/// __bench_opt_N` inside `__opt`; a chunk fn returns u128, so no return-type
/// synthesis is owed) and `main` only interleaves the chunks and prints the
/// PAIR rows.
#[must_use]
pub fn bench_source(rule: &KernelRule, pairs: &[MirroredPair]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(rule.ref_shape.len() + rule.opt_pattern.len() + 8192);
    s.push_str("// synthesized by kernel_goat_g2 (Issue 191 T5) — deterministic inputs,\n");
    s.push_str("// interleaved (ref, opt) chunk pairs, median speedup across pairs.\n");
    // The corpus arms' leading inner attributes hoist HERE (before any item,
    // where alone they are legal) — mid-file they are a hard error.
    let (ref_attrs, ref_text) = split_inner_attrs(&rule.ref_shape);
    let (opt_attrs, opt_text) = split_inner_attrs(&rule.opt_pattern);
    s.push_str(&ref_attrs);
    s.push_str(&opt_attrs);
    if !ref_attrs.is_empty() || !opt_attrs.is_empty() {
        s.push('\n');
    }
    s.push_str("use std::hint::black_box;\nuse std::time::Instant;\n\n");
    s.push_str("const INPUT_BYTES8: [u8; 8] = [0x1F, 0x2E, 0x3D, 0x4C, 0x5B, 0x6A, 0x79, 0x88];\n");
    s.push_str("const fn input_byte(i: usize) -> u8 { (i.wrapping_mul(7).wrapping_add(i / 13)) as u8 }\n");
    s.push_str("const fn build_bytes() -> [u8; 4096] { let mut out = [0u8; 4096]; let mut i = 0; while i < 4096 { out[i] = input_byte(i); i += 1; } out }\n");
    s.push_str("const fn build_f32() -> [f32; 4096] { let mut out = [0f32; 4096]; let mut i = 0; while i < 4096 { out[i] = ((i % 251) as f32) * 0.125 - 15.0; i += 1; } out }\n");
    s.push_str("const fn build_f64() -> [f64; 4096] { let mut out = [0f64; 4096]; let mut i = 0; while i < 4096 { out[i] = ((i % 241) as f64) * 0.0625 - 7.5; i += 1; } out }\n");
    s.push_str("static INPUT_BYTES: [u8; 4096] = build_bytes();\n");
    s.push_str("static INPUT_F32: [f32; 4096] = build_f32();\n");
    s.push_str("static INPUT_F64: [f64; 4096] = build_f64();\n");
    s.push_str("const INPUT_STR: &str = \"the quick brown fox jumps over the lazy dog 0123456789\";\n\n");
    // The G1 pair shape: ref at the crate root, opt in a bridged module.
    s.push_str(&ref_text);
    s.push('\n');
    // Per-pair REF chunk fns at the root (the ref fns are visible there).
    for (idx, pair) in pairs.iter().enumerate() {
        let arg_list = arg_list_of(&pair.params);
        let _ = writeln!(
            s,
            "fn __bench_ref_{idx}(iters: u32) -> u128 {{ let t = Instant::now(); for _ in 0..iters {{ black_box(black_box({})({arg_list})); }} t.elapsed().as_nanos() }}\n",
            pair.fn_name
        );
    }
    s.push_str("\nmod __opt {\nuse super::*;\n");
    // Per-pair OPT chunk fns BEFORE the corpus text (the brace-swallow
    // defense: a corpus opt arm with an UNBALANCED open brace — masked in G1
    // because its shape appends nothing after the text — would otherwise
    // swallow the appended fns into a fn body, invisible to main. Items are
    // order-independent in Rust, so the forward reference compiles).
    for (idx, pair) in pairs.iter().enumerate() {
        let arg_list = arg_list_of(&pair.params);
        let _ = writeln!(
            s,
            "pub fn __bench_opt_{idx}(iters: u32) -> u128 {{ let t = Instant::now(); for _ in 0..iters {{ black_box(black_box({})({arg_list})); }} t.elapsed().as_nanos() }}\n",
            pair.fn_name
        );
    }
    s.push_str(&opt_text);
    s.push_str("\n}");
    s.push_str("\n\n");
    // The interleaved A/B harness: calibration on the ref (iters fixed for
    // BOTH arms from ONE read), then PAIRS interleaved (ref, opt) chunks.
    // Compiled with `--test`: a corpus arm may define its OWN `fn main`
    // (E0428 under a bin — measured), and libtest's entry is then the only
    // one. The PAIR rows go to a FILE (a test binary captures stdout); the
    // per-process run dir is pid-suffixed, so the fixed name is process-local.
    s.push_str("#[test]\nfn __g2_harness() {\n    let mut out = String::new();\n");
    for (idx, pair) in pairs.iter().enumerate() {
        let _ = writeln!(
            s,
            "{{ // mirrored pair `{}`({}) — idx {}\n",
            pair.fn_name, pair.params, idx
        );
        s.push_str("    let mut iters = 1u32;\n");
        let _ = writeln!(
            s,
            "    for _ in 0..{WARMUP} {{ __bench_ref_{idx}(1); __opt::__bench_opt_{idx}(1); }}\n"
        );
        let _ = writeln!(
            s,
            "    loop {{ let d = __bench_ref_{idx}(iters); if d >= {TARGET_CHUNK_NS} || iters >= 100_000 {{ break; }} iters = (iters * 4).min(100_000); }}\n"
        );
        let _ = writeln!(
            s,
            "    let mut ratios = [0.0f64; {PAIRS}]; let mut zero = false;\n"
        );
        let _ = writeln!(
            s,
            "    for r in ratios.iter_mut() {{ let a = __bench_ref_{idx}(iters); let b = __opt::__bench_opt_{idx}(iters); if a == 0 || b == 0 {{ zero = true; }} *r = if b == 0 {{ f64::NAN }} else {{ a as f64 / b as f64 }}; }}\n"
        );
        s.push_str(
            "    ratios.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));\n",
        );
        let _ = writeln!(
            s,
            "    let med = ratios[{PAIRS} / 2];\n    out.push_str(&format!(\"PAIR\\t{{}}\\t{{}}\\t{{}}\\t{{}}\\t{{}}\\n\", \"{}\", {idx}, iters, if zero {{ \"zero\" }} else {{ \"ok\" }}, med));\n",
            pair.fn_name,
        );
        s.push_str("}\n");
    }
    s.push_str("    std::fs::write(\"pairs.tsv\", &out).expect(\"write pairs\");\n}");
    s
}

/// One mirrored pair's measured verdict.
#[derive(Debug, Clone, Serialize)]
pub struct PairVerdict {
    pub fn_name: String,
    pub iters: u32,
    /// `true` unless a chunk timed 0 ns (the loud-zero law: deleted work
    /// fails, never passes).
    pub measured: bool,
    /// Median across pairs of `t_ref / t_opt` (≥ 1.0 means opt is faster).
    pub median_speedup: f64,
}

impl PairVerdict {
    /// The T1 bar: median speedup ≥ 1.06.
    #[must_use]
    pub fn passes(&self) -> bool {
        self.measured && self.median_speedup >= 1.06
    }

    /// A measured regression (median < 1.0 — exits the benchable set).
    #[must_use]
    pub fn regresses(&self) -> bool {
        self.measured && self.median_speedup < 1.0
    }
}

/// One rule's G2 verdict.
#[derive(Debug, Clone, Serialize)]
pub struct RuleG2 {
    pub rule_id: String,
    pub file: String,
    /// One verdict per measured mirrored pair.
    pub pairs: Vec<PairVerdict>,
    /// `NOT-MEASURED` reason when no pair survived extraction/synthesis.
    pub not_measured_reason: Option<String>,
    /// The compile/run error of the synthesized bench, when it failed.
    pub error: Option<String>,
}

impl RuleG2 {
    /// Median speedup across the rule's pairs (the per-rule read: the max of
    /// the pair medians — the opt form's best measured win).
    #[must_use]
    pub fn best_speedup(&self) -> Option<f64> {
        self.pairs
            .iter()
            .filter(|p| p.measured)
            .map(|p| p.median_speedup)
            .reduce(f64::max)
    }

    /// The rule clears the bar when ANY mirrored pair does.
    #[must_use]
    pub fn passes(&self) -> bool {
        self.pairs.iter().any(PairVerdict::passes)
    }

    /// A measured regression with no passing pair (exits the benchable set).
    #[must_use]
    pub fn regresses(&self) -> bool {
        !self.passes() && self.pairs.iter().any(PairVerdict::regresses)
    }
}

/// Extract + synthesize + compile + run ONE rule's bench. The compile/run
/// half mirrors `goat_g1`'s machinery (pid-suffixed dir, timeouts, error
/// files read BEFORE the temp dir is removed).
#[must_use]
pub fn bench_rule(rule: &KernelRule, file: &str) -> RuleG2 {
    let mut pairs = mirrored_pairs(rule)
        .into_iter()
        .filter(|p| synthesizable(&p.params))
        .collect::<Vec<_>>();
    if pairs.is_empty() {
        return RuleG2 {
            rule_id: rule.rule_id.clone(),
            file: file.to_string(),
            pairs: Vec::new(),
            not_measured_reason: Some(
                "no mirrored top-level fn pair with synthesizable params \
                 (the extraction convention: same name + identical raw \
                 parameter list in both arms)"
                    .to_string(),
            ),
            error: None,
        };
    }
    // Keep at most 4 pairs per rule (compile-time bound; extraction order).
    pairs.truncate(4);
    let names: Vec<String> = pairs.iter().map(|p| p.fn_name.clone()).collect();
    let dir = goat_g1::work_dir();
    let _ = std::fs::create_dir_all(&dir);
    let src_path = dir.join("bench.rs");
    let bin_path = dir.join("bench.exe");
    let source = bench_source(rule, &pairs);
    if let Err(e) = std::fs::write(&src_path, source) {
        return compile_failed(rule, file, format!("write failed: {e}"));
    }
    let c_out = dir.join("b_c_out");
    let c_err = dir.join("b_c_err");
    let mut cmd = std::process::Command::new("rustc");
    cmd.arg("--edition")
        .arg("2021")
        .arg("--test")
        .arg("-O")
        .arg(&src_path)
        .arg("-o")
        .arg(&bin_path)
        .current_dir(&dir);
    let compiled = goat_g1::run_with_timeout(cmd, goat_g1::COMPILE_TIMEOUT, &c_out, &c_err);
    match compiled {
        Ok(status) if !status.success() => {
            // read BEFORE the temp dir goes away (the error files live there);
            // the HEAD of rustc's stderr carries the first error — the tail is
            // the note/help noise
            let err = head_of(&read_of(&c_err), 600);
            let _ = std::fs::remove_dir_all(&dir);
            return compile_failed(rule, file, format!("rustc: {err}"));
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir);
            return compile_failed(rule, file, format!("rustc: {e}"));
        }
        Ok(_) => {}
    }
    let r_out = dir.join("b_r_out");
    let r_err = dir.join("b_r_err");
    let mut run = std::process::Command::new(&bin_path);
    run.current_dir(&dir);
    let ran = goat_g1::run_with_timeout(run, goat_g1::RUN_TIMEOUT, &r_out, &r_err);
    // The PAIR rows live in pairs.tsv (a test binary captures stdout). A
    // corpus #[test] failure exits the binary non-zero WITHOUT invalidating
    // the harness's own rows — the file wins when it carries rows.
    let pairs_path = dir.join("pairs.tsv");
    let rows = read_of(&pairs_path);
    match ran {
        Ok(_) if !rows.is_empty() => {
            let _ = std::fs::remove_dir_all(&dir);
            parse_pairs(&rows, rule, file, &names)
        }
        Ok(status) if status.success() => {
            let _ = std::fs::remove_dir_all(&dir);
            compile_failed(rule, file, "bench run produced no PAIR rows".to_string())
        }
        Ok(_) => {
            let err = tail_of(&read_of(&r_err), 400);
            let _ = std::fs::remove_dir_all(&dir);
            compile_failed(rule, file, format!("bench run failed: {err}"))
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir);
            compile_failed(rule, file, format!("bench run: {e}"))
        }
    }
}

fn compile_failed(rule: &KernelRule, file: &str, error: String) -> RuleG2 {
    RuleG2 {
        rule_id: rule.rule_id.clone(),
        file: file.to_string(),
        pairs: Vec::new(),
        not_measured_reason: None,
        error: Some(error),
    }
}

/// Parse the bench's PAIR stdout rows
/// (`PAIR\t<fn_name>\t<idx>\t<iters>\t<state>\t<med>`); the name comes from
/// the row, idx is the cross-check.
fn parse_pairs(stdout: &str, rule: &KernelRule, file: &str, names: &[String]) -> RuleG2 {
    let mut pairs = Vec::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("PAIR\t") else {
            continue;
        };
        let mut f = rest.split('\t');
        let (Some(name), Some(idx), Some(iters), Some(state), Some(med)) =
            (f.next(), f.next(), f.next(), f.next(), f.next())
        else {
            continue;
        };
        let Ok(_idx) = idx.parse::<usize>() else {
            continue;
        };
        let fn_name = if names.iter().any(|n| n == name) {
            name.to_string()
        } else {
            names.first().cloned().unwrap_or_else(|| name.to_string())
        };
        pairs.push(PairVerdict {
            fn_name,
            iters: iters.parse().unwrap_or(0),
            measured: state != "zero",
            median_speedup: med.parse().unwrap_or(f64::NAN),
        });
    }
    if pairs.is_empty() {
        return compile_failed(rule, file, "bench produced no PAIR rows".to_string());
    }
    RuleG2 {
        rule_id: rule.rule_id.clone(),
        file: file.to_string(),
        pairs,
        not_measured_reason: None,
        error: None,
    }
}

fn tail_of(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        s.to_string()
    } else {
        let cut = (s.len() - max..s.len())
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(s.len());
        format!("…{}", &s[cut..])
    }
}

fn head_of(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        s.to_string()
    } else {
        let cut = (0..s.len())
            .find(|&i| i >= max && s.is_char_boundary(i))
            .unwrap_or(s.len());
        format!("{}…", &s[..cut])
    }
}

fn read_of(p: &std::path::Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

/// The full G2 tranche run: every GREEN rule in the tranche files.
#[must_use]
pub fn run_g2() -> (Vec<TrancheFile>, Vec<RuleG2>) {
    let files = tranche();
    let file_set: std::collections::HashSet<&str> =
        files.iter().map(|f| f.file.as_str()).collect();
    let mut rules = Vec::new();
    for (file, entries) in goat_g1::per_file_entries_for_tranche() {
        if !file_set.contains(file.as_str()) {
            continue;
        }
        for rule in entries {
            if goat_g1::g1_record_for(&rule.rule_id, &rule.ref_shape, &rule.opt_pattern)
                != G1Record::Green
            {
                continue;
            }
            rules.push(bench_rule(&rule, &file));
        }
    }
    (files, rules)
}

/// The G2 aggregate: measured / passing / regressing / not-measured + the
/// tranche median.
#[derive(Debug, Clone, Serialize)]
pub struct G2Report {
    pub tranche: Vec<TrancheFile>,
    pub rules: Vec<RuleG2>,
    pub measured: usize,
    pub passing: usize,
    pub regressing: usize,
    pub not_measured: usize,
    /// Median of the per-rule best speedups over MEASURED rules.
    pub tranche_median_speedup: f64,
    pub seconds: f64,
}

/// Aggregate a finished run (pure — the runner calls this for the verdict).
/// The `RuleG2::best_speedup` method reference rides `filter_map` directly
/// (the redundant-closure law).
#[must_use]
pub fn aggregate(tranche: Vec<TrancheFile>, rules: Vec<RuleG2>, seconds: f64) -> G2Report {
    let measured = rules.iter().filter(|r| !r.pairs.is_empty()).count();
    let passing = rules.iter().filter(|r| r.passes()).count();
    let regressing = rules.iter().filter(|r| r.regresses()).count();
    let mut speeds: Vec<f64> = rules
        .iter()
        .filter_map(RuleG2::best_speedup)
        .filter(|s| s.is_finite())
        .collect();
    speeds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let tranche_median_speedup = if speeds.is_empty() {
        0.0
    } else {
        speeds[speeds.len() / 2]
    };
    G2Report {
        not_measured: rules.len() - measured,
        tranche,
        rules,
        measured,
        passing,
        regressing,
        tranche_median_speedup,
        seconds,
    }
}

impl G2Report {
    /// The T1 bar 2 verdict: the tranche median ≥ 1.06 AND at least one
    /// measured rule (a zero-measurement tranche cannot demonstrate the bar).
    /// Regressions are disclosed per-rule and exit the benchable set (the
    /// demotion law) — they do not fail the bar by themselves.
    ///
    /// # Errors
    /// The named failing axis/axes when the bar does not hold.
    pub fn verdict(&self) -> Result<(), String> {
        let mut problems = Vec::new();
        if self.measured == 0 {
            problems.push(
                "G2: zero rules measured — the tranche cannot demonstrate the bar".to_string(),
            );
        }
        if self.tranche_median_speedup < 1.06 {
            problems.push(format!(
                "G2: tranche median speedup {:.4} < 1.06",
                self.tranche_median_speedup
            ));
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(ref_src: &str, opt_src: &str) -> KernelRule {
        KernelRule {
            rule_id: "test-rule".to_string(),
            gpu_concept: String::new(),
            ref_shape: ref_src.to_string(),
            opt_pattern: opt_src.to_string(),
        }
    }

    #[test]
    fn top_level_fns_extracts_column_zero_only() {
        let src = "fn plain(a: u32) -> u32 { a }\nfn nested() {\n    fn inner(x: u8) -> u8 { x }\n    let _ = inner(1);\n}\npub fn exported(s: &str) -> usize { s.len() }\nfn generic<T>(t: T) -> T { t }\nfn multiline(\n    a: u32,\n) -> u32 { a }\n";
        let fns = top_level_fns(src);
        let names: Vec<_> = fns.iter().map(|(n, _)| n.as_str()).collect();
        // `nested` IS column-0 (its BODY holds the nested fn — irrelevant);
        // the indented `inner`, the generic, and the multiline list are out.
        assert_eq!(names, vec!["plain", "nested", "exported"]);
        assert_eq!(fns[0].1, "a: u32");
        assert_eq!(fns[2].1, "s: &str");
    }

    #[test]
    fn mirrored_pairs_require_identical_params() {
        let r = rule(
            "fn work(data: &[u8]) -> u32 { 1 }\nfn other(x: i32) -> i32 { x }",
            "fn work(data: &[u8]) -> u32 { 2 }\nfn other(y: i64) -> i64 { 0 }",
        );
        let m = mirrored_pairs(&r);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].fn_name, "work");
        assert_eq!(m[0].params, "data: &[u8]");
    }

    #[test]
    fn mirrored_pairs_exclude_zero_arg_fns() {
        let r = rule("fn constant() -> u32 { 1 }", "fn constant() -> u32 { 2 }");
        assert!(mirrored_pairs(&r).is_empty());
    }

    #[test]
    fn synthesis_covers_the_common_primitive_surface() {
        for (params, ok) in [
            ("data: &[u8]", true),
            ("a: usize, b: f32", true),
            ("s: &str, n: u32", true),
            ("g: Graph", false),
            ("xs: Vec<u32>", false),
            ("", false), // zero-arg never reaches synthesis, but is unsynthesizable
        ] {
            assert_eq!(synthesizable(params), ok, "params {params:?}");
        }
    }

    #[test]
    fn bench_source_synthesizes_both_arms_and_the_pair_harness() {
        let r = rule(
            "fn work(data: &[u8]) -> u32 { data.len() as u32 }",
            "fn work(data: &[u8]) -> u32 { (data.len() / 2) as u32 }",
        );
        let m = mirrored_pairs(&r);
        let src = bench_source(&r, &m);
        assert!(src.contains("fn work(data: &[u8]) -> u32 { data.len() as u32 }"));
        assert!(src.contains("mod __opt {"));
        // the measured call lives INSIDE each arm's own scope (privacy)
        assert!(src.contains("fn __bench_ref_0(iters: u32) -> u128"));
        assert!(src.contains("pub fn __bench_opt_0(iters: u32) -> u128"));
        assert!(src.contains("black_box"));
        assert!(src.contains("PAIR"));
        assert!(src.contains("ratios.sort_by"));
    }

    #[test]
    fn aggregate_and_verdict_read_the_pre_registered_bar() {
        let mk_pair = |speedup: f64, measured: bool| PairVerdict {
            fn_name: "work".to_string(),
            iters: 100,
            measured,
            median_speedup: speedup,
        };
        let passing = RuleG2 {
            rule_id: "p".to_string(),
            file: "f".to_string(),
            pairs: vec![mk_pair(1.10, true)],
            not_measured_reason: None,
            error: None,
        };
        let below = RuleG2 {
            rule_id: "b".to_string(),
            file: "f".to_string(),
            pairs: vec![mk_pair(1.02, true)],
            not_measured_reason: None,
            error: None,
        };
        let regression = RuleG2 {
            rule_id: "r".to_string(),
            file: "f".to_string(),
            pairs: vec![mk_pair(0.90, true)],
            not_measured_reason: None,
            error: None,
        };
        let nm = RuleG2 {
            rule_id: "n".to_string(),
            file: "f".to_string(),
            pairs: Vec::new(),
            not_measured_reason: Some("no mirrored pair".to_string()),
            error: None,
        };
        let rules = vec![passing, below, regression, nm];
        let report = aggregate(Vec::new(), rules, 1.0);
        assert_eq!(report.measured, 3);
        assert_eq!(report.passing, 1);
        assert_eq!(report.regressing, 1);
        assert_eq!(report.not_measured, 1);
        // median of [0.90, 1.02, 1.10] = 1.02 < 1.06 → the bar FAILS
        assert!(report.verdict().is_err());
    }
}
