//! Issue 040 — the fixer re-gate POC runner.
//!
//! Arms:
//! - `--arm A` — CF qwen3.8-27b raw prompt (needs `CF_API_TOKEN` +
//!   `CF_ACCOUNT_ID`)
//! - `--arm B` — same model, modelless pre-pass prompt (candidate + corpus
//!   few-shots)
//! - `--arm C` — local Bonsai via the `bonsai_clippy_l4_eval --serve` daemon
//!   (`--l4-bin/--l4-gguf/--l4-lora`; note the riir-train driver REQUIRES
//!   `--lora`, so arm C needs a checkpoint — see the issue Status section)
//! - `--arm DRY` (default) — offline harness validation: golden prompt
//!   snapshots (blake3) + scoring of PLANTED fixes (positive + negative
//!   controls). No model, no network. This is the "arms pending" evidence
//!   mode.
//!
//! Run:
//!
//! ```sh
//! # offline validation (always runnable)
//! cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm DRY
//!
//! # arms A+B (once CF_API_TOKEN + CF_ACCOUNT_ID are exported)
//! cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm A
//! cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm B
//!
//! # arm C smoke (checkpoint + idle GPU required; disclose load)
//! cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm C \
//!     --l4-bin /path/to/bonsai_clippy_l4_eval \
//!     --l4-gguf ~/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
//!     --l4-lora /path/to/ckpt.bcl4 --limit 8
//! ```

use std::io::Write as _;
use std::path::PathBuf;

use riir_clippy::cf_workers_ai::{
    self, ArmSummary, CaseScore, CfClient, CfUsageToday, FewShot, MissClass, RegateHarness,
    Sample, SampleSource, reachable_samples,
};
use riir_clippy::kat_redact::{GateDecision, RedactionGate, RedactionLog, SpanOrigin};
use riir_clippy::l4_daemon::DaemonL4;
use riir_clippy::ClippyVerifier;

/// Default per-M USD pricing for the cost column — the Proposal 005 cost
/// basis (qwen3.8-27b list). Overridable via `--price-in/--price-out`;
/// these are ASSUMPTIONS, not a quote.
const DEFAULT_PRICE_IN_PER_M: f64 = 0.45;
const DEFAULT_PRICE_OUT_PER_M: f64 = 3.2;
/// Published neuron rates for the default model (pricing page 2026-08-18):
/// 40,909 neurons / M input · 290,909 / M output · $0.011 / 1k neurons ·
/// 10,000 neurons/day free (resets 00:00 UTC; EXCEEDING IT ON WORKERS FREE
/// FAILS OPERATIONS — the fixer service must respect this ceiling).
const DEFAULT_NEURONS_IN_PER_M: f64 = 40_909.0;
const DEFAULT_NEURONS_OUT_PER_M: f64 = 290_909.0;
const NEURONS_FREE_DAILY: f64 = 10_000.0;

struct Args {
    arm: String,
    source: String,
    chunks: Vec<usize>,
    limit: usize,
    few_shots: usize,
    max_new: usize,
    data_dir: Option<PathBuf>,
    out: Option<PathBuf>,
    policy: Option<PathBuf>,
    model: Option<String>,
    l4_bin: Option<String>,
    l4_gguf: Option<String>,
    l4_lora: Option<String>,
    price_in: f64,
    price_out: f64,
    behav_dir: PathBuf,
}

