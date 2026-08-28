//! Plan 031 Phase A (A3) — the trainer-window LOCAL drain (T2.0's local half).
//!
//! Productizes the Bench 049 harness (`fixer_regate`) into the daily loop
//! Proposal 005 §The daily trainer window specifies:
//!
//! ```text
//! TrajStore miss backlog → dedup + priority → modelless pre-pass
//!   → redaction gate (every span, own-repo origin; all-blocked = exit 4)
//!   → one-call smoke (~50-120 n) → LLM (groq-first free tier → CF fallback
//!      via `LlmRouter`; the USD cap guards the CF leg inside generate —
//!      Issue 051: the free tier absorbs the window before any neuron burns)
//!   → strict-keep verify (score_case: clippy oracle + contract + behavioural
//!      vetoes — the fixer_regate machinery, reused not rewritten)
//!   → Certified trajectories + the transparency ledger row
//! ```
//!
//! **NEVER-WRONG by construction:** the drain never writes to any repo
//! file. Accepted fixes land as Certified trajectory rows (future corpus
//! candidates); rejected fixes burn neurons and nothing else. A fix reaches
//! a tree only through the human review path (the sweeper's review branches
//! / corpus authoring), never directly from here.
//!
//! **Priority formula (deterministic, documented):**
//!
//! ```text
//! eligible  = traj rows with fail_count > 0 AND tier != Withdrawn
//!            (Withdrawn rows had 3 strikes — re-entry needs a fresh pattern;
//!             the drain's own failures feed the same counter, so a span the
//!             LLM keeps failing withdraws itself after 3 windows)
//! priority  = fail_count × gap_weight
//! gap_weight = 2.0 when the lint has NO corpus entry (novel pattern — fills
//!              a real rule hole) · 1.0 otherwise (the recorded fix failed)
//! order     = never-healed first (success_count == 0 — the proposal's
//!             "first-heal-of-pattern first"), then priority desc, then
//!             (lint, shape_hash) asc as the deterministic tie-break
//! dedup     = identical shape text under two lint keys keeps ONE entry
//!             (the highest-ordered one)
//! ```
//!
//! **Qwen3 gates** (Proposal 005 caveat #18, carried by the model pivot):
//! - completion budget sized for reasoning + visible output (`--max-new`,
//!   default 1024 — thinking tokens bill as completion);
//! - `finish_reason == "length"` with null content (`REASONING_OVERFLOW`) —
//!   ONE budget-escalated retry (2× `--max-new`), then a HARD per-span
//!   decline; never a retry loop into burn;
//! - `/no_think` for qwen3-family ids + BOTH response schemas parsed:
//!   already inside `CfClient` (verified — nothing duplicated here);
//! - a one-call smoke (~50-120 n) before every window; a failed smoke
//!   aborts BEFORE any backlog burn (exit 5).
//!
//! The USD daily cap (`src/llm_spend.rs`, Issue 046 — reserve-then-reconcile
//! inside `CfClient::generate`) and the 10k-neuron/day allocation are NOT
//! duplicated here. The cap-reached verdict is terminal for the UTC day:
//! the drain stops cleanly and still writes its ledger row.
//!
//! Run:
//!
//! ```sh
//! # offline queue review — no network, CI-able
//! cargo run --release --features "cf_workers_ai,groq_first" --bin kat_drain -- --dry-run
//!
//! # the daily window (needs CF_API_TOKEN + CF_ACCOUNT_ID; GROQ_API_KEY arms
//! # the free tier — the groq-first ladder absorbs the window before any CF
//! # neuron burns; ~25-40 fix-pairs of free allocation at realistic sizes)
//! cargo run --release --features "cf_workers_ai,groq_first" --bin kat_drain -- --limit 25
//! ```

use std::io::Write as _;
use std::path::PathBuf;

use riir_clippy::cf_workers_ai::{
    self, ArmSummary, CaseScore, CfUsageToday, MissClass, RegateHarness, Sample, SampleSource,
    REASONING_OVERFLOW,
};
use riir_clippy::domain::HealEntry as _;
use riir_clippy::groq::router::{LlmRouter, Provider};
use riir_clippy::kat_redact::{GateDecision, RedactionGate, RedactionLog, SpanOrigin};
use riir_clippy::traj_store::{shape_hash, TrajRowDoc, TrajStore};
use riir_clippy::EvidenceTier;

