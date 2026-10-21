//! Issue 040 — the fixer re-gate POC runner.
//!
//! Arms:
//! - `--arm A` — CF qwen3.8-27b raw prompt (needs `CLOUDFLARE_OPS_API_TOKEN` +
//!   `CLOUDFLARE_ACCOUNT_ID`)
//! - `--arm B` — same model, modelless pre-pass prompt (candidate + corpus
//!   few-shots)
//! - `--arm G` — Issue 049 growing-input prompt: arm B + the two Meta^n
//!   conditioning channels from the durable trajectory store (prior
//!   outcomes for shape-similar inputs + the per-rule failure-pattern
//!   summary). Arm A/B prompts are untouched — reproducible baselines.
//! - `--arm G2` — Issue 067 arm G' (the Research-530 reopen): the SAME two
//!   channels restricted to the QUERIED lint's family (the relevance fix —
//!   cross-rule exemplars are the paper's Random-Trace harm class) and
//!   placed BEFORE the code module (condition-first — the placement arm
//!   that dominated 26/27 cells). Empty same-rule channels degrade to arm
//!   B byte-identically; `growing_prime_channels_empty` records the
//!   degradation rate per row.
//! - `--arm C` — local Bonsai via the `bonsai_clippy_l4_eval --serve` daemon
//!   (`--l4-bin/--l4-gguf/--l4-lora`; note the riir-train driver REQUIRES
//!   `--lora`, so arm C needs a checkpoint — see the issue Status section)
//! - `--arm BT8DRY` — Issue 073 T3 offline validation: builds the 12
//!   perturbation pairs (frozen score-bench fixtures, T2 twin construction),
//!   composes both prompts, and plants known generations through the grading
//!   path. Instrument evidence only — no model, no network.
//! - `--arm BT8` — the same 12 pairs through the CF backend, one call per
//!   pair per arm: arm A = `prompt_growing` (prose baseline, channels empty
//!   for synthetic twins) vs arm Bt8 = `prompt_anchor_ledger` (prose + the
//!   discrete identity ledger). Graded by the PRE-REGISTERED bar in
//!   `.issues/073` (swapped-symbol following ≥10/12, delta ≥3, strict-keep
//!   no-regression), recorded in the bench doc.
//! - `--arm DRY` (default) — offline harness validation: golden prompt
//!   snapshots (blake3) + scoring of PLANTED fixes (positive + negative
//!   controls). No model, no network. This is the "arms pending" evidence
//!   mode.
//! - `--arm H` — Plan 202 R6: the ESCALATION chain over the same cases —
//!   the modelless answer (the pipeline pick) vs the armed manifest row's
//!   transport (seat rerank over the L0–L3 pool / hosted generate on a
//!   miss), paired per-case with the cost axes (wall/receipt ms; the
//!   llm_spend summary for the hosted leg). Arming is a manifest row
//!   (`--esc-manifest`, default `.refine/escalation_manifest.json`) under
//!   `--esc-domain` (default clippy); consent is the shared
//!   `RIIR_REFINE_CODE_EGRESS=snippet` scope; `--deadline-ms` bounds the
//!   whole readiness-retry + wire budget per case. Requires features
//!   `escalation_manifest` + the row's transport (`seat_rerank` and/or
//!   `hosted_expert`). Demotion of the loser is an operator manifest edit
//!   — the arm prints the verdict, never edits data.
//!   Issue 156: each paired case also records the FlyBy state tallies
//!   (`rescue_mine` — b0 = the modelless continuation's outcome, bd = the
//!   escalated continuation's outcome when the transport actually served)
//!   into the durable trajectory store, and emits the rescue exemplar set
//!   (`b0=0 ∧ bd≥3`) with its BLAKE3 keep-set hash. AUGMENT-ONLY: the
//!   tallies are a pre-filter; the agreement veto stays the authority.
//!
//! Run:
//!
//! ```sh
//! # offline validation (always runnable)
//! cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm DRY
//!
//! # arms A+B (once CLOUDFLARE_OPS_API_TOKEN + CLOUDFLARE_ACCOUNT_ID are exported)
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

use riir_refine::cf_workers_ai::{
    self, reachable_samples, ArmSummary, CaseScore, CfClient, CfUsageToday, FewShot, MissClass,
    RegateHarness, Sample, SampleSource,
};
use riir_refine::kat_footer::TOKENS_PER_MTOK;
use riir_refine::kat_redact::{GateDecision, RedactionGate, RedactionLog, SpanOrigin};
use riir_refine::l4_daemon::DaemonL4;
use riir_refine::ClippyVerifier;

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
    /// Per-call full prompt + generation dump (audit evidence; NOT committed
    /// by default — the rows file stays the compact record).
    gens: Option<PathBuf>,
    policy: Option<PathBuf>,
    model: Option<String>,
    l4_bin: Option<String>,
    l4_gguf: Option<String>,
    l4_lora: Option<String>,
    price_in: f64,
    price_out: f64,
    behav_dir: PathBuf,
    /// Plan 202 R6 arm H: the escalation manifest path (arming is a file
    /// edit, never code — the R1 law).
    esc_manifest: PathBuf,
    /// The domain label arm H reads its armed row under (the harness fixes
    /// clippy lints — "clippy" is the runbook default).
    esc_domain: String,
    /// The per-case escalation deadline in ms (the whole readiness-retry
    /// + wire budget; Bench 107: the lazy window ran ~100 ms under it).
    deadline_ms: u64,
    /// Plan 225 T1.1 (`--arm HARVEST`): cap on harvested cases (0 = the
    /// full deduped v2edit population). Separate from `--limit` (whose
    /// default 16 is the LLM arms' per-chunk semantics).
    harvest_cap: usize,
    /// Plan 225 T1.4's behavioural PILOT budget: the first N pool≥2 cases
    /// with ≥1 (a)∧(c)-gold candidate get the per-fn differential driver,
    /// estimating P(behaviour-ok | a∧c-ok) for the T1.6 supply projection.
    harvest_behavioural: usize,
    /// Plan 225 T2/T4: the frozen Bench-114 set (`cases.jsonl`).
    frozen_cases: Option<PathBuf>,
    /// Plan 225 T4: the replayed pool texts (`pool_texts.jsonl`, the
    /// `--arm POOLTEXTS` output — candidate texts blake3-pinned to the
    /// frozen rows).
    frozen_pools: Option<PathBuf>,
    /// Plan 225 T4: the frozen lint split (`split.json`).
    frozen_split: Option<PathBuf>,
    /// Plan 225 T4.2's A0 cost leg: verify-in-order wall on the first N
    /// scored test rows (0 = off).
    a0_cost_sample: usize,
}