impl Args {
    fn parse() -> Args {
        let mut a = Args {
            arm: "DRY".into(),
            source: "both".into(),
            chunks: vec![0],
            limit: 16,
            few_shots: 3,
            max_new: 128,
            data_dir: None,
            out: None,
            policy: None,
            model: None,
            l4_bin: None,
            l4_gguf: None,
            l4_lora: None,
            price_in: DEFAULT_PRICE_IN_PER_M,
            price_out: DEFAULT_PRICE_OUT_PER_M,
            behav_dir: std::env::temp_dir().join("issue040_behav"),
        };
        let mut argv = std::env::args().skip(1);
        while let Some(arg) = argv.next() {
            let mut val = || argv.next().unwrap_or_else(|| panic!("missing value for {arg}"));
            match arg.as_str() {
                "--arm" => a.arm = val(),
                "--source" => a.source = val(),
                "--chunks" => {
                    a.chunks = val()
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                }
                "--limit" => a.limit = val().parse().expect("--limit must be an integer"),
                "--few-shots" => a.few_shots = val().parse().expect("--few-shots must be an integer"),
                "--max-new" => a.max_new = val().parse().expect("--max-new must be an integer"),
                "--data-dir" => a.data_dir = Some(PathBuf::from(val())),
                "--out" => a.out = Some(PathBuf::from(val())),
                "--policy" => a.policy = Some(PathBuf::from(val())),
                "--model" => a.model = Some(val()),
                "--l4-bin" => a.l4_bin = Some(val()),
                "--l4-gguf" => a.l4_gguf = Some(val()),
                "--l4-lora" => a.l4_lora = Some(val()),
                "--price-in" => a.price_in = val().parse().expect("--price-in must be a float"),
                "--price-out" => a.price_out = val().parse().expect("--price-out must be a float"),
                "--behav-dir" => a.behav_dir = PathBuf::from(val()),
                other => {
                    eprintln!("unknown arg: {other}");
                    std::process::exit(2);
                }
            }
        }
        a
    }
}

/// Resolve the v2edit data dir: explicit flag > `RIIR_V2EDIT_DIR` >
/// cwd-relative `../riir-train/data`.
fn resolve_data_dir(flag: &Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(d) = flag {
        return Ok(d.clone());
    }
    if let Ok(d) = std::env::var("RIIR_V2EDIT_DIR") {
        return Ok(PathBuf::from(d));
    }
    for c in ["../riir-train/data", "../../riir-train/data"] {
        let p = std::path::Path::new(c);
        if p.join("clippy_l4_v2edit_test_chunk_00.jsonl").exists() {
            return Ok(p.to_path_buf());
        }
    }
    Err(
        "cannot locate the v2edit chunks — pass --data-dir <riir-train/data> or set \
         RIIR_V2EDIT_DIR (default probes ../riir-train/data)"
            .to_string(),
    )
}

fn build_samples(args: &Args) -> Result<Vec<Sample>, String> {
    let mut samples = Vec::new();
    if args.source == "reachable" || args.source == "both" {
        samples.extend(reachable_samples());
    }
    if args.source == "v2edit" || args.source == "both" {
        let dir = resolve_data_dir(&args.data_dir)?;
        samples.extend(cf_workers_ai::load_v2edit(&dir, &args.chunks, args.limit)?);
    }
    if samples.is_empty() {
        return Err(format!("--source {} produced zero samples", args.source));
    }
    Ok(samples)
}