/// The daily smoke probe (caveat #18(d)): one call before any backlog burn —
/// the 7.8k-neuron incident rule generalized from model-change probe to
/// daily probe. A failed smoke aborts the window (exit 5).
const SMOKE_PROMPT: &str = "Reply with exactly: OK";
/// Smoke completion budget — content is one word; thinking is suppressed by
/// `CfClient`'s `/no_think` for qwen3-family ids (and billed as completion
/// tokens when a model reasons anyway — still cheap at this size).
const SMOKE_MAX_TOKENS: usize = 32;
/// Default per-window queue cap — Proposal 005's capacity honesty: ≈25-40
/// verified fix-pairs/day at realistic ~2K-in/500-out+thinking sizes. The
/// real backlog is supply-limited long before budget in steady state.
const DEFAULT_LIMIT: usize = 25;
/// Default completion budget, sized for REASONING + visible output (never
/// visible-only — the thinking-model starvation class `parse_chat_result`
/// classifies as `REASONING_OVERFLOW`).
const DEFAULT_MAX_NEW: usize = 1024;
/// Per-span attempt cap (overflow escalates the budget once: attempt 1 at
/// `--max-new`, attempt 2 at 2×; transport errors retry within the same
/// cap). At the cap the span is hard-declined — never a loop into burn.
const DEFAULT_RETRY_CAP: usize = 2;
/// Exit code: the redaction gate blocked every span (the `fixer_regate`
/// precedent — running on zero samples is pointless, so it fails loud).
const EXIT_ALL_BLOCKED: i32 = 4;
/// Exit code: the pre-window smoke failed — abort before ANY backlog burn.
const EXIT_SMOKE_FAILED: i32 = 5;
/// The drain ledger-row schema (the `arm: "DRAIN"` row's drain-specific
/// fields; the shared cost shape stays identical to `fixer_regate`'s rows).
const DRAIN_ROW_VERSION: u32 = 1;
/// The `CapReached` display prefix (llm_spend) — the terminal-for-today
/// verdict the drain must honor by stopping, never retrying.
const CAP_REACHED_PREFIX: &str = "LLM daily cap reached";

// ── Args ───────────────────────────────────────────────────────────────────

struct Args {
    dry_run: bool,
    limit: usize,
    max_new: usize,
    retry_cap: usize,
    few_shots: usize,
    repo_root: PathBuf,
    out: Option<PathBuf>,
    policy: Option<PathBuf>,
    model: Option<String>,
    behav_dir: PathBuf,
}

impl Args {
    fn parse() -> Args {
        let mut a = Args {
            dry_run: false,
            limit: DEFAULT_LIMIT,
            max_new: DEFAULT_MAX_NEW,
            retry_cap: DEFAULT_RETRY_CAP,
            few_shots: 3,
            repo_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            out: None,
            policy: None,
            model: None,
            behav_dir: std::env::temp_dir().join("kat_drain_behav"),
        };
        let mut argv = std::env::args().skip(1);
        while let Some(arg) = argv.next() {
            let mut val = || {
                argv.next()
                    .unwrap_or_else(|| panic!("missing value for {arg}"))
            };
            match arg.as_str() {
                "--dry-run" => a.dry_run = true,
                "--limit" => a.limit = val().parse().expect("--limit must be an integer"),
                "--max-new" => a.max_new = val().parse().expect("--max-new must be an integer"),
                "--retry-cap" => {
                    a.retry_cap = val().parse().expect("--retry-cap must be an integer");
                }
                "--few-shots" => {
                    a.few_shots = val().parse().expect("--few-shots must be an integer");
                }
                "--repo-root" => a.repo_root = PathBuf::from(val()),
                "--out" => a.out = Some(PathBuf::from(val())),
                "--policy" => a.policy = Some(PathBuf::from(val())),
                "--model" => a.model = Some(val()),
                "--behav-dir" => a.behav_dir = PathBuf::from(val()),
                other => {
                    eprintln!("unknown arg: {other}");
                    std::process::exit(2);
                }
            }
        }
        if a.retry_cap == 0 {
            eprintln!("--retry-cap must be >= 1 (every span gets at least one attempt)");
            std::process::exit(2);
        }
        a
    }
}

// ── The backlog (pure — unit-tested) ───────────────────────────────────────

/// One drain-queue entry.
#[derive(Debug, Clone, PartialEq)]
struct BacklogItem {
    lint: String,
    shape: String,
    fail_count: u32,
    success_count: u32,
    off_corpus: bool,
    priority: f32,
}

/// `priority = fail_count × gap_weight` (gap 2.0 for off-corpus lints — a
/// novel pattern fills a real rule hole; 1.0 for a failed in-corpus fix).
#[allow(clippy::cast_precision_loss)] // trajectory counts are ~10^2
fn priority(row: &TrajRowDoc, off_corpus: bool) -> f32 {
    let gap = if off_corpus { 2.0 } else { 1.0 };
    row.fail_count as f32 * gap
}

