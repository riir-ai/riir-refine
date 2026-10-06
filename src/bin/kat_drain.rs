//! Plan 031 Phase A (A3) — the trainer-window LOCAL drain (T2.0's local half).
//!
//! Productizes the Bench 049 harness (`fixer_regate`) into the daily loop
//! Proposal 005 §The daily trainer window specifies:
//!
//! ```text
//! TrajStore miss backlog → dedup + priority → modelless pre-pass
//!   → redaction gate (every span, own-repo origin; all-blocked = exit 4)
//!   → one-call smoke (~50-120 n) → LLM (free-tier ladder groq → infron →
//!      CF fallback via `LlmRouter`; the USD cap guards the CF leg inside
//!      generate — Issue 051: the free tier absorbs the window before any
//!      neuron burns; Issue 106: infron is the second free leg between
//!      them)
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
//! # ladder + credential check — works on an EMPTY backlog, zero metered
//! # cost (probes the FREE leg only; refuses to spend neurons)
//! cargo run --release --features "cf_workers_ai,groq_first" --bin kat_drain -- --preflight
//!
//! # the daily window — needs AT LEAST ONE provider armed:
//! #   GROQ_API_KEY                  → the fast free leg (~25-40 fix-pairs of
//! #                                    free allocation at realistic sizes)
//! #   INFRON_API_KEY                → the slow free leg (llm.onerouter.pro,
//! #                                    ~33 tok/s, qwen3.8-27b:free — Issue
//! #                                    106; catches groq's 429s before CF)
//! #   CF_API_TOKEN + CF_ACCOUNT_ID  → the metered fallback (USD-capped)
//! # Both armed is the intended daily shape (free tier absorbs the window,
//! # CF catches the overflow). Free-only runs too: exhausted free legs
//! # then end the window cleanly (`free-tier exhausted`), and zero neurons
//! # can burn. Neither armed → a loud refusal naming both vars.
//! cargo run --release --features "cf_workers_ai,groq_first" --bin kat_drain -- --limit 25
//! ```

use std::io::Write as _;
use std::path::PathBuf;