/// T0.2 (Plan 026): run every sample through the redaction gate BEFORE any
/// arm sees it. Arms A/B send prompts to CF Workers AI (real egress) and
/// every arm writes corpus rows via `record_case` — a blocked span must
/// reach neither. Deny-by-default: no discoverable `.heal-sync-policy`
/// blocks all (fail-loud, exit 4 — running on zero samples is pointless).
///
/// Sample origins: the `reachable:` set is authored in-repo (fixtures);
/// the v2edit chunks are the pre-vetted mined research corpus. User code
/// has no path into this harness today; if one appears it maps to
/// `SpanOrigin::External`, which the shipped policy does not allow.
fn gate_samples(samples: Vec<Sample>, args: &Args) -> Vec<Sample> {
    let loaded = RedactionGate::discover(args.policy.as_deref());
    match (&loaded.path, &loaded.warning) {
        (Some(p), None) => eprintln!(
            "[regate] redaction policy: {} (v{}, origins {:?})",
            p.display(),
            riir_clippy::kat_redact::REDACT_VERSION,
            loaded.gate.policy().allow_origins
        ),
        (Some(_), Some(w)) => eprintln!("[regate] redaction policy: {w} — DENY-ALL in force"),
        (None, None) => eprintln!(
            "[regate] redaction policy: NONE FOUND — deny-by-default blocks every span; \
             create .heal-sync-policy at the repo root (Proposal 005 T0.2)"
        ),
        (None, Some(w)) => eprintln!("[regate] redaction policy: {w} — DENY-ALL in force"),
    }
    let log = RedactionLog::open(&loaded.root());
    let mut kept = Vec::with_capacity(samples.len());
    let mut blocked = 0usize;
    for mut s in samples {
        let origin = match s.source {
            SampleSource::Reachable => SpanOrigin::AuthoredFixture,
            SampleSource::V2edit => SpanOrigin::MinedCorpus,
        };
        let decision = loaded.gate.check(origin, None, &s.module);
        let fp = blake3::hash(s.module.as_bytes()).to_hex();
        let fp12 = &fp.to_string()[..12];
        if let Err(e) = log.record(&decision, origin, &s.id, fp12) {
            eprintln!("[regate] redaction log write failed: {e}");
        }
        match decision {
            GateDecision::Allow => kept.push(s),
            GateDecision::Redacted { payload, .. } => {
                eprintln!("[regate] REDACTED {} (findings replaced; policy allows redacted)", s.id);
                s.module = payload;
                kept.push(s);
            }
            GateDecision::Blocked { reason, findings } => {
                blocked += 1;
                let kinds: Vec<&str> = findings.iter().map(|f| f.kind.tag()).collect();
                let detail = if kinds.is_empty() {
                    String::new()
                } else {
                    format!(" [{kinds:?}]")
                };
                eprintln!("[regate] BLOCKED {} — {reason}{detail}", s.id);
            }
        }
    }
    eprintln!("[regate] redaction gate: {} allowed, {blocked} blocked", kept.len());
    if kept.is_empty() {
        eprintln!("FATAL: redaction gate blocked every sample — nothing to run");
        std::process::exit(4);
    }
    kept
}

/// One prepared case: sample + miss class + both prompt shapes.
struct Prepared {
    sample: Sample,
    miss: MissClass,
    modelless: Option<String>,
    #[allow(dead_code)] // kept for the JSONL row's diagnostics richness
    few_shots: Vec<FewShot>,
    prompt_raw: String,
    prompt_prepass: String,
    reachable_miss: bool,
}

fn prepare(harness: &mut RegateHarness, samples: Vec<Sample>, few_shots: usize) -> Vec<Prepared> {
    samples
        .into_iter()
        .map(|sample| {
            let (miss, modelless) = harness.classify(&sample.lints[0], &sample.module);
            let fs = harness.few_shots(&sample.module, few_shots);
            let raw = cf_workers_ai::prompt_raw(&sample.module);
            let prepass = cf_workers_ai::prompt_prepass(&sample.module, modelless.as_deref(), &fs);
            let reachable_miss = miss.is_reachable_miss();
            Prepared {
                sample,
                miss,
                modelless,
                few_shots: fs,
                prompt_raw: raw,
                prompt_prepass: prepass,
                reachable_miss,
            }
        })
        .collect()
}

/// Model-side accounting for one generated case (tokens + latency — the
/// daemon arm reports neither, hence defaults).
struct GenStats {
    latency_ms: u128,
    prompt_tokens: u64,
    completion_tokens: u64,
}

impl GenStats {
    fn empty() -> Self {
        Self { latency_ms: 0, prompt_tokens: 0, completion_tokens: 0 }
    }
}

fn record_case(
    out: &mut Option<std::fs::File>,
    p: &Prepared,
    arm: &str,
    prompt: &str,
    gen: &str,
    stats: &GenStats,
    score: &CaseScore,
) {
    if let Some(f) = out {
        let row = serde_json::json!({
            "id": p.sample.id,
            "arm": arm,
            "lints": p.sample.lints,
            "miss_class": format!("{:?}", p.miss),
            "modelless_served": p.modelless.is_some(),
            "reachable_miss": p.reachable_miss,
            "prompt_blake3_12": &blake3::hash(prompt.as_bytes()).to_hex().as_str()[..12],
            "prompt_chars": prompt.len(),
            "gen_chars": gen.len(),
            "latency_ms": stats.latency_ms,
            "prompt_tokens": stats.prompt_tokens,
            "completion_tokens": stats.completion_tokens,
            "score": {
                "em": score.em,
                "fragment_parses": score.fragment_parses,
                "merge": score.merge_strategy.map(|m| m.name()),
                "control_fires": score.control_fires,
                "oracle_ran": score.oracle_ran,
                "findings_before": score.findings_before,
                "findings_after": if score.findings_after == usize::MAX { None } else { Some(score.findings_after) },
                "strictly_fewer": score.strictly_fewer,
                "noop": score.noop,
                "contract_ok": score.contract_ok,
                "behavioural": score.behavioural,
                "keep_oracle": score.keep_oracle,
                "keep_strict": score.keep_strict,
            },
        });
        let _ = writeln!(f, "{row}");
    }
}