/// Build the ordered, deduped queue from the store's rows.
///
/// `is_known_rule` answers "does the clippy corpus carry an entry for this
/// lint" (the gap-weight axis). Pure: the tests pin every ordering rule.
fn build_backlog(
    rows: Vec<TrajRowDoc>,
    is_known_rule: impl Fn(&str) -> bool,
    limit: usize,
) -> Vec<BacklogItem> {
    let mut items: Vec<BacklogItem> = rows
        .into_iter()
        // Misses only: at least one recorded failure. Withdrawn rows had
        // their 3 strikes (absorbing tier) — excluded until a fresh pattern.
        .filter(|r| r.fail_count > 0 && EvidenceTier::from_u8(r.tier) != EvidenceTier::Withdrawn)
        .map(|r| {
            let off_corpus = !is_known_rule(&r.lint_key);
            let priority = priority(&r, off_corpus);
            BacklogItem {
                lint: r.lint_key,
                shape: r.shape,
                fail_count: r.fail_count,
                success_count: r.success_count,
                off_corpus,
                priority,
            }
        })
        .collect();
    // never-healed first, then priority desc, then the deterministic
    // (lint, shape_hash) tie-break — identical inputs, identical queue.
    items.sort_by(|a, b| {
        let never_a = a.success_count == 0;
        let never_b = b.success_count == 0;
        never_b
            .cmp(&never_a)
            .then(b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.lint.cmp(&b.lint))
            .then_with(|| shape_hash(&a.shape).cmp(&shape_hash(&b.shape)))
    });
    // Dedup identical shape text across lint keys (one entry per span — the
    // highest-ordered occurrence). Retain order while deduping.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    items.retain(|i| seen.insert(shape_hash(&i.shape)));
    items.truncate(limit);
    items
}

// ── Generate-error classification (pure — unit-tested) ─────────────────────

/// What a failed `generate` means for the drain loop.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AttemptPlan {
    /// The USD daily cap tripped — terminal for the UTC day: stop the drain.
    StopDay,
    /// Hard-decline this span (overflow at the budget cap, or a transport
    /// error that outlived the retry cap) — next span.
    DeclineSpan,
    /// A bounded retry, escalating the completion budget (the overflow
    /// remedy its own error text names: "raise max_tokens").
    RetryEscalate,
}

fn classify_error(msg: &str) -> AttemptPlan {
    if msg.starts_with(CAP_REACHED_PREFIX) {
        AttemptPlan::StopDay
    } else if msg.starts_with(REASONING_OVERFLOW) {
        AttemptPlan::RetryEscalate
    } else {
        AttemptPlan::DeclineSpan
    }
}

// ── The drain ──────────────────────────────────────────────────────────────

/// One prepared queue entry through the harness (post-classification).
struct Prepared {
    item: BacklogItem,
    sample: Sample,
    miss: MissClass,
    prompt_prepass: String,
}