impl Args {
    fn parse() -> Self {
        let mut a = Self {
            arm: "DRY".into(),
            source: "both".into(),
            chunks: vec![0],
            limit: 16,
            few_shots: 3,
            max_new: 128,
            data_dir: None,
            out: None,
            gens: None,
            policy: None,
            model: None,
            l4_bin: None,
            l4_gguf: None,
            l4_lora: None,
            price_in: DEFAULT_PRICE_IN_PER_M,
            price_out: DEFAULT_PRICE_OUT_PER_M,
            behav_dir: std::env::temp_dir().join("issue040_behav"),
            esc_manifest: PathBuf::from(".refine/escalation_manifest.json"),
            esc_domain: "clippy".into(),
            deadline_ms: 2_000,
            harvest_cap: 0,
            harvest_behavioural: 0,
            frozen_cases: None,
            frozen_pools: None,
            frozen_split: None,
            a0_cost_sample: 0,
        };
        let mut argv = std::env::args().skip(1);
        while let Some(arg) = argv.next() {
            let mut val = || {
                argv.next()
                    .unwrap_or_else(|| panic!("missing value for {arg}"))
            };
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
                "--few-shots" => {
                    a.few_shots = val().parse().expect("--few-shots must be an integer")
                }
                "--max-new" => a.max_new = val().parse().expect("--max-new must be an integer"),
                "--data-dir" => a.data_dir = Some(PathBuf::from(val())),
                "--out" => a.out = Some(PathBuf::from(val())),
                "--gens" => a.gens = Some(PathBuf::from(val())),
                "--policy" => a.policy = Some(PathBuf::from(val())),
                "--model" => a.model = Some(val()),
                "--l4-bin" => a.l4_bin = Some(val()),
                "--l4-gguf" => a.l4_gguf = Some(val()),
                "--l4-lora" => a.l4_lora = Some(val()),
                "--price-in" => a.price_in = val().parse().expect("--price-in must be a float"),
                "--price-out" => a.price_out = val().parse().expect("--price-out must be a float"),
                "--behav-dir" => a.behav_dir = PathBuf::from(val()),
                "--esc-manifest" => a.esc_manifest = PathBuf::from(val()),
                "--esc-domain" => a.esc_domain = val(),
                "--deadline-ms" => {
                    a.deadline_ms = val().parse().expect("--deadline-ms must be an integer")
                }
                "--harvest-cap" => {
                    a.harvest_cap = val().parse().expect("--harvest-cap must be an integer")
                }
                "--harvest-behavioural" => {
                    a.harvest_behavioural = val()
                        .parse()
                        .expect("--harvest-behavioural must be an integer")
                }
                "--frozen-cases" => a.frozen_cases = Some(PathBuf::from(val())),
                "--frozen-pools" => a.frozen_pools = Some(PathBuf::from(val())),
                "--frozen-split" => a.frozen_split = Some(PathBuf::from(val())),
                "--a0-cost-sample" => {
                    a.a0_cost_sample = val().parse().expect("--a0-cost-sample must be an integer")
                }
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
    // Plan 225 T1.1: the FULL deduped v2edit population (labelled chunks +
    // unlabelled sft rows; the HARVEST arm labels them by discovery).
    if args.source == "v2edit_all" {
        let dir = resolve_data_dir(&args.data_dir)?;
        samples.extend(cf_workers_ai::load_v2edit_full(&dir, args.harvest_cap)?);
    }
    if samples.is_empty() {
        return Err(format!("--source {} produced zero samples", args.source));
    }
    Ok(samples)
}

/// T0.2 (Plan 026): run every sample through the redaction gate BEFORE any
/// arm sees it. Arms A/B send prompts to CF Workers AI (real egress) and
/// every arm writes corpus rows via `record_case` — a blocked span must
/// reach neither. Deny-by-default: no discoverable `.refine-sync-policy`
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
            riir_refine::kat_redact::REDACT_VERSION,
            loaded.gate.policy().allow_origins
        ),
        (Some(_), Some(w)) => eprintln!("[regate] redaction policy: {w} — DENY-ALL in force"),
        (None, None) => eprintln!(
            "[regate] redaction policy: NONE FOUND — deny-by-default blocks every span; \
             create .refine-sync-policy at the repo root (Proposal 005 T0.2)"
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
                eprintln!(
                    "[regate] REDACTED {} (findings replaced; policy allows redacted)",
                    s.id
                );
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
    eprintln!(
        "[regate] redaction gate: {} allowed, {blocked} blocked",
        kept.len()
    );
    if kept.is_empty() {
        eprintln!("FATAL: redaction gate blocked every sample — nothing to run");
        std::process::exit(4);
    }
    kept
}

/// One prepared case: sample + miss class + every prompt shape.
struct Prepared {
    sample: Sample,
    miss: MissClass,
    modelless: Option<String>,
    #[allow(dead_code)] // kept for the JSONL row's diagnostics richness
    few_shots: Vec<FewShot>,
    prompt_raw: String,
    prompt_prepass: String,
    /// Issue 049 arm G: the growing-input prompt (arm B + channels 1/3).
    prompt_growing: String,
    /// Whether both growing channels were empty for this sample (store
    /// absent or no shape-similar rows) — arm G degenerates to arm B there;
    /// recorded per-row so the bench can report the degradation rate.
    growing_channels_empty: bool,
    /// Issue 067 arm G2 (G'): the same-rule, condition-first prompt — the
    /// two channels restricted to the queried lint's family and placed
    /// before the code module.
    prompt_growing_prime: String,
    /// Whether both SAME-RULE channels were empty for this sample — arm G2
    /// degenerates to arm B byte-identically there; recorded per-row.
    growing_prime_channels_empty: bool,
    reachable_miss: bool,
    /// Plan 202 R6 arm H: the modelless confidence + the L0–L3 candidate
    /// pool (the escalation chain's input). Read only by the arm-H runner;
    /// the allow keeps the other postures' clippy clean.
    #[allow(dead_code)]
    score: f32,
    #[allow(dead_code)]
    pool: Vec<String>,
    /// Arm H's modelless-arm latency: the classify (the serve path's
    /// pipeline work) measured at prepare — the escalation chain in
    /// production reuses THIS result, so this is the honest µs axis.
    #[allow(dead_code)]
    modelless_us: u128,
}

fn prepare(harness: &mut RegateHarness, samples: Vec<Sample>, few_shots: usize) -> Vec<Prepared> {
    samples
        .into_iter()
        .map(|sample| {
            let t_classify = std::time::Instant::now();
            let outcome = harness.classify_outcome(&sample.lints[0], &sample.module);
            let modelless_us = t_classify.elapsed().as_micros();
            let (miss, modelless) = (outcome.miss, outcome.served);
            let fs = harness.few_shots(&sample.module, few_shots);
            let raw = cf_workers_ai::prompt_raw(&sample.module);
            let prepass = cf_workers_ai::prompt_prepass(&sample.module, modelless.as_deref(), &fs);
            let (traj, patterns) = harness.trajectory_context(&sample.module, 3);
            let growing_empty = traj.is_empty() && patterns.is_empty();
            let growing = cf_workers_ai::prompt_growing(
                &sample.module,
                modelless.as_deref(),
                &fs,
                &traj,
                &patterns,
            );
            let reachable_miss = miss.is_reachable_miss();
            // Issue 067 arm G2: same-rule channels, condition-first. The
            // queried lint is the sample's primary target (the same one
            // `classify` consumed above).
            let (traj_sr, patterns_sr) =
                harness.trajectory_context_same_rule(&sample.module, &sample.lints[0], 3);
            let growing_prime_empty = traj_sr.is_empty() && patterns_sr.is_empty();
            let growing_prime = cf_workers_ai::prompt_growing_same_rule(
                &sample.module,
                modelless.as_deref(),
                &fs,
                &traj_sr,
                &patterns_sr,
            );
            Prepared {
                sample,
                miss,
                modelless,
                few_shots: fs,
                prompt_raw: raw,
                prompt_prepass: prepass,
                prompt_growing: growing,
                growing_channels_empty: growing_empty,
                prompt_growing_prime: growing_prime,
                growing_prime_channels_empty: growing_prime_empty,
                reachable_miss,
                score: outcome.score,
                pool: outcome.pool,
                modelless_us,
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
        Self {
            latency_ms: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
        }
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
            "growing_channels_empty": p.growing_channels_empty,
            "growing_prime_channels_empty": p.growing_prime_channels_empty,
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

    // Issue 073 T3: the Bt8 arms build their own pair set from the frozen
    // score-bench fixtures — the reachable/v2edit sample path does not apply.
    if matches!(args.arm.as_str(), "BT8" | "BT8DRY") {
        bt8_dispatch(&args);
        eprintln!("[regate] total wall: {:.1}s", t0.elapsed().as_secs_f32());
        return;
    }

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

    // Plan 225 T1.1 — the harvest arm runs its OWN replay loop (classify →
    // score → warm memory, interleaved per case so pools warm exactly as
    // production serves them). It must NOT go through `prepare` (no LLM
    // prompts, no batch classify — the pool capture happens per case).
    if args.arm == "HARVEST" {
        run_harvest(&args, &mut harness, samples);
        eprintln!("[regate] total wall: {:.1}s", t0.elapsed().as_secs_f32());
        return;
    }

    // Plan 225 T2 — the pool-text exporter: the frozen cases.jsonl carries
    // gate flags + per-candidate blake3_12 but NOT candidate texts; this
    // replay regenerates them (classify → verify blake3 → warm memory in
    // the harvest's exact sequence) and emits pool_texts.jsonl for the
    // trainer and the frozen read. Branches before `prepare` like HARVEST
    // (no LLM prompts, no batch classify).
    if args.arm == "POOLTEXTS" {
        run_pooltexts(&args, &mut harness);
        eprintln!("[regate] total wall: {:.1}s", t0.elapsed().as_secs_f32());
        return;
    }
    // Plan 225 T4 — the frozen read: ONE paired pass of the frozen scored
    // population (test lints only) through the LIVE seat, A0 derived from
    // the frozen gate fields, verdict per T4.3. Also branches before
    // `prepare` (the prepared samples are not the frozen population).
    if args.arm == "FROZEN" {
        let verifier = std::mem::replace(&mut harness.verifier, ClippyVerifier::new());
        run_frozen(&args, &verifier);
        eprintln!("[regate] total wall: {:.1}s", t0.elapsed().as_secs_f32());
        return;
    }

    let prepared = prepare(&mut harness, samples, args.few_shots);
    std::fs::create_dir_all(&args.behav_dir).ok();
    // The harness is consumed by `prepare`; arms get their own verifier
    // (stateless per call — same instrument).
    let verifier = std::mem::replace(&mut harness.verifier, ClippyVerifier::new());

    match args.arm.as_str() {
        "DRY" => run_dry(&args, &verifier, &prepared),
        "A" | "B" | "G" | "G2" => run_cf(&args, &verifier, &prepared),
        "C" => run_daemon(&args, &verifier, &prepared),
        "H" => hosted_dispatch(&args, &verifier, &prepared, harness.store.as_ref()),
        other => {
            eprintln!(
                "unknown --arm {other} (A|B|G|G2|C|H|DRY|HARVEST|POOLTEXTS|FROZEN|BT8|BT8DRY)"
            );
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
        let grow = blake3::hash(p.prompt_growing.as_bytes()).to_hex();
        let grow2 = blake3::hash(p.prompt_growing_prime.as_bytes()).to_hex();
        println!(
            "  {:<28} miss={:<14} raw={:<12} prepass={:<12} growing={:<12} growing2={:<12} channels={} channels2={} modelless={}",
            p.sample.id,
            format!("{:?}", p.miss),
            &raw.to_string()[..12],
            &pre.to_string()[..12],
            &grow.to_string()[..12],
            &grow2.to_string()[..12],
            if p.growing_channels_empty { "empty" } else { "live" },
            if p.growing_prime_channels_empty { "empty" } else { "live" },
            if p.modelless.is_some() { "served" } else { "declined" },
        );
    }

    let off_corpus = prepared
        .iter()
        .filter(|p| p.miss == MissClass::OffCorpus)
        .count();
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
    let v2 = prepared
        .iter()
        .filter(|p| p.sample.source == SampleSource::V2edit);
    let (mut em_hits, mut em_total, mut merge_ok, mut control) = (0usize, 0, 0, 0);
    let mut kept = 0usize;
    let mut merge_strategies: Vec<&'static str> = Vec::new();
    for (idx, p) in v2.enumerate() {
        let Some(exp) = p.sample.expected.as_deref() else {
            continue;
        };
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
                if score.findings_after == usize::MAX {
                    -1
                } else {
                    score.findings_after as isize
                },
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
    let mut out = args
        .out
        .as_ref()
        .and_then(|p| std::fs::File::create(p).ok());
    let mut summary = ArmSummary::default();
    let (mut lat_total, mut lat_max) = (0u128, 0u128);

    for p in prepared {
        let prompt = match args.arm.as_str() {
            "G" => &p.prompt_growing,
            "G2" => &p.prompt_growing_prime,
            "B" => &p.prompt_prepass,
            _ => &p.prompt_raw,
        };
        eprintln!("[{}] {} …", args.arm, p.sample.id);
        let gen = match client.generate(prompt, args.max_new) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("  GEN FAILED (recorded unscoreable): {e}");
                let score = cf_workers_ai::score_case(verifier, &p.sample, "", None);
                summary.add(&score, p.reachable_miss, 0, 0, 0.0, 0.0);
                record_case(
                    &mut out,
                    p,
                    &args.arm,
                    prompt,
                    "",
                    &GenStats::empty(),
                    &score,
                );
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
            if score.findings_after == usize::MAX {
                -1
            } else {
                score.findings_after as isize
            },
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
        if summary.n > 0 {
            summary.cost_usd / summary.n as f64
        } else {
            0.0
        },
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
        // expected vs 165.037 server on the smoke run). TOKENS_PER_MTOK is
        // the TOKEN million (per-Mtok rates) — never MICRO_PER_KAT
        // (riir-dapps Plan 035 T10).
        let neurons_from_client = summary.prompt_tokens as f64 / TOKENS_PER_MTOK as f64
            * DEFAULT_NEURONS_IN_PER_M
            + summary.completion_tokens as f64 / TOKENS_PER_MTOK as f64 * DEFAULT_NEURONS_OUT_PER_M;
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
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
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
    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
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
    spend: Option<&riir_refine::llm_spend::SpendState>,
) {
    use std::io::Write;
    let path = std::env::var("RIIR_LLM_COST_LEDGER")
        .unwrap_or_else(|_| ".benchmarks/llm_cost_ledger.jsonl".to_string());
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
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
    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
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
    let (bin, gguf, lora) =
        if let (Some(b), Some(g), Some(l)) = (&args.l4_bin, &args.l4_gguf, &args.l4_lora) {
            (b.clone(), g.clone(), l.clone())
        } else {
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
    let mut out = args
        .out
        .as_ref()
        .and_then(|p| std::fs::File::create(p).ok());
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
            if score.findings_after == usize::MAX {
                -1
            } else {
                score.findings_after as isize
            },
        );
        record_case(
            &mut out,
            p,
            "C",
            &p.prompt_prepass,
            &text,
            &GenStats {
                latency_ms: latency,
                ..GenStats::empty()
            },
            &score,
        );
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

// ── Issue 073 T3 — the Bt8 anchor-ledger A/B (entity-following twins) ─────

/// Entry for the BT8 arms; gated on the frozen fixture set (score_bench).
fn bt8_dispatch(args: &Args) {
    #[cfg(feature = "score_bench")]
    {
        let mut harness = match RegateHarness::new() {
            Ok(h) => h,
            Err(e) => {
                eprintln!("FATAL: harness init failed: {e}");
                std::process::exit(1);
            }
        };
        let verifier = std::mem::replace(&mut harness.verifier, ClippyVerifier::new());
        std::fs::create_dir_all(&args.behav_dir).ok();
        run_bt8(args, &verifier, &mut harness);
    }
    #[cfg(not(feature = "score_bench"))]
    {
        let _ = args;
        eprintln!("--arm BT8/BT8DRY requires --features score_bench (the frozen fixture set)");
        std::process::exit(2);
    }
}

/// The same-kind few-shot rule-label swap (the T2 landed pair). Absence ⇒
/// the label crosses unchanged and the row records the unflipped axis.
#[cfg(feature = "score_bench")]
fn bt8_same_kind_rule(rule: &str) -> Option<&'static str> {
    match rule {
        "manual_map" => Some("manual_unwrap_or"),
        "manual_unwrap_or" => Some("manual_map"),
        _ => None,
    }
}

/// First `fn` name in the module + its byte offset (the twin target).
/// Word-boundary guarded so a longer identifier ending in "fn" never matches.
#[cfg(feature = "score_bench")]
fn bt8_first_fn(code: &str) -> Option<(&str, usize)> {
    let mut from = 0usize;
    while let Some(rel) = code[from..].find("fn ") {
        let at = from + rel;
        let bounded = at == 0 || !code[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let name_at = at + 3;
        let rest = &code[name_at..];
        let end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        if bounded && end > 0 {
            return Some((&rest[..end], name_at));
        }
        from = at + 3;
    }
    None
}

#[cfg(feature = "score_bench")]
struct Bt8Pair {
    fixture_id: &'static str,
    clean: bool,
    lint: Option<String>,
    original: String,
    renamed: String,
    twin: String,
    prompt_a: String,
    prompt_b: String,
    sample: Option<Sample>,
    fewshot_swapped: usize,
    fewshot_total: usize,
    channels_empty: bool,
    miss: MissClass,
    modelless_served: bool,
}

/// Build the pre-registered pair set: the frozen fixtures in id order, the
/// window ending at the set tail so it CONTAINS the T2 anchor. The
/// pre-registration's arithmetic ("the T2 fixture plus the next 11") is
/// unsatisfiable forward — the frozen set has only 7 successors after the
/// anchor — so the window is the LAST 12 (anchor + all 7 successors + the 4
/// id-adjacent predecessors). Recorded in the bench doc BEFORE the run.
#[cfg(feature = "score_bench")]
fn bt8_build_pairs(harness: &mut RegateHarness, few_shots: usize) -> Vec<Bt8Pair> {
    use riir_refine::cf_workers_ai::{ident_mentioned, FixAnchors};

    const ANCHOR: &str = "katgpt-rs-bench_ptrm_width_scaling-002";
    let mut fixtures = riir_refine::score_bench::fixtures::fixtures();
    assert!(
        fixtures.len() >= 12,
        "frozen set smaller than the pre-registered pair count"
    );
    fixtures.sort_by(|a, b| a.id.cmp(b.id));
    let pos = fixtures
        .iter()
        .position(|f| f.id == ANCHOR)
        .expect("pre-registration anchor fixture missing from the frozen set");
    let start = if fixtures.len() - pos >= 12 {
        pos
    } else {
        fixtures.len() - 12
    };
    let selected: Vec<_> = fixtures[start..start + 12].to_vec();
    assert_eq!(selected.len(), 12, "the pre-registered pair count");
    assert!(
        selected.iter().any(|f| f.id == ANCHOR),
        "anchor in the window"
    );
    eprintln!(
        "[bt8] window {start}..{} of {} fixtures (anchor at {pos}) — 12 pairs, named BEFORE the run",
        start + 12,
        fixtures.len()
    );

    let mut pairs = Vec::with_capacity(selected.len());
    for f in selected {
        let (name, _at) =
            bt8_first_fn(f.code).unwrap_or_else(|| panic!("fixture {} has no fn to twin", f.id));
        let mut renamed = format!("{name}_alt");
        if ident_mentioned(f.code, &renamed) {
            renamed = format!("{name}_v2");
        }
        let twin = cf_workers_ai::perturb_symbol_ident(f.code, name, &renamed);
        if ident_mentioned(&twin, name) {
            eprintln!("[bt8] SKIP {}: rename not total (instrument hazard)", f.id);
            continue;
        }
        let fs = harness.few_shots(&twin, few_shots);
        let fewshot_swapped = fs
            .iter()
            .filter(|s| bt8_same_kind_rule(&s.rule_id).is_some())
            .count();
        let fs_twin: Vec<FewShot> = fs
            .iter()
            .map(|s| FewShot {
                rule_id: bt8_same_kind_rule(&s.rule_id)
                    .unwrap_or(&s.rule_id)
                    .to_string(),
                buggy: s.buggy.clone(),
                fixed: s.fixed.clone(),
            })
            .collect();
        let lint = f.expect_lints.first().map(|s| s.to_string());
        let (miss, modelless) = harness.classify(lint.as_deref().unwrap_or(""), &twin);
        let (traj, patterns) = harness.trajectory_context(&twin, 3);
        let channels_empty = traj.is_empty() && patterns.is_empty();
        let prompt_a =
            cf_workers_ai::prompt_growing(&twin, modelless.as_deref(), &fs_twin, &traj, &patterns);
        // The ledger restates only TRUE identity fields: the real target lint
        // (None on clean fixtures — never a lie), the renamed symbol, and the
        // symbol's verbatim byte span in the twin.
        let sym_at = twin.find(&renamed).expect("renamed symbol present in twin");
        let anchors = FixAnchors {
            rule_id: lint.clone(),
            error_code: None,
            symbols: vec![renamed.clone()],
            span: Some(format!("bytes {sym_at}..{}", sym_at + renamed.len())),
        };
        let prompt_b =
            cf_workers_ai::prompt_anchor_ledger(&twin, modelless.as_deref(), &fs_twin, &anchors);
        // Clean fixtures have no target lint: the oracle has nothing to fire
        // on, so score_case is skipped for them (recorded; both arms neutral
        // on the keep axes). The symbol-following axis still grades.
        let sample = lint.as_ref().map(|lint| Sample {
            id: format!("bt8:{}", f.id),
            lints: vec![lint.clone()],
            module: twin.clone(),
            expected: None,
            source: if f.id.starts_with("composite-") {
                SampleSource::V2edit
            } else {
                SampleSource::Reachable
            },
        });
        pairs.push(Bt8Pair {
            fixture_id: f.id,
            clean: f.clean,
            lint,
            original: name.to_string(),
            renamed,
            twin,
            prompt_a,
            prompt_b,
            sample,
            fewshot_swapped,
            fewshot_total: fs.len(),
            channels_empty,
            miss,
            modelless_served: modelless.is_some(),
        });
    }
    pairs
}

/// One graded generation. `follows` = the healed text references the SWAPPED
/// symbol (the pre-registered property); `prior` = it references the ORIGINAL
/// symbol instead (the identity-regression signal; boundary-disjoint from
/// `follows` by construction).
#[cfg(feature = "score_bench")]
struct Bt8Call {
    follows: bool,
    prior: bool,
    keep_strict: Option<bool>,
    latency_ms: u128,
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[cfg(feature = "score_bench")]
fn bt8_grade(
    verifier: &ClippyVerifier,
    p: &Bt8Pair,
    gen: &str,
    behav: &std::path::Path,
) -> Bt8Call {
    use riir_refine::cf_workers_ai::ident_mentioned;

    let text = cf_workers_ai::extract_code_block(gen);
    let call = Bt8Call {
        follows: ident_mentioned(text, &p.renamed),
        prior: ident_mentioned(text, &p.original),
        keep_strict: p
            .sample
            .as_ref()
            .map(|s| cf_workers_ai::score_case(verifier, s, gen, Some(behav)).keep_strict),
        latency_ms: 0,
        prompt_tokens: 0,
        completion_tokens: 0,
    };
    call
}

#[cfg(feature = "score_bench")]
fn bt8_row(
    out: &mut Option<std::fs::File>,
    p: &Bt8Pair,
    arm: &str,
    c: &Bt8Call,
    prompt: &str,
    gen: &str,
) {
    if let Some(f) = out.as_mut() {
        let row = serde_json::json!({
            "pair": p.fixture_id,
            "arm": arm,
            "clean": p.clean,
            "lint": p.lint,
            "renamed": p.renamed,
            "follows_swap": c.follows,
            "prior_symbol": c.prior,
            "keep_strict": c.keep_strict,
            "latency_ms": c.latency_ms,
            "prompt_tokens": c.prompt_tokens,
            "completion_tokens": c.completion_tokens,
            "fewshot_swapped": format!("{}/{}", p.fewshot_swapped, p.fewshot_total),
            "channels_empty": p.channels_empty,
            "miss_class": format!("{:?}", p.miss),
            "modelless_served": p.modelless_served,
            "prompt_blake3_12": &blake3::hash(prompt.as_bytes()).to_hex().as_str()[..12],
            "gen_chars": gen.len(),
        });
        let _ = writeln!(f, "{row}");
    }
}

#[cfg(feature = "score_bench")]
fn run_bt8(args: &Args, verifier: &ClippyVerifier, harness: &mut RegateHarness) {
    let pairs = bt8_build_pairs(harness, args.few_shots);
    eprintln!("[bt8] {} pairs built", pairs.len());

    if args.arm == "BT8DRY" {
        // Instrument evidence only: planted generations through the grading
        // path. A twin that FOLLOWS keeps the renamed symbol; a twin that
        // REGRESSES to the prior flips exactly the entity back.
        let mut all_pinned = true;
        for p in &pairs {
            let planted_follow = p.twin.clone();
            let planted_regress =
                cf_workers_ai::perturb_symbol_ident(&p.twin, &p.renamed, &p.original);
            let f = bt8_grade(verifier, p, &planted_follow, &args.behav_dir);
            let r = bt8_grade(verifier, p, &planted_regress, &args.behav_dir);
            let ok = f.follows && !f.prior && !r.follows && r.prior;
            all_pinned &= ok;
            println!(
                "  {:<44} rename={} degrade_prompt={} fs_swap={}/{} planted_follow={} planted_regress={} {}",
                p.fixture_id,
                p.renamed,
                p.channels_empty,
                p.fewshot_swapped,
                p.fewshot_total,
                f.follows,
                r.prior,
                if ok { "PINNED" } else { "BROKEN" },
            );
        }
        println!(
            "\n═ BT8DRY: {} pairs, planted instrument {} ═",
            pairs.len(),
            if all_pinned {
                "PINNED"
            } else {
                "BROKEN — do not run BT8"
            }
        );
        if !all_pinned {
            std::process::exit(5);
        }
        return;
    }

    let mut client = match CfClient::from_env(args.model.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP arm BT8: {e}");
            std::process::exit(3);
        }
    };
    let mut out = args
        .out
        .as_ref()
        .and_then(|p| std::fs::File::create(p).ok());
    let mut gens = args
        .gens
        .as_ref()
        .and_then(|p| std::fs::File::create(p).ok());
    let mut per_pair: Vec<(bool, bool, Option<bool>, Option<bool>)> = Vec::new();

    for p in &pairs {
        eprintln!("[bt8] {} (rename → {}) …", p.fixture_id, p.renamed);
        let mut calls = Vec::with_capacity(2);
        for (arm, prompt) in [("A", &p.prompt_a), ("Bt8", &p.prompt_b)] {
            let t = std::time::Instant::now();
            let gen = match client.generate(prompt, args.max_new) {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("  GEN FAILED {arm} (recorded unscoreable): {e}");
                    let c = Bt8Call {
                        follows: false,
                        prior: false,
                        keep_strict: None,
                        latency_ms: t.elapsed().as_millis(),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                    };
                    bt8_row(&mut out, p, arm, &c, prompt, "");
                    calls.push(c);
                    continue;
                }
            };
            let mut c = bt8_grade(verifier, p, &gen.text, &args.behav_dir);
            c.latency_ms = t.elapsed().as_millis();
            c.prompt_tokens = gen.prompt_tokens;
            c.completion_tokens = gen.completion_tokens;
            if let Some(gf) = gens.as_mut() {
                let _ = writeln!(
                    gf,
                    "{}",
                    serde_json::json!({
                        "pair": p.fixture_id,
                        "arm": arm,
                        "renamed": p.renamed,
                        "gen": gen.text,
                        "truncated_hint": gen.completion_tokens >= args.max_new as u64,
                    })
                );
            }
            println!(
                "  {:<14} follows={} prior={} keep_s={:?} {}ms {}+{}tok",
                arm,
                c.follows,
                c.prior,
                c.keep_strict,
                c.latency_ms,
                c.prompt_tokens,
                c.completion_tokens,
            );
            bt8_row(&mut out, p, arm, &c, prompt, &gen.text);
            calls.push(c);
        }
        per_pair.push((
            calls[0].follows,
            calls[1].follows,
            calls[0].keep_strict,
            calls[1].keep_strict,
        ));
    }

    let follow_a = per_pair.iter().filter(|(a, ..)| *a).count();
    let follow_b = per_pair.iter().filter(|(_, b, ..)| *b).count();
    let strict_a = per_pair
        .iter()
        .filter(|(.., sa, _)| sa == &Some(true))
        .count();
    let strict_b = per_pair.iter().filter(|(.., sb)| sb == &Some(true)).count();
    let delta = follow_b as isize - follow_a as isize;
    let n = per_pair.len();
    let bar_primary = follow_b >= 10;
    let bar_delta = delta >= 3;
    let bar_strict = strict_b >= strict_a;
    println!("\n═ arm BT8 summary (n={n}) ═");
    println!("| follows A | follows Bt8 | delta | strict A | strict Bt8 |");
    println!("|---|");
    println!("| {follow_a}/{n} | {follow_b}/{n} | {delta:+} | {strict_a} | {strict_b} |");
    println!(
        "  pre-registered bar: follows ≥10/12 → {}; delta ≥+3 → {}; strict no-regression → {}",
        if bar_primary { "PASS" } else { "FAIL" },
        if bar_delta { "PASS" } else { "FAIL" },
        if bar_strict { "PASS" } else { "FAIL" },
    );
    println!(
        "  VERDICT: {}",
        if bar_primary && bar_delta && bar_strict {
            "PASS"
        } else {
            "FAIL"
        }
    );
    let server = client.fetch_usage_today().ok();
    if let Some(u) = &server {
        println!(
            "  server meter (today): {} req · {}+{} tok · {:.2} neurons",
            u.requests, u.input_tokens, u.output_tokens, u.neurons
        );
    }
    println!("  {}", client.spend_summary());
    if let Some(f) = out.as_mut() {
        let row = serde_json::json!({
            "summary": {
                "n": n,
                "follows_a": follow_a,
                "follows_bt8": follow_b,
                "delta": delta,
                "strict_a": strict_a,
                "strict_bt8": strict_b,
                "bar_primary": bar_primary,
                "bar_delta": bar_delta,
                "bar_strict": bar_strict,
                "model": client.model_id(),
            }
        });
        let _ = writeln!(f, "{row}");
    }
}

// ── Plan 225 T1 — the HARVEST arm: mine the heal-decision case set ────────
//
// Replays the FULL deduped v2edit population through the production pipeline
// IN ORDER, warming memory exactly as a real heal run does (report_outcome per
// served case), and records for EVERY case the (module, target lint,
// L0–L3 pool, per-candidate oracle verdict) row the seat bench needs. The
// gold oracle is the plan's three clauses: (a) merge + compile + lint-clears
// (score_case + the new after_compiled gate), (c) contract + no added
// allow/expect, and — on a deterministic pilot subset — (b) per-fn
// behavioural equivalence. Census + lint-disjoint split + digest land in the
// out dir; the T1.6 power arithmetic is computed from them.

/// One harvested candidate row (the per-candidate oracle verdict).
#[derive(serde::Serialize)]
struct HarvestCand {
    idx: usize,
    blake3_12: String,
    merge: Option<&'static str>,
    compiled: bool,
    target_resolved: bool,
    contract_ok: bool,
    attr_silence: bool,
    /// Clause (a) ∧ (c) — the cheap gold half the full population gets.
    gold_a_c: bool,
    /// Pilot-only clause (b): "ok" | "divergent" | "unresolved" | null.
    behav: Option<&'static str>,
    /// gold_a_c ∧ behav == ok — null outside the pilot.
    gold_full: Option<bool>,
}

/// Every top-level fn name in a module (the per-fn behavioural pilot's
/// driver targets). `None` when the module does not parse.
fn module_fn_names(src: &str) -> Option<Vec<String>> {
    let file: syn::File = syn::parse_file(src).ok()?;
    let mut names = Vec::new();
    for item in file.items {
        if let syn::Item::Fn(f) = item {
            names.push(f.sig.ident.to_string());
        }
    }
    Some(names)
}

/// Run the per-fn differential driver over every fn in the module:
/// "ok" | "divergent" | "unresolved" (any Unsupported fn wins only if
/// nothing diverged — a concrete counterexample outranks a shrug).
fn behavioural_pilot_verdict(
    before: &str,
    merged: &str,
    behav_dir: &std::path::Path,
    case_id: &str,
) -> &'static str {
    let Some(names) = module_fn_names(before) else {
        return "unresolved";
    };
    if names.is_empty() {
        return "unresolved";
    }
    let dir = behav_dir.join(case_id.replace(':', "_"));
    let mut any_unsupported = false;
    for name in names {
        match riir_refine::behavioural_equivalence::check(before, merged, &name, 64, &dir) {
            riir_refine::behavioural_equivalence::Verdict::Equivalent { .. } => {}
            riir_refine::behavioural_equivalence::Verdict::Divergent { .. } => return "divergent",
            riir_refine::behavioural_equivalence::Verdict::Unsupported { .. } => {
                any_unsupported = true;
            }
        }
    }
    if any_unsupported {
        "unresolved"
    } else {
        "ok"
    }
}

fn run_harvest(args: &Args, harness: &mut RegateHarness, samples: Vec<Sample>) {
    use cf_workers_ai::{apply_edit, attr_silence_added, score_case};

    /// Per-target-lint census row (T1.5/T1.6's arithmetic substrate).
    #[derive(Default, serde::Serialize)]
    struct LintCensus {
        n: usize,
        corpus_covered: bool,
        pool_empty: usize,
        pool_singleton: usize,
        pool_ge2: usize,
        deciding: usize,
        none_gold: usize,
        all_gold: usize,
        deciding_full: usize,
        #[serde(skip)]
        pool_seen: usize,
    }

    let Some(out_dir) = args.out.clone() else {
        eprintln!(
            "FATAL: --arm HARVEST needs --out <dir> (cases.jsonl + census.json + split.json)"
        );
        std::process::exit(2);
    };
    std::fs::create_dir_all(&out_dir).expect("create harvest out dir");
    let behav_dir = out_dir.join("behav");
    std::fs::create_dir_all(&behav_dir).ok();
    let mut cases = std::fs::File::create(out_dir.join("cases.jsonl")).expect("create cases.jsonl");

    let clippy_version = std::process::Command::new("cargo")
        .arg("clippy")
        .arg("--version")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    eprintln!(
        "[harvest] {} samples; clippy {clippy_version}",
        samples.len()
    );

    // ── Phase 0: labelling pass — the target lint is what FIRES in the ────
    // scaffold (row label first when it fires, else the first discovery;
    // nothing fires → the case has no oracle and is recorded as such).
    eprintln!("[harvest] phase 0: labelling (discovery clippy per module)...");
    struct Labelled {
        sample: Sample,
        target: Option<String>,
        target_source: &'static str,
    }
    let mut labelled: Vec<Labelled> = Vec::with_capacity(samples.len());
    let mut no_control = 0usize;
    let t_label = std::time::Instant::now();
    let total_samples = samples.len();
    for (n, s) in samples.into_iter().enumerate() {
        let d = harness.verifier.discover_lints(&s.module);
        let fires: Vec<&str> = d
            .present_lints
            .iter()
            .map(|l| l.strip_prefix("clippy::").unwrap_or(l))
            .collect();
        let (target, source) = if let Some(l) = s.lints.iter().find(|l| fires.contains(&l.as_str()))
        {
            (Some(l.clone()), "row")
        } else if let Some(first) = fires.first() {
            (Some((*first).to_string()), "discovered")
        } else {
            (None, "none")
        };
        if target.is_none() {
            no_control += 1;
        }
        labelled.push(Labelled {
            sample: s,
            target,
            target_source: source,
        });
        if (n + 1) % 200 == 0 {
            eprintln!(
                "[harvest] labelled {}/{} ({:.1}s)",
                n + 1,
                labelled.len(),
                t_label.elapsed().as_secs_f32()
            );
        }
    }
    eprintln!(
        "[harvest] labelling done: {} with a firing target, {no_control} without an oracle",
        labelled.len() - no_control
    );

    // ── Phase 1: the replay — classify → score every pool candidate → warm ──
    eprintln!("[harvest] phase 1: replay (pool capture + per-candidate oracle)...");
    let mut behav_cases_left = args.harvest_behavioural;
    // The pilot's clause-(b) tally (candidate-level): every gold_a_c
    // candidate on a pilot case gets behav-scored; the ok-rate IS
    // P(behav-ok | a∧c-ok) — the T1.6 full-oracle supply projection input.
    let (
        mut pilot_gold_ac,
        mut pilot_behav_ok,
        mut pilot_behav_divergent,
        mut pilot_behav_unresolved,
    ) = (0usize, 0usize, 0usize, 0usize);
    let mut per_lint: std::collections::BTreeMap<String, LintCensus> = Default::default();
    let t_replay = std::time::Instant::now();
    for (n, l) in labelled.into_iter().enumerate() {
        let target = match &l.target {
            Some(t) => t.clone(),
            None => {
                let row = serde_json::json!({
                    "case_id": l.sample.id,
                    "target_lint": serde_json::Value::Null,
                    "target_source": l.target_source,
                    "row_lints": l.sample.lints,
                    "module": l.sample.module,
                    "pool": [],
                    "stratum": "no_control",
                });
                writeln!(cases, "{row}").ok();
                continue;
            }
        };
        let c = per_lint.entry(target.clone()).or_default();
        c.n += 1;
        c.corpus_covered = c.corpus_covered || harness.corpus.lookup(&target).next().is_some();

        // The score-time sample targets ONLY the harvest target lint.
        let score_sample = Sample {
            id: l.sample.id.clone(),
            lints: vec![target.clone()],
            module: l.sample.module.clone(),
            expected: l.sample.expected.clone(),
            source: SampleSource::V2edit,
        };
        let outcome = harness.classify_outcome(&target, &l.sample.module);
        let pool_len = outcome.pool.len();
        c.pool_seen = c.pool_seen.max(pool_len);
        if pool_len >= 2 {
            c.pool_ge2 += 1;
        } else if pool_len == 1 {
            c.pool_singleton += 1;
        } else {
            c.pool_empty += 1;
        }

        // Pilot eligibility: deterministic — the first N pool≥2 cases that
        // turn out to have ≥1 (a)∧(c)-gold candidate (checked post-score).
        let pilot_case = behav_cases_left > 0 && pool_len >= 2;
        let mut pilot_used = false;
        let mut cands: Vec<HarvestCand> = Vec::with_capacity(pool_len);
        for (i, cand) in outcome.pool.iter().enumerate() {
            let merged = apply_edit(&l.sample.module, cand, false).map(|(m, _)| m);
            let score = score_case(&harness.verifier, &score_sample, cand, None);
            let gold_a = merged.is_some()
                && score.oracle_ran
                && score.after_compiled
                && score.target_resolved;
            let silences = merged
                .as_deref()
                .is_some_and(|m| attr_silence_added(&l.sample.module, m));
            let gold_c = score.contract_ok && !silences;
            let gold_a_c = gold_a && gold_c;
            let (behav, gold_full) = if pilot_case && gold_a_c {
                pilot_used = true;
                let v = behavioural_pilot_verdict(
                    &l.sample.module,
                    merged.as_deref().unwrap_or_default(),
                    &behav_dir,
                    &l.sample.id,
                );
                pilot_gold_ac += 1;
                match v {
                    "ok" => pilot_behav_ok += 1,
                    "divergent" => pilot_behav_divergent += 1,
                    _ => pilot_behav_unresolved += 1,
                }
                (Some(v), Some(v == "ok"))
            } else {
                (None, None)
            };
            cands.push(HarvestCand {
                idx: i,
                blake3_12: blake3::hash(cand.as_bytes()).to_hex().as_str()[..12].to_string(),
                merge: score.merge_strategy.map(|m| m.name()),
                compiled: score.after_compiled,
                target_resolved: score.target_resolved,
                contract_ok: score.contract_ok,
                attr_silence: silences,
                gold_a_c,
                behav,
                gold_full,
            });
        }
        if pilot_used {
            behav_cases_left -= 1;
        }

        // Strata (T1.5): on gold_a_c over the full population. The full
        // stratum (pilot rows only) is CONFIRMED-BASED: a candidate that
        // failed (a)/(c) is confirmed non-gold; a behav-scored candidate is
        // confirmed either way; a gold_a_c candidate whose behav reads
        // "unresolved" is UNRESOLVED (never gold, per the T1.4 law — but it
        // is not confirmed non-gold either). deciding_full therefore means:
        // ≥1 confirmed full-gold AND ≥1 confirmed non-gold — the stratum
        // that separates pickers under the FULL oracle. (The original
        // all-cands-scored formulation was structurally unreachable: behav
        // only ever runs on gold_a_c candidates, so non-gold candidates
        // never carried gold_full — Bench 114's pilot read 0 deciding_full
        // on 40 deciding pilot cases.)
        let (stratum, stratum_full) = if pool_len == 0 {
            ("empty_pool", None)
        } else if pool_len == 1 {
            ("singleton_pool", None)
        } else {
            let g = cands.iter().filter(|c| c.gold_a_c).count();
            let s = if g == 0 {
                "none_gold"
            } else if g == pool_len {
                "all_gold"
            } else {
                "deciding"
            };
            let gf = cands.iter().filter(|c| c.gold_full == Some(true)).count();
            // Confirmed non-gold: failed (a)/(c), or behav-DIVERGENT.
            let confirmed_non = cands
                .iter()
                .filter(|c| !c.gold_a_c || c.gold_full == Some(false))
                .count();
            let any_pilot_scored = cands.iter().any(|c| c.behav.is_some());
            let s_full = if any_pilot_scored {
                Some(if gf == 0 {
                    "none_gold_full"
                } else if confirmed_non == 0 {
                    "all_gold_full"
                } else {
                    "deciding_full"
                })
            } else {
                None
            };
            (s, s_full)
        };
        match stratum {
            "deciding" => c.deciding += 1,
            "none_gold" => c.none_gold += 1,
            "all_gold" => c.all_gold += 1,
            _ => {}
        }
        if stratum_full == Some("deciding_full") {
            c.deciding_full += 1;
        }

        // Memory warming — the production shape: the SERVED candidate's
        // real outcome feeds the trajectory store (pool diversity for later
        // same-rule cases grows exactly as production serves them).
        let mut served_idx: Option<usize> = None;
        if let Some(served) = &outcome.served {
            served_idx = outcome.pool.iter().position(|p| p == served);
            if let Some(i) = served_idx {
                let success = cands[i].gold_a_c;
                harness
                    .pipeline
                    .report_outcome(&target, &l.sample.module, served, success, true);
            }
        }

        let row = serde_json::json!({
            "case_id": l.sample.id,
            "target_lint": target,
            "target_source": l.target_source,
            "row_lints": l.sample.lints,
            "module": l.sample.module,
            "expected": l.sample.expected,
            "pool": cands,
            "pool_size": pool_len,
            "served_idx": served_idx,
            "modelless_score": outcome.score,
            "stratum": stratum,
            "stratum_full": stratum_full,
        });
        writeln!(cases, "{row}").ok();
        if (n + 1) % 100 == 0 {
            eprintln!(
                "[harvest] {}/{} replayed ({:.1}s)",
                n + 1,
                total_samples,
                t_replay.elapsed().as_secs_f32()
            );
        }
    }
    cases.flush().ok();

    // ── Phase 2: census + split + digest ──────────────────────────────

    // The lint-disjoint split (T1.7): time/source-repo provenance is absent
    // from the v2edit rows, so the leakage-safe fallback splits by LINT —
    // each lint wholly on one side, so a test rule's cases never appear in
    // training in any form. Greedy balance on scored cases, deterministic
    // order (count desc, name asc), test target ≈45% of scored.
    let mut scored_lints: Vec<(&String, &mut LintCensus)> = per_lint
        .iter_mut()
        .filter(|(_, c)| c.deciding + c.none_gold > 0)
        .collect();
    let total_scored: usize = scored_lints
        .iter()
        .map(|(_, c)| c.deciding + c.none_gold)
        .sum();
    scored_lints.sort_by(|a, b| {
        (b.1.deciding + b.1.none_gold)
            .cmp(&(a.1.deciding + a.1.none_gold))
            .then(a.0.cmp(b.0))
    });
    let test_target = total_scored * 45 / 100;
    let mut split: std::collections::BTreeMap<String, &'static str> = Default::default();
    let mut test_scored = 0usize;
    for (lint, c) in &scored_lints {
        let scored = c.deciding + c.none_gold;
        if test_scored < test_target {
            split.insert((*lint).clone(), "test");
            test_scored += scored;
        } else {
            split.insert((*lint).clone(), "train");
        }
    }
    for (lint, c) in &per_lint {
        if c.deciding + c.none_gold == 0 {
            split.insert(lint.clone(), "train"); // unscored lints are train-side structure
        }
    }

    let deciding_total: usize = per_lint.values().map(|c| c.deciding).sum();
    let none_gold_total: usize = per_lint.values().map(|c| c.none_gold).sum();
    let all_gold_total: usize = per_lint.values().map(|c| c.all_gold).sum();
    let pool_ge2_total: usize = per_lint.values().map(|c| c.pool_ge2).sum();
    let n_total: usize = per_lint.values().map(|c| c.n).sum();
    let deciding_full_total: usize = per_lint.values().map(|c| c.deciding_full).sum();

    // The pinned T1.6 power table (superiority route): N = (z_{α/2}+z_β)² ·
    // (p_d − δ₀²)/δ₀² with δ₀ = 0.05, α = 0.05 two-sided, power 0.80.
    let n_power = |p_d: f64| (7.849 * (p_d - 0.0025) / 0.0025).ceil() as usize;

    let census = serde_json::json!({
        "clippy_version": clippy_version,
        "samples": n_total + no_control,
        "no_control": no_control,
        "with_target": n_total,
        "pool_ge2": pool_ge2_total,
        "strata": {
            "deciding": deciding_total,
            "none_gold": none_gold_total,
            "all_gold_unscored": all_gold_total,
        },
        "behav_pilot": {
            "cases": args.harvest_behavioural - behav_cases_left,
            "deciding_full": deciding_full_total,
            "gold_ac_candidates": pilot_gold_ac,
            "behav_ok": pilot_behav_ok,
            "behav_divergent": pilot_behav_divergent,
            "behav_unresolved": pilot_behav_unresolved,
            "p_behav_ok_given_gold_ac": if pilot_gold_ac > 0 {
                Some(pilot_behav_ok as f64 / pilot_gold_ac as f64)
            } else {
                None
            },
        },
        "t16_power_table": {
            "inputs": "delta0=0.05, alpha=0.05 two-sided, power=0.80, paired per-case score",
                       "n_at_pd_0.20": n_power(0.20),
            "n_at_pd_0.35": n_power(0.35),
            "n_at_pd_0.50": n_power(0.50),
        },
        "per_lint": per_lint,
        "split": &split,
        "split_test_scored": test_scored,
    });
    std::fs::write(
        out_dir.join("census.json"),
        serde_json::to_string_pretty(&census).unwrap(),
    )
    .expect("write census.json");
    std::fs::write(
        out_dir.join("split.json"),
        serde_json::to_string_pretty(&split).unwrap(),
    )
    .expect("write split.json");

    println!("═ arm HARVEST — Plan 225 T1 case-set census ═");
    println!("  clippy {clippy_version}");
    println!(
        "  modules: {} ({} without a firing target)",
        n_total + no_control,
        no_control
    );
    println!("  pool≥2: {pool_ge2_total}   deciding: {deciding_total}   none-gold: {none_gold_total}   all-gold(unscored): {all_gold_total}");
    println!(
        "  behav pilot cases: {}   deciding_full among them: {deciding_full_total}",
        args.harvest_behavioural - behav_cases_left
    );
    println!(
        "  clause-(b) pilot: {pilot_gold_ac} gold_a_c cands → ok {pilot_behav_ok} / divergent {pilot_behav_divergent} / unresolved {pilot_behav_unresolved}"
    );
    println!(
        "  split (lint-disjoint): test {} / {} scored",
        test_scored, total_scored
    );
    println!(
        "  T1.6 N(p_d): p_d=0.20 → {}, 0.35 → {}, 0.50 → {}",
        n_power(0.20),
        n_power(0.35),
        n_power(0.50)
    );
    for (lint, c) in &per_lint {
        println!(
            "    {:<28} n={:<4} corpus={} pool≥2={:<4} deciding={:<3} none-gold={:<3} all-gold={:<3}",
            lint,
            c.n,
            if c.corpus_covered { "y" } else { "n" },
            c.pool_ge2,
            c.deciding,
            c.none_gold,
            c.all_gold
        );
    }
    eprintln!("[harvest] artifacts: {}", out_dir.display());
}

// ── Plan 202 R6 — arm H: the escalation chain over the existing harness ────
//
// The hosted arm: per case, the modelless answer (the pipeline's pick, from
// `prepare`) vs the ESCALATED answer — the armed manifest row's transport
// (seat rerank over the L0–L3 pool, or hosted generate on a miss). Paired
// per-case on the same case list, with the explicit cost axes (the plan's
// R6 row): wall/receipt time per answer and, for the hosted leg, the
// llm_spend ledger. The demotion itself is an operator manifest edit — the
// arm PRINTS the loser, never edits data.

/// Entry gate: the dispatch module is compiled only with the manifest AND a
/// transport; everything else refuses loudly naming the missing features.
/// `store` is the harness's durable trajectory store (Issue 156: the rescue
/// tallies' home; `None` = run-local tallies, honestly disclosed).
fn hosted_dispatch(
    args: &Args,
    verifier: &ClippyVerifier,
    prepared: &[Prepared],
    store: Option<&riir_refine::traj_store::TrajStore>,
) {
    #[cfg(all(
        feature = "escalation_manifest",
        any(feature = "seat_rerank", feature = "hosted_expert")
    ))]
    {
        run_hosted(args, verifier, prepared, store);
    }
    #[cfg(not(all(
        feature = "escalation_manifest",
        any(feature = "seat_rerank", feature = "hosted_expert")
    )))]
    {
        let _ = (args, verifier, prepared, store);
        eprintln!(
            "--arm H requires --features escalation_manifest plus the armed row's transport \
             (seat_rerank and/or hosted_expert) — an armed row must never look armed while \
             doing nothing"
        );
        std::process::exit(2);
    }
}