/// Planted fixes for the DRY arm — POSITIVE controls (known-good fixes: the
/// Bench 039 n=9 run's verified-clean wins + hand-written correct shapes)
/// and NEGATIVE controls (noop echo, a destructive item-drop, garbage).
fn planted_fixes() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        ("reachable:ptr_arg", "pub fn f(v: &[u8]) -> usize { v.len() }", true),
        (
            "reachable:or_fun_call",
            "pub fn f(opt: Option<u32>) -> u32 { opt.unwrap_or_else(expensive) }\npub fn expensive() -> u32 { 42 }",
            true,
        ),
        (
            "reachable:redundant_clone",
            "pub fn f() { let s = String::from(\"hi\"); drop(s); }",
            true,
        ),
        (
            "reachable:box_collection",
            "pub struct S { v: Vec<u8> }\npub fn mk() -> S { S { v: Vec::new() } }",
            true,
        ),
        // Negative: noop echo must never be kept.
        ("reachable:ptr_arg", "pub fn f(v: &Vec<u8>) -> usize { v.len() }", false),
        // Negative: destructive item-drop resolves the lint but loses mk.
        ("reachable:box_collection", "pub struct S { v: Vec<u8> }", false),
        // Negative: garbage never merges.
        ("reachable:ptr_arg", "}}}not rust{{{", false),
    ]
}

fn main() {
    let args = Args::parse();
    let t0 = std::time::Instant::now();

    let mut harness = match RegateHarness::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("FATAL: harness init failed: {e}");
            std::process::exit(1);
        }
    };
    let samples = match build_samples(&args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(1);
        }
    };
    // T0.2: gate BEFORE any arm dispatch — no egress, no corpus row, for a
    // span that has not passed (Plan 026).
    let samples = gate_samples(samples, &args);
    eprintln!(
        "[regate] {} samples (source={}, chunks={:?}, limit={}), arm={}",
        samples.len(),
        args.source,
        args.chunks,
        args.limit,
        args.arm
    );

    let prepared = prepare(&mut harness, samples, args.few_shots);
    std::fs::create_dir_all(&args.behav_dir).ok();
    // The harness is consumed by `prepare`; arms get their own verifier
    // (stateless per call — same instrument).
    let verifier = std::mem::replace(&mut harness.verifier, ClippyVerifier::new());

    match args.arm.as_str() {
        "DRY" => run_dry(&args, &verifier, &prepared),
        "A" | "B" => run_cf(&args, &verifier, &prepared),
        "C" => run_daemon(&args, &verifier, &prepared),
        other => {
            eprintln!("unknown --arm {other} (A|B|C|DRY)");
            std::process::exit(2);
        }
    }
    eprintln!("[regate] total wall: {:.1}s", t0.elapsed().as_secs_f32());
}