fn main() {
    let args = Args::parse();
    let t0 = std::time::Instant::now();
    let run_id = {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0u128, |d| d.as_nanos());
        format!("drain-{t:x}-{}", std::process::id())
    };

    // ── The backlog: TrajStore at the repo root (the miss source).
    let (store, bad_rows) = match TrajStore::open(&args.repo_root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("FATAL: cannot open the trajectory store: {e}");
            std::process::exit(1);
        }
    };
    if bad_rows > 0 {
        eprintln!("[drain] store: {bad_rows} undecodable row(s) skipped");
    }
    let rows = store.dump();

    // ── The harness: corpus (gap weight) + pipeline (miss class) + matcher
    // (few-shots). One build, reused for the whole window.
    let mut harness = match RegateHarness::new() {
        Ok(h) => h,
        Err(e) => {
            eprintln!("FATAL: harness init failed: {e}");
            std::process::exit(1);
        }
    };
    let corpus_rules: std::collections::HashSet<&str> = harness
        .corpus
        .entries
        .iter()
        .map(|e| e.rule_id())
        .collect();
    let queue = build_backlog(rows, |lint| corpus_rules.contains(lint), args.limit);
    eprintln!(
        "[drain] backlog: {} queued (limit {}, store {} row(s)) — priority = fail_count × \
         gap(2.0 off-corpus / 1.0 in-corpus), never-healed first",
        queue.len(),
        args.limit,
        store.len()
    );

    // ── Modelless pre-pass (the L4 reachability contract): classify each
    // queued span; only the classes the modelless seam cannot serve reach
    // the LLM. The candidate + few-shots ride the arm-B prompt shape.
    let mut prepared: Vec<Prepared> = Vec::new();
    let mut modelless_served = 0usize;
    for item in queue {
        let lint = item.lint.clone();
        let shape = item.shape.clone();
        let (miss, modelless) = harness.classify(&lint, &shape);
        if !miss.is_reachable_miss() {
            eprintln!(
                "[drain] skip {lint}: modelless seam serves it ({miss:?}) — the L4 contract \
                 keeps the LLM off non-misses"
            );
            modelless_served += 1;
            continue;
        }
        let fs = harness.few_shots(&shape, args.few_shots);
        let sample = Sample {
            id: format!("traj:{lint}:{}", shape_hash(&shape)),
            lints: vec![lint.clone()],
            module: shape.clone(),
            expected: None,
            source: SampleSource::Reachable,
        };
        let prompt_prepass = cf_workers_ai::prompt_prepass(&shape, modelless.as_deref(), &fs);
        prepared.push(Prepared { item, sample, miss, prompt_prepass });
    }

    // ── Redaction gate: EVERY span, before any dispatch (own-repo origin —
    // the vetted class; these spans came from our own healing runs).
    let prepared_count = prepared.len();
    let loaded = RedactionGate::discover(args.policy.as_deref());
    let log = RedactionLog::open(&loaded.root());
    let mut kept: Vec<Prepared> = Vec::with_capacity(prepared.len());
    let mut blocked = 0usize;
    for mut p in prepared {
        let decision = loaded.gate.check(SpanOrigin::OwnRepo, None, &p.sample.module);
        let fp = blake3::hash(p.sample.module.as_bytes()).to_hex();
        let fp12 = &fp.to_string()[..12];
        if let Err(e) = log.record(&decision, SpanOrigin::OwnRepo, &p.sample.id, fp12) {
            eprintln!("[drain] redaction log write failed: {e}");
        }
        match decision {
            GateDecision::Allow => kept.push(p),
            GateDecision::Redacted { payload, .. } => {
                eprintln!(
                    "[drain] REDACTED {} (findings replaced; policy allows redacted)",
                    p.sample.id
                );
                p.sample.module = payload;
                // The prompt embeds the ORIGINAL module — rebuild it on the
                // redacted text so the egress payload matches the gate's.
                p.item.shape = p.sample.module.clone();
                kept.push(p);
            }
            GateDecision::Blocked { reason, findings } => {
                blocked += 1;
                let kinds: Vec<&str> = findings.iter().map(|f| f.kind.tag()).collect();
                let detail = if kinds.is_empty() {
                    String::new()
                } else {
                    format!(" [{kinds:?}]")
                };
                eprintln!("[drain] BLOCKED {} — {reason}{detail}", p.sample.id);
            }
        }
    }
    eprintln!("[drain] redaction gate: {} allowed, {blocked} blocked", kept.len());
    if kept.is_empty() {
        if prepared_count == 0 {
            // The healthy state: the backlog (or the modelless pre-pass)
            // produced nothing to drain. Not an error — exit 0, the cron
            // row records a clean no-op.
            eprintln!("[drain] queue empty — nothing to drain");
            return;
        }
        eprintln!(
            "FATAL: redaction gate blocked every span (queue was non-empty) — nothing to drain"
        );
        std::process::exit(EXIT_ALL_BLOCKED);
    }

    // ── Dry run: the printed queue IS the deliverable (no network, CI-able).
    if args.dry_run {
        println!("═ kat_drain DRY — queue + priorities (no LLM calls) ═");
        println!("{:<34} {:<13} {:>5} {:>5} {:>8}  {:<10} prompt-b3-12", "id", "miss", "fail", "succ", "prio", "off-corpus");
        for p in &kept {
            let h = blake3::hash(p.prompt_prepass.as_bytes()).to_hex();
            println!(
                "{:<34} {:<13} {:>5} {:>5} {:>8.1}  {:<10} {}",
                p.sample.id,
                format!("{:?}", p.miss),
                p.item.fail_count,
                p.item.success_count,
                p.item.priority,
                p.item.off_corpus,
                &h.to_string()[..12],
            );
        }
        println!(
            "\n{} queued → {} reach the LLM (modelless pre-pass kept {modelless_served} off), \
             {blocked} blocked by redaction; run without --dry-run (CF_API_TOKEN + CF_ACCOUNT_ID) \
             to drain",
            kept.len() + modelless_served + blocked,
            kept.len(),
        );
        return;
    }

    // ── The LLM window (Issue 051: groq-first ladder — the free tier
    // absorbs the window; CF remains the metered fallback its USD cap
    // guards). GROQ_API_KEY absent → pure CF, byte-identical to pre-051.
    let mut llm = match LlmRouter::from_env(args.model.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(3);
        }
    };
    if llm.groq_enabled() {
        eprintln!(
            "[drain] groq-first armed (model {}, free tier absorbs the window; CF fallback \
             stays cap-guarded)",
            llm.groq_model().unwrap_or("?")
        );
    } else {
        eprintln!("[drain] groq not armed (GROQ_API_KEY unset) — pure CF window");
    }
    let (price_in, price_out) = riir_clippy::llm_spend::price_for(llm.model_id())
        .map_or((0.0, 0.0), |p| (p.input_per_m, p.output_per_m));

    // Smoke (caveat #18(d)): one cheap call BEFORE the backlog. A failure
    // aborts the window — the 169-billed-request incident class this probe
    // exists to kill. The probe rides the ladder (groq-first), so a dead
    // free tier is surfaced before the queue starts.
    let smoke = llm.generate(SMOKE_PROMPT, SMOKE_MAX_TOKENS);
    let smoke_provider = smoke
        .as_ref()
        .map_or("none", |g| g.provider.as_str())
        .to_string();
    match &smoke {
        Ok(g) => eprintln!(
            "[drain] smoke OK (via {}, {}ms{}; window opens)",
            g.provider.as_str(),
            g.latency_ms,
            if g.provider == Provider::Groq {
                String::new()
            } else {
                format!(", {:.1} neurons", g.neurons)
            }
        ),
        Err(e) => {
            eprintln!("FATAL: smoke failed ({e}) — aborting BEFORE any backlog burn");
            append_ledger(
                &args,
                llm.model_id(),
                &ArmSummary::default(),
                None,
                Some(llm.cf().spend_state()),
                &DrainStats {
                    run_id: run_id.clone(),
                    queued: kept.len(),
                    llm_scored: 0,
                    declines: 0,
                    overflow_declines: 0,
                    smoke_neurons: 0.0,
                    smoke_provider: smoke_provider.clone(),
                    smoke_failed: true,
                    stopped_early: true,
                    wall_ms: t0.elapsed().as_millis(),
                },
                &llm,
            );
            std::process::exit(EXIT_SMOKE_FAILED);
        }
    }
    let smoke_neurons = smoke.as_ref().map_or(0.0, |g| g.neurons);

    let verifier = std::mem::replace(&mut harness.verifier, riir_clippy::ClippyVerifier::new());
    std::fs::create_dir_all(&args.behav_dir).ok();
    let mut out = args
        .out
        .as_ref()
        .and_then(|p| std::fs::File::create(p).ok());
    let mut summary = ArmSummary::default();
    let (mut declines, mut overflow_declines, mut llm_scored, mut stopped_early) =
        (0usize, 0usize, 0usize, false);

    for p in &kept {
        eprintln!("[drain] {} …", p.sample.id);
        let mut budget = args.max_new;
        let mut attempts = 0usize;
        let gen = loop {
            attempts += 1;
            match llm.generate(&p.prompt_prepass, budget) {
                Ok(g) => break Some(g),
                Err(e) => match classify_error(&e) {
                    AttemptPlan::StopDay => {
                        eprintln!("[drain] {e}");
                        declines += 1;
                        stopped_early = true;
                        break None;
                    }
                    AttemptPlan::RetryEscalate if attempts < args.retry_cap => {
                        eprintln!(
                            "[drain] reasoning-overflow at {budget} tokens — one escalated \
                             retry at {} (cap {}/{})",
                            budget * 2,
                            attempts,
                            args.retry_cap
                        );
                        budget *= 2;
                        continue;
                    }
                    AttemptPlan::RetryEscalate => {
                        eprintln!("[drain] hard-decline {}: {e}", p.sample.id);
                        declines += 1;
                        overflow_declines += 1;
                        break None;
                    }
                    AttemptPlan::DeclineSpan => {
                        if attempts < args.retry_cap {
                            continue; // bounded transport retry, same budget
                        }
                        eprintln!("[drain] decline {} after {attempts} attempt(s): {e}", p.sample.id);
                        declines += 1;
                        break None;
                    }
                },
            }
        };
        if stopped_early {
            break;
        }
        let Some(gen) = gen else {
            // A decline is a MODEL/budget verdict, not a fix-quality verdict —
            // it never feeds the trajectory counters (a span must not withdraw
            // because the model starved, only because the FIX failed).
            let score = cf_workers_ai::score_case(&verifier, &p.sample, "", None);
            summary.add(&score, true, 0, 0, price_in, price_out);
            record_case(&mut out, p, "decline", "", &score);
            continue;
        };
        llm_scored += 1;
        let score =
            cf_workers_ai::score_case(&verifier, &p.sample, &gen.text, Some(&args.behav_dir));
        // Accounting honesty (Issue 051): groq tokens are FREE — they count
        // toward the quality summary but contribute ZERO to the CF cost
        // estimate. The groq usage rides the ledger's `providers.groq`
        // block (see append_ledger), never the `client` block.
        let (tok_in, tok_out) = if gen.provider == Provider::Cloudflare {
            (gen.prompt_tokens, gen.completion_tokens)
        } else {
            (0, 0)
        };
        summary.add(&score, true, tok_in, tok_out, price_in, price_out);
        // NEVER-WRONG: accepted → a Certified trajectory (the ONLY write the
        // drain performs); rejected → a failure verdict (three failures and
        // the span withdraws itself out of the queue — the self-limiting
        // property). Nothing is written to any repo file, ever.
        let outcome_ok = if score.keep_strict {
            if let Some((merged, _)) = cf_workers_ai::apply_edit(&p.sample.module, &gen.text, true) {
                    let _ = store.record(&p.item.lint, &p.item.shape, &merged, true, true, &run_id);
                    true
                } else {
                    // Unreachable in practice (keep_strict implies a merge) —
                    // record the failure rather than fabricating a keep.
                    let _ = store.record(&p.item.lint, &p.item.shape, &gen.text, false, false, &run_id);
                    false
                }
        } else {
            let _ = store.record(&p.item.lint, &p.item.shape, &gen.text, false, false, &run_id);
            false
        };
        record_case(&mut out, p, if outcome_ok { "kept" } else { "rejected" }, &gen.text, &score);
        println!(
            "  {:<34} keep_s={} f {}→{} attempts={attempts} {:.0}ms {}+{}tok",
            p.sample.id,
            score.keep_strict,
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
    }

    println!("\n═ drain summary (window {}) ═", if stopped_early { "STOPPED EARLY (cap)" } else { "complete" });
    println!("| arm | n | EM | parses | merge | keep_o/scoreable | keep_s | vetoed | divergent | regressions | $ |");
    println!("{}", summary.markdown_row("DRAIN"));
    println!("  queued {} · llm-scored {llm_scored} · declined {declines} (overflow {overflow_declines})", kept.len());
    let server = llm.cf().fetch_usage_today().ok();
    if let Some(u) = &server {
        println!(
            "  server meter (today, this model): {} req · {:.2} neurons ≈ ${:.5}",
            u.requests,
            u.neurons,
            u.neurons / 1000.0 * 0.011
        );
    }
    let gstats = llm.stats.clone();
    if gstats.ok > 0 {
        println!(
            "  groq absorbed {} call(s), {}+{} tok (est ${:.5} of CF avoided; rate-limited {})",
            gstats.ok,
            gstats.prompt_tokens,
            gstats.completion_tokens,
            riir_clippy::groq::router::avoided_usd(
                gstats.prompt_tokens,
                gstats.completion_tokens,
                price_in,
                price_out
            ),
            gstats.rate_limited,
        );
    }
    println!("  {}", llm.cf().spend_summary());
    eprintln!("[drain] store events → {}", store.root.display());
    append_ledger(
        &args,
        llm.model_id(),
        &summary,
        server.as_ref(),
        Some(llm.cf().spend_state()),
        &DrainStats {
            run_id,
            queued: kept.len(),
            llm_scored,
            declines,
            overflow_declines,
            smoke_neurons,
            smoke_provider,
            smoke_failed: false,
            stopped_early,
            wall_ms: t0.elapsed().as_millis(),
        },
        &llm,
    );
}