#[cfg(all(
    feature = "escalation_manifest",
    any(feature = "seat_rerank", feature = "hosted_expert")
))]
fn run_hosted(
    args: &Args,
    verifier: &ClippyVerifier,
    prepared: &[Prepared],
    store: Option<&riir_refine::traj_store::TrajStore>,
) {
    use riir_refine::escalation_dispatch::{
        EscalatedFix, EscalationDispatch, ReadinessPolicy, PRESENTATION_CAP,
    };
    use riir_refine::escalation_manifest::EscalationManifest;

    // ── arm (manifest-only; the ALL-EMPTY posture measures nothing) ────
    let manifest = match EscalationManifest::load(&args.esc_manifest) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(1);
        }
    };
    let mut dispatch = match EscalationDispatch::arm(&manifest, &args.esc_domain) {
        Ok(Some(d)) => d,
        Ok(None) => {
            eprintln!(
                "SKIP arm H: no armed escalation row for {:?} in {} — arm H measures the \
                 ESCALATION chain; add the row (arming is a manifest edit, never code)",
                args.esc_domain,
                args.esc_manifest.display()
            );
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(1);
        }
    };
    eprintln!(
        "[regate] arm H: domain={} kind={} cap={} max_rate={} deadline={}ms consent={}",
        dispatch.label(),
        dispatch.row().target.kind,
        dispatch.row().modelless_cap,
        dispatch.row().max_rate,
        args.deadline_ms,
        if std::env::var("RIIR_REFINE_CODE_EGRESS").is_ok_and(|v| v.trim() == "snippet") {
            "granted"
        } else {
            "REFUSED (set RIIR_REFINE_CODE_EGRESS=snippet for live seats)"
        }
    );

    let policy = ReadinessPolicy::default();
    let mut out = args
        .out
        .as_ref()
        .and_then(|p| std::fs::File::create(p).ok());
    let mut escalated_summary = ArmSummary::default();
    let mut modelless_summary = ArmSummary::default();

    // Paired axes.
    let (mut esc_wins, mut esc_losses, mut ties) = (0usize, 0usize, 0usize);
    let (mut n_worthy, mut n_pick, mut n_generate) = (0usize, 0usize, 0usize);
    let (mut n_abstain, mut n_below_min, mut n_declined, mut n_latched) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut esc_wall_ms_total, mut esc_wall_ms_max) = (0u128, 0u128);
    let mut cases_escalated = 0u128;
    #[cfg(feature = "seat_rerank")]
    let mut receipt_us_total = 0u64;
    #[cfg(not(feature = "seat_rerank"))]
    let receipt_us_total = 0u64;
    #[cfg(not(feature = "seat_rerank"))]
    let _ = receipt_us_total;
    let kind = dispatch.row().target.kind.clone();
    // Issue 156: one run id per invocation (the store's per-run attribution
    // convention) + the run's touched states for the rescue summary.
    let run_id = format!(
        "armh-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    );
    let mut touched: Vec<(String, String, riir_refine::rescue_mine::RescueTally)> = Vec::new();

    for p in prepared {
        eprintln!("[H] {} …", p.sample.id);
        // ── the modelless arm (the incumbent the chain protects) ───────
        let modelless_gen: String = p.modelless.clone().unwrap_or_default();
        let score_m =
            cf_workers_ai::score_case(verifier, &p.sample, &modelless_gen, Some(&args.behav_dir));
        modelless_summary.add(&score_m, p.reachable_miss, 0, 0, 0.0, 0.0);

        // ── the escalation arm (worthiness → ceiling → transport) ──────
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_millis(args.deadline_ms);
        let (outcome_class, served_from, fix, receipt_note, wall_ms) = match kind.as_str() {
            "seat_rerank" => {
                let t = std::time::Instant::now();
                let answer =
                    dispatch.escalate_rerank(p.score, &p.sample.module, &p.pool, deadline, policy);
                let wall = t.elapsed().as_millis();
                // The R7 receipt tier (seat_rerank builds only): the
                // seat-side decision time + the build/confidence audit
                // note, read BEFORE the answer is consumed below.
                #[cfg(feature = "seat_rerank")]
                if let Some(r) = answer.receipt() {
                    receipt_us_total += r.us.unwrap_or(0);
                }
                let note = receipt_audit_note(&answer);
                match answer {
                    EscalatedFix::Answer { fix, source, .. } => {
                        n_pick += 1;
                        (outcome_class_of(&fix), source.to_string(), fix, note, wall)
                    }
                    EscalatedFix::StoodByModelless { reason } => {
                        let (class, reason) = stand_reason_axes(
                            reason,
                            &mut n_worthy,
                            &mut n_abstain,
                            &mut n_below_min,
                            &mut n_declined,
                            &mut n_latched,
                        );
                        (
                            class,
                            "modelless".to_string(),
                            modelless_gen.clone(),
                            reason,
                            wall,
                        )
                    }
                }
            }
            "hosted_expert" => {
                let t = std::time::Instant::now();
                let fix = dispatch.escalate_generate(
                    p.score,
                    &p.sample.module,
                    deadline,
                    args.max_new as u64,
                );
                let wall = t.elapsed().as_millis();
                match fix {
                    EscalatedFix::Answer { fix, source, .. } => {
                        n_generate += 1;
                        (
                            outcome_class_of(&fix),
                            source.to_string(),
                            fix,
                            "no-receipt".to_string(),
                            wall,
                        )
                    }
                    EscalatedFix::StoodByModelless { reason } => {
                        let (class, reason) = stand_reason_axes(
                            reason,
                            &mut n_worthy,
                            &mut n_abstain,
                            &mut n_below_min,
                            &mut n_declined,
                            &mut n_latched,
                        );
                        (
                            class,
                            "modelless".to_string(),
                            modelless_gen.clone(),
                            reason,
                            wall,
                        )
                    }
                }
            }
            other => (
                "skip".to_string(),
                "modelless".to_string(),
                modelless_gen.clone(),
                format!("unknown kind {other:?}"),
                0,
            ),
        };
        if served_from != "modelless" {
            esc_wall_ms_total += wall_ms;
            esc_wall_ms_max = esc_wall_ms_max.max(wall_ms);
            cases_escalated += 1;
        }

        // ── score the escalated answer (same instrument, same case) ────
        let score_e = cf_workers_ai::score_case(verifier, &p.sample, &fix, Some(&args.behav_dir));
        escalated_summary.add(&score_e, p.reachable_miss, 0, 0, 0.0, 0.0);

        // Paired keep_strict verdict.
        match (score_e.keep_strict, score_m.keep_strict) {
            (true, false) => esc_wins += 1,
            (false, true) => esc_losses += 1,
            _ => ties += 1,
        }

        // ── Issue 156 T1: the FlyBy state tallies (bookkeeping over the
        // SAME paired continuations — no new sampling). b0 = the no-tool
        // (modelless) continuation's outcome; bd = the tool-assisted
        // (escalated) outcome, counted ONLY when the transport actually
        // served — a stood-by-modelless leg is a no-tool continuation in
        // disguise. The store write rides the orthogonal tally axis (never
        // the outcome counters); a verified rescue additionally lands as
        // the existing success+verified evidence (the store method's job).
        let m_success = score_m.keep_strict;
        let e_served = served_from != "modelless";
        let e_depth = riir_refine::rescue_mine::RescueDepth::from_kind(&kind);
        let escalated_obs = if e_served {
            e_depth.map(|d| (score_e.keep_strict, d))
        } else {
            None
        };
        let mut rescue = riir_refine::rescue_mine::RescueTally::default();
        rescue.observe_modelless(m_success);
        if let Some((s, d)) = escalated_obs {
            rescue.observe_escalated(s, d);
        }
        if let Some(st) = store {
            if let Err(e) = st.record_rescue_pair(
                &p.sample.lints[0],
                &p.sample.module,
                &fix,
                m_success,
                escalated_obs,
                &run_id,
            ) {
                eprintln!("[H] rescue tally write failed: {e}");
            }
        }
        touched.push((p.sample.lints[0].clone(), p.sample.module.clone(), rescue));

        if let Some(f) = out.as_mut() {
            let row = serde_json::json!({
                "id": p.sample.id,
                "arm": "H",
                "kind": dispatch.row().target.kind,
                "miss_class": format!("{:?}", p.miss),
                "modelless": {
                    "served": p.modelless.is_some(),
                    "us": p.modelless_us,
                    "keep_o": score_m.keep_oracle,
                    "keep_s": score_m.keep_strict,
                    "em": score_m.em,
                    // The worthiness gate's INPUT (the R6 audit trail — a
                    // cap choice must be quotable against the score
                    // distribution it routes on).
                    "score": p.score,
                },
                "escalated": {
                    "outcome_class": outcome_class,
                    "served_from": served_from,
                    "pool_len": p.pool.len(),
                    "pool_truncated": p.pool.len() > PRESENTATION_CAP,
                    "keep_o": score_e.keep_oracle,
                    "keep_s": score_e.keep_strict,
                    "em": score_e.em,
                    "wall_ms": wall_ms,
                    "receipt": receipt_note,
                },
                // Issue 156 T1: this run's FlyBy contribution for the state
                // (the accumulated history lives in the store, not here).
                // `depth` names the depth a tool-assisted leg RAN at —
                // "none" when the chain stood by modelless.
                "rescue": {
                    "b0_attempted": 1,
                    "b0_success": m_success,
                    "bd_attempted": e_served,
                    "bd_success": e_served && score_e.keep_strict,
                    "depth": escalated_obs
                        .as_ref()
                        .map_or("none", |(_, d)| d.label()),
                },
            });
            let _ = writeln!(f, "{row}");
        }
        println!(
            "  {:<28} miss={:<13} pool={:<2} m[keep_s={} {}µs] e[{} {} keep_s={} {}ms {}]",
            p.sample.id,
            format!("{:?}", p.miss),
            p.pool.len(),
            score_m.keep_strict,
            p.modelless_us,
            served_from,
            outcome_class,
            score_e.keep_strict,
            wall_ms,
            receipt_note,
        );
    }

    // ── the paired summary + the cost axes ────────────────────────────
    let n = prepared.len();
    println!(
        "\n═ arm H summary ({n} paired cases; kind={}) ═",
        dispatch.row().target.kind
    );
    println!("| arm | n | EM | parses | merge | keep_o/scoreable | keep_s | vetoed | divergent | regressions | $ |");
    println!("{}", modelless_summary.markdown_row("H-modelless"));
    println!("{}", escalated_summary.markdown_row("H-escalated"));
    println!(
        "  chain: worthy(stands) {n_worthy} · seat picks {n_pick} · hosted generates {n_generate} · \
         abstained {n_abstain} · below min_pool {n_below_min} · declined {n_declined} · rate-latched {n_latched}"
    );
    println!(
        "  paired keep_s over {n}: escalated wins {esc_wins}, loses {esc_losses}, ties {ties}"
    );
    if cases_escalated > 0 {
        println!(
            "  cost axes: escalated wall total {esc_wall_ms_total}ms over {cases_escalated} case(s) \
             (max {esc_wall_ms_max}ms); seat-side decision time total {receipt_us_total}µs; \
             modelless arm total {}µs",
            prepared.iter().map(|p| p.modelless_us).sum::<u128>(),
        );
    } else {
        println!(
            "  cost axes: no escalated case served — modelless arm total {}µs",
            prepared.iter().map(|p| p.modelless_us).sum::<u128>()
        );
    }
    #[cfg(feature = "hosted_expert")]
    if let Some(line) = dispatch.spend_summary_line() {
        println!("  {line}");
    }
    println!("  {}", escalated_summary.verdict_vs_decline());
    let verdict = if esc_wins > esc_losses {
        "ESCALATION WINS — demote the modelless incumbent via the manifest cap (raise modelless_cap), never code"
    } else if esc_losses > esc_wins {
        "MODELLESS WINS — demote the escalation (manifest row demoted:true or the RIIR_REFINE_ESC=0 literal), never code"
    } else {
        "TIE — no outcome-based selection signal; keep the modelless default (free + µs)"
    };
    println!("  VERDICT: {verdict}");

    // ── Issue 156 T2/T5: the rescue exemplar set over this run's states ──
    // Read back the ACCUMULATED tallies when the store is durable — a
    // single run contributes at most one continuation per arm, so the keep
    // predicate (bd≥3) can only fire on the store's history. Run-local
    // tallies are still summarized honestly when no store is present.
    let states: Vec<riir_refine::rescue_mine::StateRescueTally> = touched
        .iter()
        .map(|(lint, shape, run_tally)| {
            let (tally, tier) = match store {
                Some(st) => st.rescue_state(lint, shape),
                None => (*run_tally, riir_refine::memory::EvidenceTier::Heuristic),
            };
            riir_refine::rescue_mine::StateRescueTally {
                state_id: format!("{lint}:{}", riir_refine::traj_store::shape_hash(shape)),
                b0_attempts: tally.b0_attempts,
                b0_successes: tally.b0_successes,
                bd_attempts: tally.bd_attempts,
                bd_successes: tally.bd_successes,
                depth: tally.depth,
                tier,
            }
        })
        .collect();
    let rescue_summary = riir_refine::rescue_mine::summarize_states(&states);
    println!(
        "  rescue tallies (Issue 156, FlyBy b0=0 ∧ bd≥3 — augment-only pre-filter): {} state(s) \
         this run; keep-set {}, learnable-hard {}, intractable-eviction candidates {}, \
         unprobed {}, exec-reachable {}",
        rescue_summary.population,
        rescue_summary.keep,
        rescue_summary.learnable_hard,
        rescue_summary.intractable,
        rescue_summary.unprobed,
        rescue_summary.exec_reachable,
    );
    if rescue_summary.keep > 0 {
        println!(
            "  keep-set hash: {} (same transcripts → identical)",
            &rescue_summary.keep_hash[..12]
        );
        for s in states.iter().filter(|s| s.keep()) {
            println!(
                "    exemplar: {} depth={} tier={}",
                s.state_id,
                riir_refine::rescue_mine::RescueDepth::from_u8(s.depth).label(),
                riir_refine::rescue_mine::tier_label(s.tier),
            );
        }
    }
    if store.is_none() {
        println!(
            "  (no durable store — tallies are run-local; run from the repo root or set \
             RIIR_REGATE_STORE_DIR)"
        );
    }

    // The quality time series gets the ESCALATED arm's scores (the modelless
    // baseline's numbers are arm H-modelless above; the incumbent series is
    // the existing arms').
    escalated_summary.latency_ms_total = esc_wall_ms_total;
    escalated_summary.latency_ms_max = esc_wall_ms_max;
    let model_tag = match dispatch.row().target.kind.as_str() {
        "seat_rerank" => format!("seat:{}:heal", args.esc_domain),
        _ => "hosted_expert".to_string(),
    };
    append_quality_history("H", &model_tag, &escalated_summary);
}