use riir_refine::cf_workers_ai::{
    self, ArmSummary, CaseScore, CfUsageToday, MissClass, RegateHarness, Sample, SampleSource,
    REASONING_OVERFLOW,
};
use riir_refine::domain::HealEntry as _;
use riir_refine::groq::router::{LlmRouter, Provider, FREE_TIER_EXHAUSTED};
use riir_refine::kat_redact::{GateDecision, RedactionGate, RedactionLog, SpanOrigin};
use riir_refine::traj_store::{shape_hash, TrajRowDoc, TrajStore};
use riir_refine::EvidenceTier;

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
    preflight: bool,
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
    fn parse() -> Self {
        let mut a = Self {
            dry_run: false,
            preflight: false,
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
                "--preflight" => a.preflight = true,
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

/// Why the backlog has the size it has — the eligibility census.
///
/// [`build_backlog`] applies TWO independent exclusions and the drain used to
/// report only their SUM ("0 queued"), which cannot tell a HEALTHY store
/// (every row clean) from a STALLED one (real failures exist, but every one
/// of them sits in the absorbing `Withdrawn` tier and will never be queued
/// again). Both print `queue empty — nothing to drain` and exit 0, so the
/// daily window looks green either way. That is the Issue 044 T12 / Issue 051
/// T13 defect class: a green surface hiding real state.
///
/// The census is a pure fold over the rows, so the operator line and the
/// tests read the same numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct BacklogCensus {
    /// Every row in the store.
    total: usize,
    /// `fail_count == 0` — nothing to heal. The healthy majority.
    clean: usize,
    /// `fail_count > 0` AND tier == `Withdrawn`: real, recorded failures the
    /// drain will NEVER queue again (3 strikes, absorbing). A growing count
    /// here is a silent stall, not a clean backlog.
    withdrawn: usize,
    /// `fail_count > 0` AND tier != `Withdrawn` — the queue's source
    /// population (before `--limit` truncation).
    eligible: usize,
}

/// Pure fold: classify every store row into exactly one census bucket.
///
/// The predicates are the SAME two `build_backlog` filters on, deliberately
/// written in the same order — a census that could disagree with the queue it
/// explains would be worse than no census at all.
fn census(rows: &[TrajRowDoc]) -> BacklogCensus {
    let mut c = BacklogCensus {
        total: rows.len(),
        ..BacklogCensus::default()
    };
    for r in rows {
        match () {
            () if r.fail_count == 0 => c.clean += 1,
            () if EvidenceTier::from_u8(r.tier) == EvidenceTier::Withdrawn => c.withdrawn += 1,
            () => c.eligible += 1,
        }
    }
    c
}

/// The operator-facing reason nothing reached the LLM.
///
/// The caller's branch is `prepared_count == 0`, which is reached FIVE
/// different ways — and they are not the same news. Pure so the wording is
/// pinned by tests; these states must never collapse into one string.
///
/// `queued` is the post-`--limit` queue length and `modelless_served` the
/// count the L4 pre-pass absorbed, because `prepared_count == queued -
/// modelless_served`: without both, a window where the modelless seam served
/// EVERY span (the best possible outcome — zero LLM cost) is indistinguishable
/// from an empty backlog, and a `--limit 0` run is indistinguishable from a
/// census bug.
fn empty_queue_reason(
    c: BacklogCensus,
    limit: usize,
    queued: usize,
    modelless_served: usize,
) -> String {
    match () {
        () if c.total == 0 => "the trajectory store is empty — no heal run has recorded a \
             trajectory yet"
            .to_string(),
        // The BEST outcome, and the one most easily misread as a failure: the
        // backlog was real, and the modelless seam served all of it. The L4
        // contract keeps the LLM off non-misses, so zero spend here is the
        // mechanism WORKING.
        () if queued > 0 && modelless_served == queued => format!(
            "not idle — the modelless seam served all {queued} queued span(s) (L4 contract \
             keeps the LLM off non-misses): the backlog was real and was healed at zero LLM \
             cost. Nothing to escalate."
        ),
        // A deliberate operator choice, not a defect.
        () if limit == 0 && c.eligible > 0 => format!(
            "{} eligible row(s) withheld by `--limit 0` — raise the limit to drain them",
            c.eligible
        ),
        // Only NOW is a disagreement genuinely unexplained.
        () if c.eligible > 0 && queued == 0 => format!(
            "BUG: {} row(s) are eligible but none queued (limit {limit}) — the census and \
             build_backlog disagree",
            c.eligible
        ),
        () if c.withdrawn > 0 => format!(
            "STALL, not a clean store: {} of {} row(s) DO carry failures but every one is \
             Withdrawn (3 strikes, absorbing) — re-entry needs a fresh pattern, so the drain \
             will never queue them again. This backlog is withheld, not absent.",
            c.withdrawn, c.total
        ),
        () => format!(
            "healthy: all {} store row(s) are clean (fail_count == 0) — nothing to heal",
            c.total
        ),
    }
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
            .then(b.priority.total_cmp(&a.priority))
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
    // Both terminal verdicts stop the window: the USD cap (paid leg) and an
    // exhausted free tier with no paid leg armed (Issue 051 groq-only).
    if msg.starts_with(CAP_REACHED_PREFIX) || msg.starts_with(FREE_TIER_EXHAUSTED) {
        AttemptPlan::StopDay
    } else if msg.starts_with(REASONING_OVERFLOW) {
        AttemptPlan::RetryEscalate
    } else {
        AttemptPlan::DeclineSpan
    }
}

/// Which provider actually served this window (pure — unit-tested).
///
/// Load-bearing for ledger attribution. The row's `model` field is the CF
/// PRICING BASIS — it prices `usd_avoided` and is resolved even when the CF
/// leg was never armed (Issue 051 T9) — so it is NOT a claim that CF served
/// anything. Without this field a free-only window is indistinguishable from
/// a CF window that happened to cost $0, and the dashboard credits the CF
/// model with samples it never touched: the `glm-4.7-flash 0` mis-render the
/// owner hand-reconciled in `7bdb710`, which T9 turned from an anomaly into a
/// ROUTINE row shape.
///
/// A CF-served SMOKE counts as CF usage even when a free leg served every
/// span — those neurons are real and must not be attributed to the free
/// tier.
fn served_by(
    groq_served: bool,
    infron_served: bool,
    cf_tokens: u64,
    smoke_neurons: f64,
    server_neurons: f64,
) -> &'static str {
    let cf_used = cf_tokens > 0 || smoke_neurons > 0.0 || server_neurons > 0.0;
    match (groq_served, infron_served, cf_used) {
        (true, false, false) => "groq",
        (false, true, false) => "infron",
        (true, true, false) => "groq+infron",
        (false, false, true) => "cloudflare",
        (_, _, true) => "mixed",
        (false, false, false) => "none",
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

/// `--preflight`: arm the ladder, prove the free leg SERVES, exit. Runs
/// before the backlog is even opened, because the whole point is that it
/// works on an EMPTY backlog — the daily window returns at "queue empty"
/// before the router is constructed, so without this an operator has no way
/// to validate credentials or the ladder on a quiet day. That gap is why
/// Issue 051's T7 pre-flight existed only as an `#[ignore]`d unit test.
///
/// **Zero metered cost by construction.** The probe runs ONLY when the free
/// leg is armed. A pure-CF preflight is deliberately refused: it would burn
/// ~50-120 unledgered neurons (this fn writes no ledger row) while telling
/// the operator nothing the daily window's own ledgered smoke does not
/// already report. `.benchmarks/llm_cost_ledger.jsonl` is the usage record
/// of last resort — no path may spend behind its back.
///
/// Exits: 0 armed-and-serving (or CF-only, reported without a call) · 3 no
/// provider armed · 5 the free leg failed its probe.
fn run_preflight(args: &Args) -> ! {
    println!("═ kat_drain --preflight (ladder + credential check, zero metered cost) ═");
    let mut llm = match LlmRouter::from_env(args.model.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(3);
        }
    };
    println!(
        "  groq (free)      : {}",
        match llm.groq_model() {
            Some(m) => format!("ARMED   model {m}"),
            None => "unarmed (GROQ_API_KEY unset)".to_string(),
        }
    );
    println!(
        "  infron (free)    : {}",
        match llm.infron_model() {
            Some(m) => format!("ARMED   model {m}"),
            None => "unarmed (INFRON_API_KEY unset)".to_string(),
        }
    );
    println!(
        "  cloudflare (paid): {}",
        if llm.cf_armed() {
            format!("ARMED   model {}", llm.model_id())
        } else {
            "unarmed (CF_API_TOKEN / CF_ACCOUNT_ID unset)".to_string()
        }
    );
    println!(
        "  billing basis    : {} (prices usd_avoided)",
        llm.model_id()
    );
    if let Some(cf) = llm.cf() {
        println!("  cf spend         : {}", cf.spend_summary());
    }
    if !llm.groq_enabled() && !llm.infron_enabled() {
        println!(
            "\n  free legs unarmed → NO probe sent (a CF probe would burn unledgered \
             neurons).\n  Export GROQ_API_KEY or INFRON_API_KEY to exercise the ladder for free."
        );
        std::process::exit(0);
    }
    match llm.generate(SMOKE_PROMPT, SMOKE_MAX_TOKENS) {
        Ok(g) => {
            println!(
                "\n  probe OK via {} · {}+{} tok · {:.1} neurons · {}ms",
                g.provider.as_str(),
                g.prompt_tokens,
                g.completion_tokens,
                g.neurons,
                g.latency_ms,
            );
            if llm.cf_armed() {
                println!("  verdict: ladder READY (free tiers absorb, CF catches overflow)");
            } else {
                println!(
                    "  verdict: ladder READY (free-ONLY — exhausted free legs will end the \
                     window cleanly; zero neurons can burn)"
                );
            }
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\nFATAL: free-leg probe failed ({e})");
            std::process::exit(EXIT_SMOKE_FAILED);
        }
    }
}

fn main() {
    riir_refine::install_env_rename_compat();
    let args = Args::parse();
    if args.preflight {
        run_preflight(&args);
    }
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
    let corpus_rules: std::collections::HashSet<&str> =
        harness.corpus.entries.iter().map(|e| e.rule_id()).collect();
    // The census is taken BEFORE build_backlog consumes the rows: an empty
    // queue has three distinct causes and the operator needs to know which.
    let census = census(&rows);
    let queue = build_backlog(rows, |lint| corpus_rules.contains(lint), args.limit);
    // Captured before the pre-pass consumes `queue`: an empty `prepared` set
    // means something very different when the queue was non-empty.
    let queued = queue.len();
    eprintln!(
        "[drain] backlog: {} queued (limit {}, store {} row(s)) — priority = fail_count × \
         gap(2.0 off-corpus / 1.0 in-corpus), never-healed first",
        queue.len(),
        args.limit,
        store.len()
    );
    eprintln!(
        "[drain] census: {} clean · {} eligible · {} withdrawn-with-failures (of {} row(s))",
        census.clean, census.eligible, census.withdrawn, census.total
    );
    // A withheld backlog is the failure mode a bare "queue empty" hides. Say
    // it on EVERY window, not only the empty ones — a non-empty queue can
    // coexist with a growing withdrawn pile.
    if census.withdrawn > 0 {
        eprintln!(
            "[drain] WARN: {} row(s) carry failures but are Withdrawn (absorbing tier) — the \
             drain will never queue them; they need a fresh pattern to re-enter",
            census.withdrawn
        );
    }

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
        prepared.push(Prepared {
            item,
            sample,
            miss,
            prompt_prepass,
        });
    }

    // ── Redaction gate: EVERY span, before any dispatch (own-repo origin —
    // the vetted class; these spans came from our own healing runs).
    let prepared_count = prepared.len();
    let loaded = RedactionGate::discover(args.policy.as_deref());
    let log = RedactionLog::open(&loaded.root());
    let mut kept: Vec<Prepared> = Vec::with_capacity(prepared.len());
    let mut blocked = 0usize;
    for mut p in prepared {
        let decision = loaded
            .gate
            .check(SpanOrigin::OwnRepo, None, &p.sample.module);
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
    eprintln!(
        "[drain] redaction gate: {} allowed, {blocked} blocked",
        kept.len()
    );
    if kept.is_empty() {
        if prepared_count == 0 {
            // The healthy state: the backlog (or the modelless pre-pass)
            // produced nothing to drain. Not an error — exit 0, the cron
            // row records a clean no-op.
            eprintln!(
                "[drain] queue empty — nothing to drain · {}",
                empty_queue_reason(census, args.limit, queued, modelless_served)
            );
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
        println!(
            "{:<34} {:<13} {:>5} {:>5} {:>8}  {:<10} prompt-b3-12",
            "id", "miss", "fail", "succ", "prio", "off-corpus"
        );
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

    // ── The LLM window (Issue 051: the free-first ladder — the free tier
    // absorbs the window; CF remains the metered fallback its USD cap
    // guards. Issue 106: infron is the second free leg BETWEEN them). A
    // free-leg key absent → the ladder just starts lower, byte-identical.
    let mut llm = match LlmRouter::from_env(args.model.as_deref()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("FATAL: {e}");
            std::process::exit(3);
        }
    };
    let free_armed = llm.groq_enabled() || llm.infron_enabled();
    let free_models = format!(
        "{}{}",
        llm.groq_model().unwrap_or("-"),
        llm.infron_model()
            .map_or_else(String::new, |m| format!(" + infron {m}"))
    );
    match (free_armed, llm.cf_armed()) {
        (true, true) => eprintln!(
            "[drain] free-first ladder armed ({}; CF fallback stays cap-guarded)",
            free_models
        ),
        (true, false) => eprintln!(
            "[drain] free-ONLY window ({}; no CF fallback armed) — exhausted free legs \
             end the window cleanly; zero neurons can burn",
            free_models
        ),
        (false, _) => eprintln!(
            "[drain] free legs not armed (GROQ_API_KEY / INFRON_API_KEY unset) — pure CF \
             window"
        ),
    }
    let (price_in, price_out) = riir_refine::llm_spend::price_for(llm.model_id())
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
            if g.provider == Provider::Cloudflare {
                format!(", {:.1} neurons", g.neurons)
            } else {
                String::new()
            }
        ),
        Err(e) => {
            eprintln!("FATAL: smoke failed ({e}) — aborting BEFORE any backlog burn");
            append_ledger(
                &args,
                llm.model_id(),
                &ArmSummary::default(),
                None,
                llm.cf().map(cf_workers_ai::CfClient::spend_state),
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

    let verifier = std::mem::replace(&mut harness.verifier, riir_refine::ClippyVerifier::new());
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
                        eprintln!(
                            "[drain] decline {} after {attempts} attempt(s): {e}",
                            p.sample.id
                        );
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
            if let Some((merged, _)) = cf_workers_ai::apply_edit(&p.sample.module, &gen.text, true)
            {
                let _ = store.record(&p.item.lint, &p.item.shape, &merged, true, true, &run_id);
                true
            } else {
                // Unreachable in practice (keep_strict implies a merge) —
                // record the failure rather than fabricating a keep.
                let _ = store.record(
                    &p.item.lint,
                    &p.item.shape,
                    &gen.text,
                    false,
                    false,
                    &run_id,
                );
                false
            }
        } else {
            let _ = store.record(
                &p.item.lint,
                &p.item.shape,
                &gen.text,
                false,
                false,
                &run_id,
            );
            false
        };
        record_case(
            &mut out,
            p,
            if outcome_ok { "kept" } else { "rejected" },
            &gen.text,
            &score,
        );
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

    println!(
        "\n═ drain summary (window {}) ═",
        if stopped_early {
            "STOPPED EARLY (cap)"
        } else {
            "complete"
        }
    );
    println!("| arm | n | EM | parses | merge | keep_o/scoreable | keep_s | vetoed | divergent | regressions | $ |");
    println!("{}", summary.markdown_row("DRAIN"));
    println!("  queued {} · llm-scored {llm_scored} · declined {declines} (overflow {overflow_declines})", kept.len());
    let server = llm.cf().and_then(|c| c.fetch_usage_today().ok());
    if let Some(u) = &server {
        println!(
            "  server meter (today, this model): {} req · {:.2} neurons ≈ ${:.5}",
            u.requests,
            u.neurons,
            u.neurons / 1000.0 * 0.011
        );
    }
    let gstats = llm.groq_stats.clone();
    if gstats.ok > 0 {
        println!(
            "  groq absorbed {} call(s), {}+{} tok (est ${:.5} of CF avoided; rate-limited {})",
            gstats.ok,
            gstats.prompt_tokens,
            gstats.completion_tokens,
            riir_refine::groq::router::avoided_usd(
                gstats.prompt_tokens,
                gstats.completion_tokens,
                price_in,
                price_out
            ),
            gstats.rate_limited,
        );
    }
    let istats = llm.infron_stats.clone();
    if istats.ok > 0 {
        println!(
            "  infron absorbed {} call(s), {}+{} tok (est ${:.5} of CF avoided; rate-limited {})",
            istats.ok,
            istats.prompt_tokens,
            istats.completion_tokens,
            riir_refine::groq::router::avoided_usd(
                istats.prompt_tokens,
                istats.completion_tokens,
                price_in,
                price_out
            ),
            istats.rate_limited,
        );
    }
    match llm.cf() {
        Some(cf) => println!("  {}", cf.spend_summary()),
        // Free-only: no paid leg, so no USD spend line to print. The
        // absorption lines above ARE the whole cost record for this window.
        None => println!("  CF fallback not armed — $0.0000 metered spend (free-only window)"),
    }
    eprintln!("[drain] store events → {}", store.root.display());
    append_ledger(
        &args,
        llm.model_id(),
        &summary,
        server.as_ref(),
        llm.cf().map(cf_workers_ai::CfClient::spend_state),
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
    /// Which provider served the smoke ("groq" / "infron" / "cloudflare" / "none").
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
    spend: Option<&riir_refine::llm_spend::SpendState>,
    drain: &DrainStats,
    llm: &LlmRouter,
) {
    let path = std::env::var("RIIR_LLM_COST_LEDGER").map_or_else(
        |_| args.repo_root.join(".benchmarks/llm_cost_ledger.jsonl"),
        PathBuf::from,
    );
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (price_in, price_out) = riir_refine::llm_spend::price_for(model)
        .map_or((0.0, 0.0), |p| (p.input_per_m, p.output_per_m));
    let gstats = &llm.groq_stats;
    let istats = &llm.infron_stats;
    let groq_avoided = riir_refine::groq::router::avoided_usd(
        gstats.prompt_tokens,
        gstats.completion_tokens,
        price_in,
        price_out,
    );
    let infron_avoided = riir_refine::groq::router::avoided_usd(
        istats.prompt_tokens,
        istats.completion_tokens,
        price_in,
        price_out,
    );
    // Issues 051+106: the free tiers' absorption rides `providers.groq` /
    // `providers.infron` — the dashboard's card reads these; legacy rows
    // without the keys are unaffected.
    let mut providers = riir_refine::groq::router::providers_json(
        gstats,
        llm.groq_model(),
        groq_avoided,
        istats,
        llm.infron_model(),
        infron_avoided,
    );
    if let Some(reason) = llm.groq_disable_reason() {
        providers["groq"]["disabled_reason"] = serde_json::json!(reason);
    }
    if let Some(reason) = llm.infron_disable_reason() {
        providers["infron"]["disabled_reason"] = serde_json::json!(reason);
    }
    let free_note = if gstats.ok > 0 || istats.ok > 0 {
        format!(
            "free legs absorbed {}/{} groq + {}/{} infron call(s) (est ${:.5} CF avoided)",
            gstats.ok,
            gstats.calls,
            istats.ok,
            istats.calls,
            groq_avoided + infron_avoided
        )
    } else if gstats.calls > 0 || istats.calls > 0 {
        "free legs armed but served nothing this window".to_string()
    } else {
        "pure CF window (no free leg armed)".to_string()
    };
    // Issue 051 T13 — WHO SERVED this window. Load-bearing for attribution:
    // `model` is the CF PRICING BASIS (it prices `usd_avoided` and is
    // resolved even when the CF leg was never armed), NOT a claim that CF
    // served anything.
    let served_by = served_by(
        gstats.ok > 0,
        istats.ok > 0,
        summary.prompt_tokens + summary.completion_tokens,
        drain.smoke_neurons,
        server.map_or(0.0, |u| u.neurons),
    );
    let row = serde_json::json!({
        "ts_unix": ts,
        "arm": "DRAIN",
        "model": model,
        "served_by": served_by,
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
        "note": format!("kat null until Phase 1 rail; Plan 031 A3 trainer-window drain (never-wrong: keeps are Certified trajectories, rejects never land); {free_note}"),
    });
    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "[drain] cost ledger write skipped ({}: {e})",
                path.display()
            );
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
            journey_runs: 0,
            journey_taint_sum: 0,
            src_path: String::new(),
        }
    }

    const CERTIFIED: u8 = 0;
    const HEURISTIC: u8 = 1;
    const WITHDRAWN: u8 = 2;

    // ── Backlog census: an empty queue has three distinct causes ─────────

    #[test]
    fn census_buckets_are_disjoint_and_total() {
        let rows = vec![
            row("clean_a", "fn a() {}", 0, 3, CERTIFIED),
            row("clean_b", "fn b() {}", 0, 1, HEURISTIC),
            // fail_count == 0 wins even at the Withdrawn tier: nothing to heal.
            row("clean_withdrawn", "fn c() {}", 0, 0, WITHDRAWN),
            row("eligible_a", "fn d() {}", 2, 0, HEURISTIC),
            row("stalled_a", "fn e() {}", 5, 1, WITHDRAWN),
            row("stalled_b", "fn f() {}", 3, 0, WITHDRAWN),
        ];
        let c = census(&rows);
        assert_eq!(c.total, 6);
        assert_eq!(c.clean, 3);
        assert_eq!(c.eligible, 1);
        assert_eq!(c.withdrawn, 2);
        // Disjoint + exhaustive: the buckets must account for every row, or
        // the operator line silently drops rows.
        assert_eq!(c.clean + c.eligible + c.withdrawn, c.total);
    }

    /// The census must never disagree with the queue it explains.
    #[test]
    fn census_eligible_matches_what_build_backlog_queues() {
        let rows = vec![
            row("clean", "fn a() {}", 0, 3, CERTIFIED),
            row("eligible_a", "fn d() {}", 2, 0, HEURISTIC),
            row("eligible_b", "fn g() {}", 1, 2, CERTIFIED),
            row("stalled", "fn e() {}", 5, 1, WITHDRAWN),
        ];
        let c = census(&rows);
        let q = build_backlog(rows, |_| true, usize::MAX);
        assert_eq!(
            c.eligible,
            q.len(),
            "census eligible must equal the unlimited queue length"
        );
    }

    /// A store whose only failures are Withdrawn is a STALL — it must not
    /// read like a clean store. This is the whole point of the census.
    #[test]
    fn an_all_withdrawn_backlog_reports_a_stall_not_a_clean_store() {
        let rows = vec![
            row("clean", "fn a() {}", 0, 3, CERTIFIED),
            row("stalled", "fn e() {}", 5, 1, WITHDRAWN),
        ];
        let c = census(&rows);
        assert_eq!(c.eligible, 0, "nothing queues");
        let msg = empty_queue_reason(c, 25, 0, 0);
        assert!(msg.contains("STALL"), "{msg}");
        assert!(msg.contains("withheld"), "{msg}");
        assert!(
            !msg.contains("healthy"),
            "a withheld backlog must never read healthy: {msg}"
        );
    }

    #[test]
    fn an_all_clean_backlog_reports_healthy() {
        let rows = vec![
            row("clean_a", "fn a() {}", 0, 3, CERTIFIED),
            row("clean_b", "fn b() {}", 0, 1, HEURISTIC),
        ];
        let msg = empty_queue_reason(census(&rows), 25, 0, 0);
        assert!(msg.contains("healthy"), "{msg}");
        assert!(!msg.contains("STALL"), "{msg}");
    }

    #[test]
    fn an_empty_store_is_distinct_from_a_clean_one() {
        let msg = empty_queue_reason(census(&[]), 25, 0, 0);
        assert!(msg.contains("store is empty"), "{msg}");
        // "no heal run has recorded a trajectory" is a DIFFERENT operator
        // action (run the healer) than "every row is clean" (nothing to do).
        assert!(!msg.contains("healthy"), "{msg}");
    }

    /// The regression this fix exists for. `prepared_count == 0` is ALSO
    /// reached when the modelless seam served every queued span — the BEST
    /// outcome (real backlog, healed at zero LLM cost). The first cut of
    /// `empty_queue_reason` reported that as "BUG: the census and
    /// build_backlog disagree", i.e. it screamed defect at the mechanism
    /// working exactly as the L4 contract designs it.
    #[test]
    fn a_fully_modelless_window_is_not_a_bug_and_not_an_empty_backlog() {
        let rows = vec![
            row("clean", "fn a() {}", 0, 3, CERTIFIED),
            row("eligible_a", "fn d() {}", 2, 0, HEURISTIC),
            row("eligible_b", "fn g() {}", 1, 2, CERTIFIED),
        ];
        let c = census(&rows);
        assert_eq!(c.eligible, 2);
        // 2 queued, all 2 absorbed by the modelless pre-pass.
        let msg = empty_queue_reason(c, 25, 2, 2);
        assert!(
            !msg.contains("BUG"),
            "the best outcome must not read as a defect: {msg}"
        );
        assert!(msg.contains("modelless seam"), "{msg}");
        assert!(msg.contains("zero LLM cost"), "{msg}");
        assert!(
            !msg.contains("healthy: all"),
            "the backlog was REAL, not clean: {msg}"
        );
    }

    /// `--limit 0` withholds a real backlog by operator choice — also not a
    /// census disagreement.
    #[test]
    fn limit_zero_is_an_operator_choice_not_a_census_bug() {
        let rows = vec![row("eligible_a", "fn d() {}", 2, 0, HEURISTIC)];
        let msg = empty_queue_reason(census(&rows), 0, 0, 0);
        assert!(!msg.contains("BUG"), "{msg}");
        assert!(msg.contains("--limit 0"), "{msg}");
    }

    /// The BUG branch must still fire on a genuine, unexplained disagreement:
    /// eligible rows, a positive limit, and nothing queued.
    #[test]
    fn a_genuine_census_disagreement_still_reports_bug() {
        let rows = vec![row("eligible_a", "fn d() {}", 2, 0, HEURISTIC)];
        let msg = empty_queue_reason(census(&rows), 25, 0, 0);
        assert!(
            msg.contains("BUG"),
            "a real disagreement must not be explained away: {msg}"
        );
    }

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
    fn served_by_attributes_each_window_shape() {
        // The T9 shape this field exists for: groq did all the work, the CF
        // leg was never even armed.
        assert_eq!(served_by(true, false, 0, 0.0, 0.0), "groq");
        // The Issue 106 shape: infron absorbed the window (groq down).
        assert_eq!(served_by(false, true, 0, 0.0, 0.0), "infron");
        // Both free legs served, no CF.
        assert_eq!(served_by(true, true, 0, 0.0, 0.0), "groq+infron");
        // Pure CF (free legs unarmed or served nothing).
        assert_eq!(served_by(false, false, 1000, 0.0, 0.0), "cloudflare");
        assert_eq!(served_by(false, false, 0, 0.0, 587.36), "cloudflare");
        // Mixed: the free tiers absorbed part, CF caught the rest.
        assert_eq!(served_by(true, false, 500, 0.0, 250.0), "mixed");
        assert_eq!(served_by(true, true, 500, 0.0, 250.0), "mixed");
        // A failed smoke that never reached any provider.
        assert_eq!(served_by(false, false, 0, 0.0, 0.0), "none");
    }

    #[test]
    fn a_cf_smoke_is_not_attributed_to_the_free_tier() {
        // groq served every SPAN but the smoke landed on CF — those neurons
        // are real. Calling this "groq" would hide metered spend behind the
        // free-tier label, which is the one thing the ledger must never do.
        assert_eq!(served_by(true, false, 0, 60.0, 0.0), "mixed");
    }

    #[test]
    fn free_tier_exhausted_classifies_stop_day() {
        // Issue 051 fix: a groq-only window whose free tier runs out has no
        // paid leg to fall back to — terminal, exactly like the USD cap.
        assert_eq!(
            classify_error(
                "free-tier exhausted: groq is the only armed provider (429 rate limit) — no \
                 CF fallback (CF_API_TOKEN / CF_ACCOUNT_ID unset)"
            ),
            AttemptPlan::StopDay
        );
    }

    #[test]
    fn a_transient_groq_failure_without_cf_declines_only_the_span() {
        // A transport blip is NOT terminal even with no fallback — the next
        // span retries groq. Only the (windowed) rate limit ends the day.
        assert_eq!(
            classify_error("groq attempt failed and no CF fallback is armed (connect timeout)"),
            AttemptPlan::DeclineSpan
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