// ── JSONL writers (the fixer_regate row conventions) ───────────────────────

/// Drain-specific transparency fields (the shared cost shape stays identical
/// to `fixer_regate`'s `.benchmarks/llm_cost_ledger.jsonl` rows — same file,
/// `arm: "DRAIN"`).
struct DrainStats {
    run_id: String,
    queued: usize,
    llm_scored: usize,
    declines: usize,
    overflow_declines: usize,
    smoke_neurons: f64,
    /// Which provider served the smoke ("groq" / "cloudflare" / "none").
    smoke_provider: String,
    smoke_failed: bool,
    stopped_early: bool,
    wall_ms: u128,
}

fn append_ledger(
    args: &Args,
    model: &str,
    summary: &ArmSummary,
    server: Option<&CfUsageToday>,
    spend: Option<&riir_clippy::llm_spend::SpendState>,
    drain: &DrainStats,
    llm: &LlmRouter,
) {
    let path = std::env::var("RIIR_LLM_COST_LEDGER")
        .map_or_else(
            |_| args.repo_root.join(".benchmarks/llm_cost_ledger.jsonl"),
            PathBuf::from,
        );
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (price_in, price_out) = riir_clippy::llm_spend::price_for(model)
        .map_or((0.0, 0.0), |p| (p.input_per_m, p.output_per_m));
    let gstats = &llm.stats;
    let avoided = riir_clippy::groq::router::avoided_usd(
        gstats.prompt_tokens,
        gstats.completion_tokens,
        price_in,
        price_out,
    );
    // Issue 051: the free tier's absorption rides `providers.groq` — the
    // dashboard's card reads it; legacy rows without the key are unaffected.
    let mut providers = riir_clippy::groq::router::providers_json(
        gstats,
        avoided,
        llm.groq_model(),
    );
    if let Some(reason) = llm.disable_reason() {
        providers["groq"]["disabled_reason"] = serde_json::json!(reason);
    }
    let groq_note = if gstats.ok > 0 {
        format!(
            "groq-first absorbed {}/{} call(s) +{}+{} tok (est ${:.5} CF avoided)",
            gstats.ok,
            gstats.calls,
            gstats.prompt_tokens,
            gstats.completion_tokens,
            avoided
        )
    } else if gstats.calls > 0 {
        "groq armed but served nothing this window".to_string()
    } else {
        "pure CF window (groq not armed)".to_string()
    };
    let row = serde_json::json!({
        "ts_unix": ts,
        "arm": "DRAIN",
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
        "providers": providers,
        "drain": {
            "row_version": DRAIN_ROW_VERSION,
            "run_id": drain.run_id,
            "queued": drain.queued,
            "llm_scored": drain.llm_scored,
            "declines": drain.declines,
            "overflow_declines": drain.overflow_declines,
            "keeps": summary.keep_strict,
            "smoke_neurons": drain.smoke_neurons,
            "smoke_provider": drain.smoke_provider,
            "smoke_failed": drain.smoke_failed,
            "stopped_early": drain.stopped_early,
            "wall_ms": u64::try_from(drain.wall_ms).unwrap_or(u64::MAX),
        },
        "kat": serde_json::Value::Null,
        "note": format!("kat null until Phase 1 rail; Plan 031 A3 trainer-window drain (never-wrong: keeps are Certified trajectories, rejects never land); {groq_note}"),
    });
    let mut f = match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[drain] cost ledger write skipped ({}: {e})", path.display());
            return;
        }
    };
    if let Err(e) = writeln!(f, "{row}") {
        eprintln!("[drain] cost ledger write failed: {e}");
    } else {
        println!("  cost ledger: {} ← 1 row", path.display());
    }
}