/// The receipt-audit note for one dispatch answer (seat_rerank builds read
/// the R7 receipt tier; every other build/answer reports `no-receipt`).
#[cfg(all(
    feature = "escalation_manifest",
    any(feature = "seat_rerank", feature = "hosted_expert")
))]
fn receipt_audit_note(answer: &riir_refine::escalation_dispatch::EscalatedFix) -> String {
    #[cfg(feature = "seat_rerank")]
    if let Some(r) = answer.receipt() {
        return match (&r.build, r.us, &r.confidence) {
            (Some(b), Some(us), Some(c)) => format!(
                "build={}&us={us}&conf={c}",
                b.chars().take(8).collect::<String>()
            ),
            _ => "partial-receipt".to_string(),
        };
    }
    #[cfg(not(feature = "seat_rerank"))]
    let _ = answer;
    "no-receipt".to_string()
}

/// The outcome class for an ANSWER (audit axis on the rows file) — what
/// kind of thing the chain served.
#[cfg(all(
    feature = "escalation_manifest",
    any(feature = "seat_rerank", feature = "hosted_expert")
))]
fn outcome_class_of(fix: &str) -> String {
    if fix.trim().is_empty() {
        "empty".to_string()
    } else {
        "answer".to_string()
    }
}

/// Fold a [`StandReason`] into the paired counters + the row axes. Returns
/// (outcome_class, reason_text).
#[cfg(all(
    feature = "escalation_manifest",
    any(feature = "seat_rerank", feature = "hosted_expert")
))]
fn stand_reason_axes(
    reason: riir_refine::escalation_dispatch::StandReason,
    n_worthy: &mut usize,
    n_abstain: &mut usize,
    n_below_min: &mut usize,
    n_declined: &mut usize,
    n_latched: &mut usize,
) -> (String, String) {
    match reason {
        riir_refine::escalation_dispatch::StandReason::Worthy => {
            *n_worthy += 1;
            ("worthy".to_string(), "modelless_cap met".to_string())
        }
        riir_refine::escalation_dispatch::StandReason::SeatAbstained => {
            *n_abstain += 1;
            (
                "abstained".to_string(),
                "seat abstained — modelless stands".to_string(),
            )
        }
        riir_refine::escalation_dispatch::StandReason::BelowMinPool => {
            *n_below_min += 1;
            (
                "below_min_pool".to_string(),
                "pool below the row's bound".to_string(),
            )
        }
        riir_refine::escalation_dispatch::StandReason::RateLatched => {
            *n_latched += 1;
            (
                "rate_latched".to_string(),
                "cost ceiling latched (sticky)".to_string(),
            )
        }
        riir_refine::escalation_dispatch::StandReason::Declined(m) => {
            *n_declined += 1;
            ("declined".to_string(), m)
        }
    }
}