/// The offline validation arm: golden prompt snapshots + planted-fix scoring.
/// This validates the INSTRUMENT (prompts, merge, oracle, vetoes), not any
/// model — every number here is harness evidence, not model quality.
fn run_dry(args: &Args, verifier: &ClippyVerifier, prepared: &[Prepared]) {
    println!("═ arm DRY — offline harness validation (planted fixes; NOT model quality) ═");

    println!("\n── golden prompt snapshots (determinism; first 5) ──");
    for p in prepared.iter().take(5) {
        let raw = blake3::hash(p.prompt_raw.as_bytes()).to_hex();
        let pre = blake3::hash(p.prompt_prepass.as_bytes()).to_hex();
        println!(
            "  {:<28} miss={:<14} raw={:<12} prepass={:<12} modelless={}",
            p.sample.id,
            format!("{:?}", p.miss),
            &raw.to_string()[..12],
            &pre.to_string()[..12],
            if p.modelless.is_some() { "served" } else { "declined" },
        );
    }

    let off_corpus = prepared.iter().filter(|p| p.miss == MissClass::OffCorpus).count();
    let off_corpus_declined = prepared
        .iter()
        .filter(|p| p.miss == MissClass::OffCorpus)
        .filter(|p| p.modelless.is_none())
        .count();
    println!(
        "\n── decline baseline: {off_corpus_declined}/{off_corpus} off-corpus cases serve no \
         modelless fix (expect all — the Issue 030 gate)"
    );

    println!("\n── planted fixes (positive + negative controls) ──");
    let mut summary = ArmSummary::default();
    let mut all_ok = true;
    for (id, gen, expect_keep) in planted_fixes() {
        let Some(p) = prepared.iter().find(|p| p.sample.id == id) else {
            eprintln!("  planted id {id} not in sample set — skipping");
            continue;
        };
        let score = cf_workers_ai::score_case(verifier, &p.sample, gen, Some(&args.behav_dir));
        let ok = score.keep_strict == expect_keep;
        if !ok {
            all_ok = false;
        }
        println!(
            "  {:<28} keep_o={} keep_s={} (expect {}) {} [merge={:?} ctrl={} f {}→{} contract={:?} behav={:?}]",
            id,
            score.keep_oracle,
            score.keep_strict,
            expect_keep,
            if ok { "OK  " } else { "MISMATCH" },
            score.merge_strategy.map(|m| m.name()),
            score.control_fires,
            score.findings_before,
            if score.findings_after == usize::MAX { -1 } else { score.findings_after as isize },
            score.contract_ok,
            score.behavioural,
        );
        summary.add(&score, p.reachable_miss, 0, 0, 0.0, 0.0);
    }

    println!("\n── v2edit reference responses (self-consistency; details for first 8) ──");
    let v2 = prepared.iter().filter(|p| p.sample.source == SampleSource::V2edit);
    let (mut em_hits, mut em_total, mut merge_ok, mut control) = (0usize, 0, 0, 0);
    let mut kept = 0usize;
    let mut merge_strategies: Vec<&'static str> = Vec::new();
    for (idx, p) in v2.enumerate() {
        let Some(exp) = p.sample.expected.as_deref() else { continue };
        let score = cf_workers_ai::score_case(verifier, &p.sample, exp, None);
        em_total += 1;
        em_hits += usize::from(score.em == Some(true));
        merge_ok += usize::from(score.merge_strategy.is_some());
        if let Some(m) = score.merge_strategy {
            merge_strategies.push(m.name());
        }
        control += usize::from(score.control_fires);
        kept += usize::from(score.keep_oracle);
        if idx < 8 {
            println!(
                "  {:<18} lints={:<30} EM={:?} merge={:?} ctrl={} f {}→{} keep_o={}",
                p.sample.id,
                p.sample.lints.join("+"),
                score.em,
                score.merge_strategy.map(|m| m.name()),
                score.control_fires,
                score.findings_before,
                if score.findings_after == usize::MAX { -1 } else { score.findings_after as isize },
                score.keep_oracle,
            );
        }
    }
    merge_strategies.sort_unstable();
    println!(
        "  v2edit self-consistency (n={em_total}): EM {em_hits}/{em_total} (expect all — the \
         reference IS the expected text), merge {merge_ok}/{em_total} [{}] control {control}/{em_total}, \
         keep_o {kept}/{em_total}",
        merge_strategies.join(",")
    );

    println!("\n── instrument verdict ──");
    println!("  {}", summary.verdict_vs_decline());
    println!(
        "  planted controls: {}",
        if all_ok {
            "ALL PASS — instrument validated, arms pending (token / checkpoint)"
        } else {
            "MISMATCH — instrument is wrong, fix BEFORE any arm runs"
        }
    );
}