/// One per-case JSONL row (the `--out` diagnostic stream — same shape family
/// as `fixer_regate`'s `record_case`, with the drain's verdict class).
fn record_case(
    out: &mut Option<std::fs::File>,
    p: &Prepared,
    verdict: &str,
    gen: &str,
    score: &CaseScore,
) {
    if let Some(f) = out {
        let row = serde_json::json!({
            "id": p.sample.id,
            "arm": "DRAIN",
            "verdict": verdict,
            "lint": p.item.lint,
            "miss_class": format!("{:?}", p.miss),
            "off_corpus": p.item.off_corpus,
            "fail_count": p.item.fail_count,
            "priority": p.item.priority,
            "prompt_blake3_12": &blake3::hash(p.prompt_prepass.as_bytes()).to_hex().as_str()[..12],
            "gen_chars": gen.len(),
            "keep_strict": score.keep_strict,
            "findings_before": score.findings_before,
            "findings_after": if score.findings_after == usize::MAX { None } else { Some(score.findings_after) },
        });
        let _ = writeln!(f, "{row}");
    }
}

// ── Tests (offline; pinned by ci_feature_guard.sh Layer 5c) ────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn row(lint: &str, shape: &str, fail: u32, success: u32, tier: u8) -> TrajRowDoc {
        TrajRowDoc {
            lint_key: lint.to_string(),
            shape: shape.to_string(),
            fix: "fixed".to_string(),
            direction: vec![0.0; 8],
            success_count: success,
            fail_count: fail,
            consecutive_fails: fail.min(3),
            tier,
            last_run_id: String::new(),
            last_seen_at: 0,
            elo: 1200.0,
            span_elo: 1200.0,
            component: None,
        }
    }

    const CERTIFIED: u8 = 0;
    const HEURISTIC: u8 = 1;
    const WITHDRAWN: u8 = 2;

    #[test]
    fn backlog_takes_only_rows_with_failures() {
        let q = build_backlog(
            vec![
                row("lint_a", "fn a() {}", 0, 3, CERTIFIED),
                row("lint_b", "fn b() {}", 1, 0, HEURISTIC),
            ],
            |_| true,
            10,
        );
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].lint, "lint_b");
    }

    fn backlog_with(rows: Vec<TrajRowDoc>) -> Vec<BacklogItem> {
        build_backlog(rows, |_| true, 10)
    }

    #[test]
    fn withdrawn_rows_are_excluded_until_a_fresh_pattern() {
        let q = backlog_with(vec![row("lint_a", "fn a() {}", 9, 0, WITHDRAWN)]);
        assert!(q.is_empty());
    }

    #[test]
    fn never_healed_first_then_priority_desc() {
        // High-priority but previously-healed must NOT outrank never-healed.
        let q = backlog_with(vec![
            row("lint_z", "fn z() {}", 8, 2, HEURISTIC), // prio 8, healed
            row("lint_a", "fn a() {}", 1, 0, HEURISTIC), // prio 1, never healed
        ]);
        assert_eq!(q[0].lint, "lint_a");
        assert_eq!(q[1].lint, "lint_z");
    }

    #[test]
    fn off_corpus_doubles_the_priority() {
        let q = build_backlog(
            vec![
                row("known_rule", "fn k() {}", 2, 0, HEURISTIC),
                row("novel_rule", "fn n() {}", 2, 0, HEURISTIC),
            ],
            |lint| lint == "known_rule",
            10,
        );
        // Equal fail counts: novel (2×2=4) outranks known (2×1=2).
        assert_eq!(q[0].lint, "novel_rule");
        assert_eq!(q[0].priority, 4.0);
        assert_eq!(q[1].priority, 2.0);
        assert!(q[0].off_corpus);
        assert!(!q[1].off_corpus);
    }

    #[test]
    fn identical_shapes_dedup_to_one_entry() {
        let q = backlog_with(vec![
            row("lint_a", "fn same() {}", 1, 0, HEURISTIC),
            row("lint_b", "fn same() {}", 5, 0, HEURISTIC),
        ]);
        assert_eq!(q.len(), 1);
        // The higher-priority occurrence wins (lint_b: fail 5).
        assert_eq!(q[0].lint, "lint_b");
    }

    #[test]
    fn tie_break_is_deterministic_on_lint_then_shape() {
        let rows = vec![
            row("lint_b", "fn s1() {}", 2, 0, HEURISTIC),
            row("lint_a", "fn s2() {}", 2, 0, HEURISTIC),
        ];
        let q1 = backlog_with(rows.clone());
        let q2 = backlog_with(rows);
        assert_eq!(q1, q2);
        assert_eq!(q1[0].lint, "lint_a");
    }

    #[test]
    fn limit_truncates_after_ordering() {
        let rows: Vec<TrajRowDoc> = (0..10)
            .map(|i| row("lint", &format!("fn s{i}() {{}}"), 1, 0, HEURISTIC))
            .collect();
        let q = backlog_with(rows);
        assert_eq!(q.len(), 10);
        let rows: Vec<TrajRowDoc> = (0..10)
            .map(|i| row("lint", &format!("fn s{i}() {{}}"), 1, 0, HEURISTIC))
            .collect();
        let q = build_backlog(rows, |_| true, 3);
        assert_eq!(q.len(), 3);
    }

    #[test]
    fn cap_reached_classifies_stop_day() {
        assert_eq!(
            classify_error(
                "LLM daily cap reached: $1.0000 spent + $0.0000 reserved + $0.0013 wanted > \
                 $1.00 cap — hard-declining for the rest of the UTC day"
            ),
            AttemptPlan::StopDay
        );
    }

    #[test]
    fn reasoning_overflow_classifies_retry_then_decline_boundary() {
        assert_eq!(
            classify_error("reasoning-overflow: content null with finish_reason=length"),
            AttemptPlan::RetryEscalate
        );
    }

    #[test]
    fn transport_errors_decline_the_span() {
        assert_eq!(
            classify_error("CF request failed: network error"),
            AttemptPlan::DeclineSpan
        );
    }

    #[test]
    fn smoke_constants_stay_cheap() {
        // The probe must stay in the ~50-120 n band — a fat smoke defeats
        // its own purpose (the window pays for it every day).
        const {
            assert!(SMOKE_MAX_TOKENS <= 64);
            assert!(SMOKE_PROMPT.len() < 64);
        };
    }

    #[test]
    fn default_budget_is_sized_for_reasoning() {
        // Caveat #18(a): visible-only budgets (the 128 the POC used) starve
        // thinking models — the drain default must be ≥ 512.
        const {
            assert!(DEFAULT_MAX_NEW >= 512);
        };
    }
}