// ── Plan 225 T2 — the POOLTEXTS arm: regenerate the frozen pools' texts ────
//
// The frozen cases.jsonl carries gate flags + per-candidate blake3_12 but
// NOT candidate texts. This replay walks the frozen rows in file order,
// re-runs `classify_outcome` per row (the harvest's exact pool birth),
// verifies every regenerated candidate's blake3_12 against the frozen
// entry, warms memory with the frozen served success (the harvest's exact
// sequence), and emits `{case_id, texts, blake_ok, served_ok}` rows for
// the trainer and the frozen read.
fn run_pooltexts(args: &Args, harness: &mut RegateHarness) {
    let cases_path = args
        .frozen_cases
        .as_ref()
        .unwrap_or_else(|| panic!("--arm POOLTEXTS needs --frozen-cases <cases.jsonl>"));
    let out_path = args
        .out
        .as_ref()
        .unwrap_or_else(|| panic!("--arm POOLTEXTS needs --out <pool_texts.jsonl>"));
    let raw = std::fs::read_to_string(cases_path)
        .unwrap_or_else(|e| panic!("frozen cases {}: {e}", cases_path.display()));
    let mut out = std::io::BufWriter::new(std::fs::File::create(out_path).expect("out creates"));
    let (mut ok_rows, mut drift_rows) = (0usize, 0usize);
    let t = std::time::Instant::now();
    for (n, line) in raw.lines().enumerate() {
        let row: serde_json::Value = serde_json::from_str(line).expect("frozen row parses");
        let Some(target) = row["target_lint"].as_str().map(str::to_string) else {
            continue; // no_control rows: skipped entirely, exactly as harvest
        };
        let module = row["module"].as_str().unwrap_or_default().to_string();
        let frozen_pool = row["pool"].as_array().cloned().unwrap_or_default();

        let outcome = harness.classify_outcome(&target, &module);
        let mut blake_ok = outcome.pool.len() == frozen_pool.len();
        let mut texts: Vec<String> = Vec::with_capacity(outcome.pool.len());
        for (i, frozen) in frozen_pool.iter().enumerate() {
            match outcome.pool.get(i) {
                Some(text) => {
                    let h = blake3::hash(text.as_bytes()).to_hex();
                    if &h[..12] != frozen["blake3_12"].as_str().unwrap_or("") {
                        blake_ok = false;
                    }
                    texts.push(text.clone());
                }
                None => blake_ok = false,
            }
        }

        // Memory warming — the harvest's exact shape: the SERVED candidate's
        // frozen (a∧c) verdict feeds the pipeline, in file order.
        if let Some(served) = &outcome.served {
            if let Some(i) = outcome.pool.iter().position(|p| p == served) {
                let success = frozen_pool
                    .get(i)
                    .and_then(|c| c["gold_a_c"].as_bool())
                    .unwrap_or(false);
                harness
                    .pipeline
                    .report_outcome(&target, &module, served, success, true);
            }
        }
        let served_ok = outcome
            .served
            .as_ref()
            .and_then(|s| outcome.pool.iter().position(|p| p == s))
            == row["served_idx"].as_u64().map(|v| v as usize);
        if blake_ok && served_ok {
            ok_rows += 1;
        } else {
            drift_rows += 1;
            eprintln!(
                "[pooltexts] DRIFT {} blake_ok={blake_ok} served_ok={served_ok}",
                row["case_id"]
            );
        }
        let row_out = serde_json::json!({
            "case_id": row["case_id"],
            "texts": texts,
            "blake_ok": blake_ok,
            "served_ok": served_ok,
        });
        writeln!(out, "{row_out}").ok();
        if (n + 1) % 100 == 0 {
            eprintln!(
                "[pooltexts] {}/{} lines ({:.1}s)",
                n + 1,
                raw.lines().count(),
                t.elapsed().as_secs_f32()
            );
        }
    }
    let total = ok_rows + drift_rows;
    eprintln!(
        "[pooltexts] done: {total} target rows, ok={ok_rows} drift={drift_rows} ({:.1}s)",
        t.elapsed().as_secs_f32()
    );
    // Environment drift is measured, not refused here: the frozen pin is
    // enforced PER CONSUMER (the trainer drops unpinned rows; the frozen
    // read enforces the pre-registered ≤5%-of-scored-test wall). Only a
    // total reproduction failure is fatal at this layer.
    if total == 0 || ok_rows * 10 < total {
        eprintln!("FATAL: pool replay reproduced under 10% — the frozen pin does not hold at all; investigate before training or reading");
        std::process::exit(1);
    }
    out.flush().ok();
}