fn run_cf(args: &Args, verifier: &ClippyVerifier, prepared: &[Prepared]) {
    let mut client = match CfClient::from_env(args.model.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP arm {}: {e}", args.arm);
            std::process::exit(3);
        }
    };
    let prepass = args.arm == "B";
    let mut out = args.out.as_ref().and_then(|p| std::fs::File::create(p).ok());
    let mut summary = ArmSummary::default();
    let (mut lat_total, mut lat_max) = (0u128, 0u128);

    for p in prepared {
        let prompt = if prepass { &p.prompt_prepass } else { &p.prompt_raw };
        eprintln!("[{}] {} …", args.arm, p.sample.id);
        let gen = match client.generate(prompt, args.max_new) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("  GEN FAILED (recorded unscoreable): {e}");
                let score = cf_workers_ai::score_case(verifier, &p.sample, "", None);
                summary.add(&score, p.reachable_miss, 0, 0, 0.0, 0.0);
                record_case(&mut out, p, &args.arm, prompt, "", &GenStats::empty(), &score);
                continue;
            }
        };
        let score =
            cf_workers_ai::score_case(verifier, &p.sample, &gen.text, Some(&args.behav_dir));
        summary.add(
            &score,
            p.reachable_miss,
            gen.prompt_tokens,
            gen.completion_tokens,
            args.price_in,
            args.price_out,
        );
        lat_total += gen.latency_ms;
        lat_max = lat_max.max(gen.latency_ms);
        println!(
            "  {:<28} miss={:<13} keep_o={} keep_s={} EM={:?} f {}→{} {}ms {}+{}tok",
            p.sample.id,
            format!("{:?}", p.miss),
            score.keep_oracle,
            score.keep_strict,
            score.em,
            score.findings_before,
            if score.findings_after == usize::MAX { -1 } else { score.findings_after as isize },
            gen.latency_ms,
            gen.prompt_tokens,
            gen.completion_tokens,
        );
        record_case(
            &mut out,
            p,
            &args.arm,
            prompt,
            &gen.text,
            &GenStats {
                latency_ms: gen.latency_ms,
                prompt_tokens: gen.prompt_tokens,
                completion_tokens: gen.completion_tokens,
            },
            &score,
        );
    }

    println!("\n═ arm {} summary ═", args.arm);
    println!("| arm | n | EM | parses | merge | keep_o/scoreable | keep_s | vetoed | divergent | regressions | $ |");
    println!("{}", summary.markdown_row(&args.arm));
    println!(
        "  tokens: {} prompt + {} completion · $/fix ≈ {:.5}",
        summary.prompt_tokens,
        summary.completion_tokens,
        if summary.n > 0 { summary.cost_usd / summary.n as f64 } else { 0.0 },
    );
    println!("  latency: total {lat_total}ms, max {lat_max}ms");
    println!("  {}", summary.verdict_vs_decline());

    // ── Cost track (Proposal 005 §cost-tracking; Issue 040 T3) ────────────
    // Two meters: the client token-meter above (response.usage × list price)
    // and the server billing truth (GraphQL aiInferenceAdaptiveGroups →
    // Neurons). Both land in the committed ledger; KAT denomination happens
    // at Phase 1 (kat: null until the rail exists).
    let server = client.fetch_usage_today().ok();
    if let Some(u) = &server {
        let usd_neurons = u.neurons / 1000.0 * 0.011;
        println!(
            "  server meter (today, this model): {} req · {}+{} tok · {:.2} neurons ≈ ${:.5} (list $0.011/1k)",
            u.requests, u.input_tokens, u.output_tokens, u.neurons, usd_neurons
        );
        println!(
            "  free-allocation: {:.1}% of the 10k neurons/day (per-model view; account total ≥ this; resets 00:00 UTC)",
            u.neurons / NEURONS_FREE_DAILY * 100.0
        );
        // Third meter: client tokens × published neuron rates — should land
        // within a few % of the server neurons (validated 2026-08-26: 165.04
        // expected vs 165.037 server on the smoke run).
        let neurons_from_client = summary.prompt_tokens as f64 / 1e6 * DEFAULT_NEURONS_IN_PER_M
            + summary.completion_tokens as f64 / 1e6 * DEFAULT_NEURONS_OUT_PER_M;
        if neurons_from_client > 0.0 {
            println!(
                "  neuron cross-check: client tokens → {:.2} neurons vs server {:.2} ({:+.0}% — server counts template tokens)",
                neurons_from_client,
                u.neurons,
                (u.neurons / neurons_from_client - 1.0) * 100.0
            );
        }
        if summary.cost_usd > 0.0 && u.neurons > 0.0 {
            println!(
                "  reconciliation: client ${:.5} vs server ${:.5} ({:+.0}%)",
                summary.cost_usd,
                usd_neurons,
                (usd_neurons / summary.cost_usd - 1.0) * 100.0
            );
        }
    } else {
        println!("  server meter: unavailable (GraphQL fetch failed — client meter only)");
    }
    summary.latency_ms_total = lat_total;
    summary.latency_ms_max = lat_max;
    // Issue 046 T6: the USD daily-cap status (the gate lives INSIDE
    // `generate` — reserve-then-reconcile; this is the reporting half).
    println!("  {}", client.spend_summary());
    append_cost_ledger(
        &args.arm,
        client.model_id(),
        &summary,
        server.as_ref(),
        Some(client.spend_state()),
    );
    append_quality_history(&args.arm, client.model_id(), &summary);
}

/// Append one row to `.benchmarks/fixer_quality_history.jsonl` (Issue 044
/// T8 — keeps/vetoes per arm per session as a TIME SERIES, not prose in a
/// `.md`). The `score_history.jsonl` / `llm_cost_ledger.jsonl` precedent:
/// committed, append-only; the ops dashboard's fixer axis reads this file.
fn append_quality_history(arm: &str, model: &str, summary: &ArmSummary) {
    use std::io::Write;
    let path = std::env::var("RIIR_FIXER_QUALITY_HISTORY")
        .unwrap_or_else(|_| ".benchmarks/fixer_quality_history.jsonl".to_string());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let row = serde_json::json!({
        "ts_unix": ts,
        "arm": arm,
        "model": model,
        "n": summary.n,
        "em": {"hits": summary.em_hits, "total": summary.em_total},
        "fragment_parses": summary.fragment_parses,
        "merge_ok": summary.merge_ok,
        "control_fires": summary.control_fires,
        "oracle_scoreable": summary.oracle_scoreable,
        "strictly_fewer": summary.strictly_fewer,
        "keep_oracle": summary.keep_oracle,
        "keep_strict": summary.keep_strict,
        "vetoed": summary.vetoed,
        "divergent": summary.divergent,
        "regressions": summary.regressions,
        "reachable": {"n": summary.reachable_n, "keep_strict": summary.reachable_keep_strict},
        "tokens": {"prompt": summary.prompt_tokens, "completion": summary.completion_tokens},
        // u128 ms → u64 (clippy-safe; latencies are far below 2^52 anyway).
        "latency_ms": {
            "total": u64::try_from(summary.latency_ms_total).unwrap_or(u64::MAX),
            "max": u64::try_from(summary.latency_ms_max).unwrap_or(u64::MAX),
        },
    });
    let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[regate] quality-history write skipped ({path}: {e})");
            return;
        }
    };
    if let Err(e) = writeln!(f, "{row}") {
        eprintln!("[regate] quality-history write failed: {e}");
    }
}