// ── Plan 225 T4 — the FROZEN arm: the one paired read over the live seat ──
//
// Pre-registered pins (Bench 119 records them beside the run): primary
// oracle = a∧c (`gold_a_c`); deciding stratum = the pre-registered test
// (T1.6's power population), none-gold reported with its n (no inference
// at n<2); A0 = verify-in-order over the frozen gate fields (first
// `compiled && target_resolved` else the served fallback); superiority
// = paired LB95 > 0 (Holm, m=1 this round — A2/T3 not minted); NI route
// = paired LB95 > −0.02 AND seat cost below the A0 verify-in-order wall.
// Pool drift drops the case; >5% drift invalidates the read.
#[cfg(feature = "escalation_manifest")]
fn run_frozen(args: &Args, verifier: &ClippyVerifier) {
    use riir_refine::escalation_dispatch::{EscalatedFix, EscalationDispatch, ReadinessPolicy};
    use riir_refine::escalation_manifest::EscalationManifest;

    let cases_path = args
        .frozen_cases
        .as_ref()
        .unwrap_or_else(|| panic!("--arm FROZEN needs --frozen-cases <cases.jsonl>"));
    let pools_path = args
        .frozen_pools
        .as_ref()
        .unwrap_or_else(|| panic!("--arm FROZEN needs --frozen-pools <pool_texts.jsonl>"));
    let split_path = args
        .frozen_split
        .as_ref()
        .unwrap_or_else(|| panic!("--arm FROZEN needs --frozen-split <split.json>"));
    let split: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(split_path).expect("split.json reads"))
            .expect("split.json parses");

    // case_id → (texts, pins_ok)
    let mut pools = std::collections::BTreeMap::new();
    for line in std::fs::read_to_string(pools_path)
        .expect("pool_texts reads")
        .lines()
    {
        let row: serde_json::Value = serde_json::from_str(line).expect("pool row parses");
        let id = row["case_id"].as_str().unwrap_or_default().to_string();
        let texts: Vec<String> = row["texts"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|t| t.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default();
        let ok = row["blake_ok"].as_bool().unwrap_or(false)
            && row["served_ok"].as_bool().unwrap_or(false);
        pools.insert(id, (texts, ok));
    }

    let manifest = match EscalationManifest::load(&args.esc_manifest) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(1);
        }
    };
    // The manifest is CONSULTED per case (a fresh dispatch per case — the
    // rolling-rate latch's window is 200 observes and the bench posture is
    // ~100% escalation, so one process-wide dispatch would RateLatch at
    // case #200; the fresh-per-case posture keeps the window unfilled and
    // the seat live for every scored case).
    if matches!(
        EscalationDispatch::arm(&manifest, &args.esc_domain),
        Ok(None) | Err(_)
    ) {
        eprintln!(
            "SKIP arm FROZEN: no armed escalation row for {:?} in {}",
            args.esc_domain,
            args.esc_manifest.display()
        );
        std::process::exit(3);
    }

    let mut out = args
        .out
        .as_ref()
        .map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("ledger creates")));
    let policy = ReadinessPolicy::default();
    let (mut a1_dec, mut a0_dec) = (Vec::<bool>::new(), Vec::<bool>::new());
    let (mut a1_none, mut a0_none) = (Vec::<bool>::new(), Vec::<bool>::new());
    let (mut n_dropped_drift, mut n_scored) = (0usize, 0usize);
    let (mut n_pick, mut n_abstain, mut n_stood_other) = (0usize, 0usize, 0usize);
    let mut seat_wall_ms: Vec<f64> = Vec::new();
    let mut seat_us: Vec<u64> = Vec::new();
    let mut a0_cost_ms: Vec<f64> = Vec::new();
    let t = std::time::Instant::now();

    for line in std::fs::read_to_string(cases_path)
        .expect("cases reads")
        .lines()
    {
        let row: serde_json::Value = serde_json::from_str(line).expect("frozen row parses");
        let Some(target) = row["target_lint"].as_str() else {
            continue;
        };
        if split.get(target).map(String::as_str) != Some("test") {
            continue;
        }
        let stratum = row["stratum"].as_str().unwrap_or_default();
        if stratum != "deciding" && stratum != "none_gold" {
            continue;
        }
        let id = row["case_id"].as_str().unwrap_or_default().to_string();
        let Some((texts, pins_ok)) = pools.get(&id) else {
            n_dropped_drift += 1;
            continue;
        };
        if !pins_ok {
            n_dropped_drift += 1;
            eprintln!("[frozen] DROP (pool pin) {id}");
            continue;
        }
        let frozen_pool = row["pool"].as_array().cloned().unwrap_or_default();
        let gold: Vec<bool> = frozen_pool
            .iter()
            .map(|c| c["gold_a_c"].as_bool().unwrap_or(false))
            .collect();

        // A0 — verify-in-order over the FROZEN gate fields (T4.1).
        let a0_idx = frozen_pool
            .iter()
            .position(|c| {
                c["compiled"].as_bool().unwrap_or(false)
                    && c["target_resolved"].as_bool().unwrap_or(false)
            })
            .or_else(|| row["served_idx"].as_u64().map(|v| v as usize));
        let a0_score = a0_idx.and_then(|i| gold.get(i).copied()).unwrap_or(false);

        // A0 cost leg — the verify-in-order wall on a pre-registered
        // subsample (the first N scored rows; the gates run exactly as the
        // harvest ran them).
        if a0_cost_ms.len() < args.a0_cost_sample {
            let module = row["module"].as_str().unwrap_or_default();
            let score_sample = Sample {
                id: id.clone(),
                lints: vec![target.to_string()],
                module: module.to_string(),
                expected: row["expected"].as_str().map(str::to_string),
                source: SampleSource::V2edit,
            };
            let tc = std::time::Instant::now();
            for cand in texts {
                let merged = cf_workers_ai::apply_edit(module, cand, false);
                if let Some((merged_module, _)) = merged {
                    let sc = cf_workers_ai::score_case(verifier, &score_sample, cand, None);
                    let _ = merged_module;
                    if sc.oracle_ran && sc.after_compiled && sc.target_resolved {
                        break;
                    }
                }
            }
            a0_cost_ms.push(tc.elapsed().as_secs_f64() * 1000.0);
        }

        // A1 — the LIVE seat (fresh dispatch per case, see above).
        let t_seat = std::time::Instant::now();
        let answer = match EscalationDispatch::arm(&manifest, &args.esc_domain) {
            Ok(Some(mut d)) => d.escalate_rerank(
                row["modelless_score"].as_f64().unwrap_or(0.0) as f32,
                row["module"].as_str().unwrap_or_default(),
                texts,
                std::time::Instant::now() + std::time::Duration::from_millis(args.deadline_ms),
                policy,
            ),
            _ => {
                eprintln!("FATAL: dispatch refused mid-read");
                std::process::exit(1);
            }
        };
        let wall_ms = t_seat.elapsed().as_secs_f64() * 1000.0;
        seat_wall_ms.push(wall_ms);
        let (a1_score, seat_note): (bool, String) = match &answer {
            EscalatedFix::Answer { fix, .. } => {
                n_pick += 1;
                let idx = texts.iter().position(|t| *t == *fix);
                match idx.and_then(|i| gold.get(i).copied()) {
                    Some(g) => (g, format!("pick:{:?}", idx.unwrap())),
                    None => {
                        eprintln!("[frozen] seat pick not in pool for {id} — scoring 0");
                        (false, "pick:NOT_IN_POOL".to_string())
                    }
                }
            }
            EscalatedFix::StoodByModelless { reason } => {
                // Abstention is first-class: abstain-when-none-gold = 1,
                // abstain on a deciding case = 0 (no gold picked).
                let abstained = matches!(
                    reason,
                    riir_refine::escalation_dispatch::StandReason::SeatAbstained
                );
                if abstained {
                    n_abstain += 1;
                } else {
                    n_stood_other += 1;
                }
                (
                    abstained && stratum == "none_gold",
                    format!("stand:{reason:?}"),
                )
            }
        };
        #[cfg(feature = "seat_rerank")]
        if let Some(r) = answer.receipt() {
            if let Some(us) = r.us {
                seat_us.push(us);
            }
        }

        let modelless_idx = row["served_idx"].as_u64().map(|v| v as usize);
        let modelless_score = modelless_idx
            .and_then(|i| gold.get(i).copied())
            .unwrap_or(false);
        if stratum == "deciding" {
            a1_dec.push(a1_score);
            a0_dec.push(a0_score);
        } else {
            a1_none.push(a1_score);
            a0_none.push(a0_score);
        }
        n_scored += 1;
        if let Some(w) = out.as_mut() {
            let receipt_us = seat_us.last().copied().unwrap_or(0);
            let ledger = serde_json::json!({
                "case_id": id,
                "stratum": stratum,
                "pool_size": texts.len(),
                "a0": {"pick_idx": a0_idx, "score": a0_score},
                "seat": {"note": seat_note, "score": a1_score, "wall_ms": wall_ms, "receipt_us": receipt_us},
                "modelless": {"score": modelless_score},
            });
            writeln!(w, "{ledger}").ok();
        }
        if n_scored % 50 == 0 {
            eprintln!(
                "[frozen] {n_scored} scored ({:.1}s)",
                t.elapsed().as_secs_f32()
            );
        }
    }

    // ── the paired verdict (T4.2/T4.3, pre-registered) ──────────────────
    let paired = |a1: &[bool], a0: &[bool]| -> (f64, f64, usize, usize, usize) {
        let n = a1.len();
        let diffs: Vec<f64> = a1
            .iter()
            .zip(a0)
            .map(|(&x, &y)| i8::from(x) as f64 - i8::from(y) as f64)
            .collect();
        let mean = diffs.iter().sum::<f64>() / n.max(1) as f64;
        let wins = a1.iter().zip(a0).filter(|(x, y)| **x && !**y).count();
        let losses = a1.iter().zip(a0).filter(|(x, y)| !**x && **y).count();
        let se = if n > 1 {
            let var = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
            (var / n as f64).sqrt()
        } else {
            0.0
        };
        (mean, mean - 1.96 * se, wins, losses, n)
    };
    let (dec_mean, dec_lb95, dec_wins, dec_losses, dec_n) = paired(&a1_dec, &a0_dec);
    let (none_mean, none_lb95, none_wins, none_losses, none_n) = paired(&a1_none, &a0_none);
    let a1_dec_correct = a1_dec.iter().filter(|b| **b).count();
    let a0_dec_correct = a0_dec.iter().filter(|b| **b).count();
    let p50 = |v: &mut Vec<f64>| -> f64 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v.get(v.len() / 2).copied().unwrap_or(0.0)
    };
    let seat_p50 = p50(&mut seat_wall_ms);
    let seat_max = seat_wall_ms.iter().cloned().fold(0.0f64, f64::max);
    let a0_p50 = p50(&mut a0_cost_ms);

    println!("\n═ arm FROZEN — Plan 225 T4: the heal-seat frozen read ═");
    println!(
        "scored: {n_scored} (deciding {} · none_gold {}) · dropped(pool pin) {n_dropped_drift}",
        a1_dec.len(),
        a1_none.len()
    );
    println!(
        "DECIDING  n={dec_n}: A1 {a1_dec_correct}/{dec_n} · A0 {a0_dec_correct}/{dec_n} · W/L/T {dec_wins}/{dec_losses}/{} · mean {dec_mean:+.5} · LB95 {dec_lb95:+.5}",
        dec_n - dec_wins - dec_losses
    );
    println!(
        "NONE_GOLD n={none_n}: A1 {}/{} · A0 {}/{} · W/L/T {none_wins}/{none_losses}/{} · mean {none_mean:+.5} · LB95 {none_lb95:+.5} (no inference at n<2 — disclosed)",
        a1_none.iter().filter(|b| **b).count(),
        none_n,
        a0_none.iter().filter(|b| **b).count(),
        none_n,
        none_n - none_wins - none_losses
    );
    println!("seat: picks {n_pick} · abstains {n_abstain} · stood-other {n_stood_other} · wall p50 {seat_p50:.2} ms (budget ≤ 50 ms) max {seat_max:.2} ms");
    if !seat_us.is_empty() {
        let mut us = seat_us.clone();
        us.sort_unstable();
        println!(
            "seat receipt µs: p50 {} max {}",
            us[us.len() / 2],
            us[us.len() - 1]
        );
    }
    if a0_p50 > 0.0 {
        println!(
            "A0 verify-in-order wall (n={} subsample): p50 {a0_p50:.1} ms — the seat's cost leg",
            a0_cost_ms.len()
        );
    }
    let superiority = dec_lb95 > 0.0;
    let ni_quality = dec_lb95 > -0.02;
    let ni_cost = a0_p50 <= 0.0 || seat_p50 < a0_p50;
    println!(
        "VERDICT: {}",
        if superiority {
            "SEAT WINS (superiority: Holm m=1, paired LB95 > 0) — arm the heal row by manifest edit (T5, OWNER-GATED — this bench prints, never edits)"
        } else if ni_quality && ni_cost {
            "SEAT WINS (non-inferiority at lower cost: paired LB95 > −0.02 AND seat p50 below the A0 verify-in-order wall) — arm the heal row by manifest edit (T5, OWNER-GATED — this bench prints, never edits)"
        } else {
            "DECLINE — the seat stays empty; decline is a correct answer (record the negative)"
        }
    );
    if n_dropped_drift * 20 > n_scored + n_dropped_drift {
        eprintln!("⚠ READ INVALID: pool drift exceeded 5% of scored rows");
        std::process::exit(1);
    }
}