/// Append one run row to `.benchmarks/llm_cost_ledger.jsonl` (the
/// score_history.jsonl precedent — committed, append-only).
///
/// KAT denomination is Phase 1 territory: rows carry `kat: null` until the
/// rail exists, then get back-filled at the mint rate of their run date.
fn append_cost_ledger(
    arm: &str,
    model: &str,
    summary: &ArmSummary,
    server: Option<&CfUsageToday>,
    spend: Option<&riir_clippy::llm_spend::SpendState>,
) {
    use std::io::Write;
    let path = std::env::var("RIIR_LLM_COST_LEDGER")
        .unwrap_or_else(|_| ".benchmarks/llm_cost_ledger.jsonl".to_string());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let row = serde_json::json!({
        "ts_unix": ts,
        "arm": arm,
        "model": model,
        "n": summary.n,
        "client": {
            "prompt_tokens": summary.prompt_tokens,
            "completion_tokens": summary.completion_tokens,
            "usd_est": summary.cost_usd,
        },
        "cap": spend.map(|s| serde_json::json!({
            "spent_usd": s.spent_usd,
            "cap_usd": s.cap_usd,
            "calls": s.calls,
            "declines": s.declines,
        })),
        "server": server.map(|u| serde_json::json!({
            "requests": u.requests,
            "input_tokens": u.input_tokens,
            "output_tokens": u.output_tokens,
            "neurons": u.neurons,
            "usd_list": u.neurons / 1000.0 * 0.011,
            "free_alloc_pct_of_daily_10k": u.neurons / 10_000.0 * 100.0,
        })),
        "kat": serde_json::Value::Null,
        "note": "kat null until Phase 1 rail; server = GraphQL aiInferenceAdaptiveGroups (billing truth)",
    });
    let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[regate] cost ledger write skipped ({path}: {e})");
            return;
        }
    };
    if let Err(e) = writeln!(f, "{row}") {
        eprintln!("[regate] cost ledger write failed: {e}");
    } else {
        println!("  cost ledger: {path} ← 1 row");
    }
}

fn run_daemon(args: &Args, verifier: &ClippyVerifier, prepared: &[Prepared]) {
    let (bin, gguf, lora) = if let (Some(b), Some(g), Some(l)) = (&args.l4_bin, &args.l4_gguf, &args.l4_lora) { (b.clone(), g.clone(), l.clone()) } else {
            eprintln!(
                "SKIP arm C: --l4-bin, --l4-gguf and --l4-lora are all required (the riir-train \
                 driver refuses to start without --lora)"
            );
            std::process::exit(3);
        };
    let mut daemon = match DaemonL4::spawn(&bin, &gguf, &lora, args.max_new) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("FATAL: daemon spawn failed: {e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "[regate] LOAD-DISCLOSURE: record sibling GPU/CPU load before quoting arm C — numbers \
         measured under compute contention are provisional (the GPU-exclusivity rule)"
    );
    let mut out = args.out.as_ref().and_then(|p| std::fs::File::create(p).ok());
    let mut summary = ArmSummary::default();
    let (mut lat_total, mut lat_max) = (0u128, 0u128);

    for p in prepared {
        eprintln!("[C] {} …", p.sample.id);
        let t = std::time::Instant::now();
        // Arm C uses the pre-pass prompt (the issue's arm table) via the
        // custom-prompt transport; the daemon reports no token counts.
        let Some(text) = daemon.request_prompt(&p.prompt_prepass) else {
            println!("  {:<28} DECLINE (empty frame)", p.sample.id);
            continue;
        };
        let latency = t.elapsed().as_millis();
        let score = cf_workers_ai::score_case(verifier, &p.sample, &text, Some(&args.behav_dir));
        summary.add(&score, p.reachable_miss, 0, 0, 0.0, 0.0);
        lat_total += latency;
        lat_max = lat_max.max(latency);
        println!(
            "  {:<28} keep_o={} keep_s={} f {}→{} {latency}ms",
            p.sample.id,
            score.keep_oracle,
            score.keep_strict,
            score.findings_before,
            if score.findings_after == usize::MAX { -1 } else { score.findings_after as isize },
        );
        record_case(&mut out, p, "C", &p.prompt_prepass, &text, &GenStats { latency_ms: latency, ..GenStats::empty() }, &score);
    }

    println!("\n═ arm C summary (daemon; tokens n/a) ═");
    println!("| arm | n | EM | parses | merge | keep_o/scoreable | keep_s | vetoed | divergent | regressions | $ |");
    println!("{}", summary.markdown_row("C"));
    println!("  latency: total {lat_total}ms, max {lat_max}ms");
    println!("  {}", summary.verdict_vs_decline());
    // Issue 044 T8: arm C lands in the same quality time series (tokens n/a
    // for the local daemon — the keep/veto axes are the point).
    summary.latency_ms_total = lat_total;
    summary.latency_ms_max = lat_max;
    append_quality_history("C", "local-bonsai-daemon", &summary);
}
