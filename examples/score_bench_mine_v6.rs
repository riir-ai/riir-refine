//! clippy-bench generation-6 fixture mining — Plan 095 T2 (twice-amended
//! per §1a: fn-granularity AND adjacency-run extraction were both falsified
//! by measurement — the multi-lint population lives in crate-internal code
//! that the bare-stub filter drops. The v1 record already carried the
//! answer: its real fixtures note "trait scaffolding added, lint spans
//! verbatim". The CARVE pipeline mechanizes exactly that: each candidate is
//! carved with its FILE'S OWN header context so the lint spans compile
//! verbatim — no synthesized stubs, no edited spans).
//!
//! Pipeline (deterministic, byte-stable):
//! 1. Rank sweep-healed files by diff magnitude (lint-richness proxy), top
//!    `--files N`.
//! 2. Candidates per file: every top-level `fn` + every `impl` block.
//! 3. Carve tiers (first that compiles wins):
//!    T1 = file's use + type + const items + the candidate;
//!    T2 = T1 + macro + impl items;
//!    T3 = T2 + the file's other fns (total capped — a real multi-fn
//!    excerpt, the v1 quantile_balance_router shape).
//! 4. Batch-label every compiled carve (vocabulary intersection).
//! 5. Emit label-k ≥ `--min-lints` entries (multi) + k==1 controls.
//!
//! Instrument honesty: the BATCH instrument is discovery-only — the landed
//! `expect_lints` are provisional claims the freeze e2e adjudicates with
//! the BENCH instrument (single-snippet `lints_for`); disagreement is a
//! loud freeze-time exclusion or label correction (the gen-3 flow).
//!
//! ## `--order margin` + `--ab` — the margin-weighted arm (issue 077 T3)
//!
//! With `--features margin_mining`, `--order margin` replaces step 1's
//! ordering: every `.rs` file in the SAME diff universe is carved (syn
//! only, no oracle), each carve gets the drafter's pre-rerank pool margin
//! `M = top1 − top2` (`riir_clippy::draft::margin_weight::span_min_margin`
//! — the `rank_into` canon, margins ≥ 0 by construction), and files rank by
//! their total `e^{−2M}` weight mass — the paper's w(M) ∝ e^{−2M} law:
//! smallest margin → largest weight → mined FIRST. Uncontested carves
//! (fewer than two candidates in every enabled domain) weigh exactly 0.
//! Ties break by path; the budget cut is the SAME `--files N`.
//! The DEFAULT (no flag) stays diff-count-desc and is bit-identical with
//! the feature off — the arm lives behind `margin_mining` end to end.
//!
//! ### `--ab` — the A/B protocol (PRE-REGISTERED before the first run)
//!
//! Mine TWO budget-capped candidate sets at the SAME `--files` budget from
//! the same diff universe — control = count-desc (today's order), arm =
//! margin-weighted — and measure BOTH on the real score bench
//! (`run_bench_with`, the generation-1 instrument: real clippy, the frozen
//! scoring rules). **Verdict rule, fixed before the first run: the arm
//! WINS iff its heal rate STRICTLY beats the control's; a tie is a loss**
//! (no gain = demote). A recorded loss keeps `margin_mining` opt-in and the
//! miner's default at count-desc — that is a valid, publishable outcome.
//! Set composition (both arms): every labeled carve carrying ≥1 vocabulary
//! lint (multi + k==1 controls; no 15-control cap — the budget is
//! `--files`). Secondary diagnostics (n_ran, created_rate, parse_safety,
//! decline_correctness) are recorded but NOT part of the verdict. G1
//! margin-contract + concentration tests: `src/draft/margin_weight.rs`.
//! G3: the arm's ranking pass is two in-place sorts over pre-allocated
//! buffers — zero per-candidate allocation in the ranking loop itself (the
//! matcher's own scratch reuse is the existing substrate surface).
//!
//! Run (from the riir-clippy repo root):
//!
//! ```text
//! cargo run --example score_bench_mine_v6 --features score_bench -- \
//!     ../riir-chain 7421245a~1 7421245a --min-lints 2
//! cargo run --example score_bench_mine_v6 --features margin_mining -- \
//!     ../riir-chain 7421245a~1 7421245a --order margin
//! cargo run --example score_bench_mine_v6 --features margin_mining -- \
//!     ../riir-chain 7421245a~1 7421245a --files 6 --ab
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use syn::spanned::Spanned;

use riir_clippy::score_bench::{vocabulary, wrap};

#[cfg(feature = "margin_mining")]
use riir_clippy::score_bench::BenchFixture;

#[cfg(feature = "margin_mining")]
use riir_clippy::draft::{LatentMatcher, margin_weight, span_min_margin};

const CARVE_MAX_BYTES: usize = 8000;
const CHUNK: usize = 15;

struct Cfg {
    repo: String,
    sha: String,
    sweep: String,
    min_lints: usize,
    max_files: usize,
    /// `--tree <prefix>`: discover files by `git ls-tree` under a path
    /// prefix instead of a sweep-commit diff (for repos with no sweep
    /// commit — the oracle label is the evidence; the sweep diff was only
    /// a yield heuristic).
    tree: Option<String>,
    /// `--order <count|margin>` (issue 077 T3; `margin` needs
    /// `--features margin_mining`). Default `count` = bit-identical to the
    /// pre-T3 behavior.
    order: Option<String>,
    /// `--ab`: run BOTH orders and the pre-registered bench A/B
    /// (`margin_mining` only).
    ab: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |name: &str| -> Option<usize> {
        args.iter()
            .position(|a| a == name)
            .and_then(|p| args.get(p + 1))
            .and_then(|v| v.parse().ok())
    };
    let opt_str = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|p| args.get(p + 1))
            .cloned()
    };
    let min_lints = opt("--min-lints").unwrap_or(2);
    let max_files = opt("--files").unwrap_or(30);
    let tree = opt_str("--tree");
    let order = opt_str("--order");
    let ab = args.iter().any(|a| a == "--ab");
    let positional: Vec<String> = args
        .into_iter()
        .filter(|a| !a.starts_with("--") && a.parse::<usize>().is_err())
        .filter(|a| Some(a.as_str()) != tree.as_deref())
        .filter(|a| Some(a.as_str()) != order.as_deref())
        .collect();
    let [repo, sha, sweep] = positional.as_slice() else {
        eprintln!(
            "usage: score_bench_mine_v6 <repo> <sha> <sweep-commit> [--min-lints N] [--files N]\n       [--tree <prefix>] [--order count|margin] [--ab]  (margin/--ab need --features margin_mining)"
        );
        std::process::exit(2);
    };
    let cfg = Cfg {
        repo: repo.clone(),
        sha: sha.clone(),
        sweep: sweep.clone(),
        min_lints,
        max_files,
        tree,
        order,
        ab,
    };

    let oracle = riir_clippy::draft::ClippyOracle::new()
        .with_work_root(std::env::temp_dir().join("riir_clippy_score_bench_v6"));
    if !oracle.is_available() {
        eprintln!("cargo not on PATH — the oracle needs real clippy");
        std::process::exit(1);
    }
    let vocab = vocabulary();
    let repo_name = cfg
        .repo
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(&cfg.repo)
        .to_string();

    // Validate the order knob before any work.
    if let Some(o) = &cfg.order {
        if o != "count" && o != "margin" {
            eprintln!("--order must be count|margin (got {o})");
            std::process::exit(2);
        }
    }

    // The A/B runs its own two pipelines; nothing below is shared with it.
    #[cfg(feature = "margin_mining")]
    if cfg.ab {
        run_ab(&cfg, &oracle, &vocab, &repo_name);
        return;
    }
    #[cfg(not(feature = "margin_mining"))]
    if cfg.ab || cfg.order.as_deref() == Some("margin") {
        eprintln!("--order margin / --ab require --features margin_mining");
        std::process::exit(2);
    }

    // Carve candidates: (id, provenance-path, tiered code variants). The
    // margin order (feature-gated) selects its own files; the default path
    // is byte-identical to the pre-T3 miner.
    let mut candidates: Vec<Cand> = Vec::new();
    #[cfg(feature = "margin_mining")]
    let margin_ordered = cfg.order.as_deref() == Some("margin");
    #[cfg(feature = "margin_mining")]
    if margin_ordered {
        let mut matcher = LatentMatcher::from_enabled_domains().unwrap_or_else(|e| {
            eprintln!("matcher build failed: {e}");
            std::process::exit(1);
        });
        candidates = margin_selected_candidates(&cfg, &mut matcher);
    }
    #[cfg(feature = "margin_mining")]
    if !margin_ordered {
        let files = sweep_files(&cfg, Some(cfg.max_files));
        eprintln!(
            "# {repo_name}: {} file(s) from {} (order count); mining at {}",
            files.len(),
            cfg.tree.as_ref().map_or_else(|| format!("sweep {}", cfg.sweep), |t| format!("tree {t}")),
            cfg.sha
        );
        for path in &files {
            candidates.append(&mut carve_file(&cfg, path).unwrap_or_default());
        }
    }
    #[cfg(not(feature = "margin_mining"))]
    {
        let files = sweep_files(&cfg, Some(cfg.max_files));
        eprintln!(
            "# {repo_name}: {} file(s) from {} (order count); mining at {}",
            files.len(),
            cfg.tree.as_ref().map_or_else(|| format!("sweep {}", cfg.sweep), |t| format!("tree {t}")),
            cfg.sha
        );
        for path in &files {
            candidates.append(&mut carve_file(&cfg, path).unwrap_or_default());
        }
    }
    eprintln!("# {} carve candidate(s)", candidates.len());
    run_mine(&cfg, &oracle, &vocab, &repo_name, candidates);
}

/// The count-order (default) tail of the miner: compile tiers, batch label,
/// census, paste-ready emission. Shared unchanged by both `--order` modes.
fn run_mine(
    cfg: &Cfg,
    oracle: &riir_clippy::draft::ClippyOracle,
    vocab: &BTreeSet<String>,
    repo_name: &str,
    candidates: Vec<Cand>,
) {
    let labels = compile_and_label(oracle, vocab, &candidates);

    // Census + emission.
    let mut census: BTreeMap<usize, usize> = BTreeMap::new();
    let mut class_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut emitted = 0usize;
    let mut controls: Vec<(usize, &BTreeSet<String>, &String)> = Vec::new();
    for (i, labeled) in labels.iter().enumerate() {
        let Some((lints, code)) = labeled else { continue };
        let k = lints.len();
        *census.entry(k).or_insert(0) += 1;
        for l in lints {
            *class_count.entry(l.clone()).or_insert(0) += 1;
        }
        if k >= cfg.min_lints {
            emit(repo_name, &cfg.sha, &candidates[i], lints, code, emitted);
            emitted += 1;
        } else if k == 1 {
            controls.push((i, lints, code));
        }
    }
    eprintln!("\n# carve census (distinct-vocab-lint count → carves): {census:?}");
    eprintln!("# lint-class frequency: {class_count:?}");
    eprintln!("# emitted {emitted} multi carve(s) (min-lints {})", cfg.min_lints);
    eprintln!("# single control candidates (k=1, first 15):");
    for (n, (i, lints, code)) in controls.iter().take(15).enumerate() {
        emit(repo_name, &cfg.sha, &candidates[*i], lints, code, n);
    }
}

/// The pre-registered A/B (issue 077 T3). See the module doc for the
/// verdict rule — fixed before the first run: arm wins iff its bench heal
/// rate STRICTLY beats the control's.
#[cfg(feature = "margin_mining")]
fn run_ab(
    cfg: &Cfg,
    oracle: &riir_clippy::draft::ClippyOracle,
    vocab: &BTreeSet<String>,
    repo_name: &str,
) {
    // ── Control arm: count-desc (today's order) ─────────────────────────
    let control_files = sweep_files(cfg, Some(cfg.max_files));
    eprintln!(
        "# A/B control: top {} of the diff universe by diff magnitude",
        cfg.max_files
    );
    let mut control_cands: Vec<Cand> = Vec::new();
    for path in &control_files {
        control_cands.append(&mut carve_file(cfg, path).unwrap_or_default());
    }

    // ── Arm: e^{−2M} margin-weighted file mass, same budget ────────────
    let mut matcher = LatentMatcher::from_enabled_domains().unwrap_or_else(|e| {
        eprintln!("matcher build failed: {e}");
        std::process::exit(1);
    });
    eprintln!(
        "# A/B arm: same universe ranked by e^{{−2M}} file mass (domains {:?})",
        matcher.domains()
    );
    let arm_cands = margin_selected_candidates(cfg, &mut matcher);

    eprintln!(
        "# control: {} carve candidate(s) | arm: {} carve candidate(s)",
        control_cands.len(),
        arm_cands.len()
    );

    // ── Measure both on the real score bench (generation-1 instrument) ──
    let control = measure_arm(cfg, oracle, vocab, repo_name, "control(count-desc)", control_cands);
    let arm = measure_arm(cfg, oracle, vocab, repo_name, "arm(margin e^−2M)", arm_cands);

    // ── The pre-registered verdict ──────────────────────────────────────
    println!("\n── margin-mining A/B (issue 077 T3) ──");
    println!(
        "control (count-desc):     heal_rate {:.1}%  ({} ran / {} healable, created {} @ {:.1}%, parse {:.1}%, decline {:.1}%)",
        100.0 * control.heal_rate,
        control.n_ran,
        control.healable,
        control.created_total,
        100.0 * control.created_rate,
        100.0 * control.parse_safety,
        100.0 * control.decline_correctness
    );
    println!(
        "arm     (margin e^−2M):   heal_rate {:.1}%  ({} ran / {} healable, created {} @ {:.1}%, parse {:.1}%, decline {:.1}%)",
        100.0 * arm.heal_rate,
        arm.n_ran,
        arm.healable,
        arm.created_total,
        100.0 * arm.created_rate,
        100.0 * arm.parse_safety,
        100.0 * arm.decline_correctness
    );
    if arm.heal_rate > control.heal_rate {
        println!("VERDICT: WIN — arm heal rate strictly beats control (pre-registered rule)");
    } else {
        println!(
            "VERDICT: DEMOTE — arm did NOT strictly beat control (tie or loss); \
             margin_mining stays opt-in, the miner default stays count-desc"
        );
    }
}

/// Compile-tier + label + bench one arm's candidates; returns the bench
/// score plus the healable count (the A/B report's denominator view).
#[cfg(feature = "margin_mining")]
#[allow(clippy::cast_precision_loss)] // count→ratio over fixture counts
fn measure_arm(
    cfg: &Cfg,
    oracle: &riir_clippy::draft::ClippyOracle,
    vocab: &BTreeSet<String>,
    repo_name: &str,
    tag: &str,
    candidates: Vec<Cand>,
) -> ArmScore {
    let labels = compile_and_label(oracle, vocab, &candidates);
    // Set composition (pre-registered): every labeled carve with ≥1
    // vocabulary lint — multi + controls, no 15-cap; the budget is --files.
    // Code = the tier the oracle adjudicated (not a T1/T2 guess).
    let mut mined: Vec<BenchFixture> = Vec::new();
    for (cand, labeled) in candidates.iter().zip(&labels) {
        let Some((lints, code)) = labeled else { continue };
        if lints.is_empty() {
            continue;
        }
        mined.push(BenchFixture {
            id: Box::leak(cand.id.clone().into_boxed_str()),
            provenance: Box::leak(
                format!("{repo_name}@{}:{} (file-header carve)", cfg.sha, cand.path)
                    .into_boxed_str(),
            ),
            code: Box::leak(code.clone().into_boxed_str()),
            expect_lints: Box::leak(
                lints.iter()
                    .map(|l| &*Box::leak(l.clone().into_boxed_str()))
                    .collect::<Vec<&str>>()
                    .into_boxed_slice(),
            ),
            clean: false,
        });
    }
    eprintln!(
        "# {tag}: {}/{} carve(s) labeled ≥1 vocab lint → bench set",
        mined.len(),
        candidates.len()
    );
    let score = riir_clippy::score_bench::run_bench_with(oracle, &mined, &[]);
    let healable = score.outcomes.iter().filter(|o| !o.vocab_before.is_empty()).count();
    ArmScore {
        heal_rate: score.heal_rate,
        created_total: score.created_total,
        created_rate: score.created_rate,
        parse_safety: score.parse_safety,
        decline_correctness: score.decline_correctness,
        n_ran: score.n_ran,
        healable,
    }
}

#[cfg(feature = "margin_mining")]
struct ArmScore {
    heal_rate: f32,
    created_total: usize,
    created_rate: f32,
    parse_safety: f32,
    decline_correctness: f32,
    n_ran: usize,
    healable: usize,
}

/// The margin-weighted file selection (issue 077 T3): carve EVERY file in
/// the diff universe (syn only — the oracle never runs here), weight each
/// carve by the drafter's margin law, and keep the top-`max_files` files by
/// total `e^{−2M}` mass. G3: the ranking pass is two in-place sorts over
/// pre-allocated buffers — zero per-candidate allocation in the ranking
/// loop itself (`pool_and_ranked`'s own scratch reuse is the existing
/// substrate surface, shared with `jury_disagreement`).
#[cfg(feature = "margin_mining")]
fn margin_selected_candidates(cfg: &Cfg, matcher: &mut LatentMatcher) -> Vec<Cand> {
    let all_files = sweep_files(cfg, None);
    eprintln!(
        "# margin arm: {} file(s) in the diff universe",
        all_files.len()
    );
    let mut masses: Vec<(f32, String)> = Vec::with_capacity(all_files.len());
    let mut per_file: Vec<Vec<Cand>> = Vec::with_capacity(all_files.len());
    for path in &all_files {
        let cands = carve_file(cfg, path).unwrap_or_default();
        let mut mass = 0.0f32;
        for c in &cands {
            // The tier the oracle would settle is unknown pre-compile; the
            // margin reads the minimal carve form (T1, falling back to T2
            // when the size cap dropped T1).
            let text = if c.t1.is_empty() { c.t2.as_str() } else { c.t1.as_str() };
            mass += margin_weight(span_min_margin(matcher, text));
        }
        masses.push((mass, path.clone()));
        per_file.push(cands);
    }
    // Law order: mass desc, path asc (determinism); then the SAME budget cut.
    masses.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    masses.truncate(cfg.max_files);
    for (mass, path) in &masses {
        eprintln!("#   mass {mass:.4}  {path}");
    }
    let selected: BTreeSet<&String> = masses.iter().map(|(_, p)| p).collect();
    let mut out = Vec::new();
    for (i, path) in all_files.iter().enumerate() {
        if selected.contains(path) {
            out.append(&mut per_file[i]);
        }
    }
    out
}

/// One file's carve candidates (header-context scaffolding per the Plan 095
/// pipeline). `None` = unreadable / unparseable / nothing to scaffold with.
fn carve_file(cfg: &Cfg, path: &str) -> Option<Vec<Cand>> {
    let src = git_show(&cfg.repo, &cfg.sha, path)?;
    let file = syn::parse_file(&src).ok()?;
    let stem = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".rs");
    let mut header = String::new(); // use + type + const items
    let mut macros = String::new();
    let mut impls = String::new();
    let mut fns: Vec<String> = Vec::new();
    for item in &file.items {
        let range = item.span().byte_range();
        let Some(text) = src.get(range) else { continue };
        let text = format!("{text}\n");
        match item {
            syn::Item::Use(_) | syn::Item::Struct(_) | syn::Item::Enum(_)
            | syn::Item::Union(_) | syn::Item::Type(_) | syn::Item::Const(_)
            | syn::Item::Static(_) => header.push_str(&text),
            syn::Item::Macro(_) => macros.push_str(&text),
            syn::Item::Impl(_) => impls.push_str(&text),
            syn::Item::Fn(f) if !f.block.stmts.is_empty() => {
                fns.push(text);
            }
            _ => {}
        }
    }
    if header.is_empty() && impls.is_empty() {
        return None; // nothing to scaffold with — the bare crust
    }
    let mut cands = Vec::with_capacity(fns.len());
    let mut idx = 0usize;
    for f in &fns {
        cands.push(build_cand(stem, path, &mut idx, "fn", f, &header, &macros, &impls, &fns));
    }
    // Impl blocks as whole candidates (methods are where src lints live).
    for item in &file.items {
        let syn::Item::Impl(_) = item else { continue };
        let range = item.span().byte_range();
        let Some(text) = src.get(range) else { continue };
        let text = format!("{text}\n");
        if text.len() > CARVE_MAX_BYTES {
            continue;
        }
        cands.push(build_cand(stem, path, &mut idx, "impl", &text, &header, &macros, &impls, &fns));
    }
    Some(cands)
}

/// Tiered compile (T1→T2→T3) + batch oracle labeling — the miner's shared
/// instrument. Returns per-candidate (vocabulary labels, final tier text)
/// (`None` = did not compile in any tier, or the oracle stayed silent on
/// it). The resolved text is what the A/B bench must measure — the tier the
/// oracle actually adjudicated, not a T1/T2 guess.
fn compile_and_label(
    oracle: &riir_clippy::draft::ClippyOracle,
    vocab: &BTreeSet<String>,
    candidates: &[Cand],
) -> Vec<Option<(BTreeSet<String>, String)>> {
    // Probe tiers: T1 for all; T2 for T1 failures; T3 for T2 failures.
    let t1: Vec<String> = candidates.iter().map(|c| wrap(&c.t1)).collect();
    let ok1 = batch_compile(oracle, &t1);
    let t2_idx: Vec<usize> = (0..candidates.len()).filter(|&i| !ok1[i]).collect();
    let t2: Vec<String> = t2_idx.iter().map(|&i| wrap(&candidates[i].t2)).collect();
    let ok2 = batch_compile(oracle, &t2);
    let t3_idx: Vec<usize> = t2_idx
        .iter()
        .zip(&ok2)
        .filter(|(_, &ok)| !ok)
        .map(|(&i, _)| i)
        .collect();
    let t3: Vec<String> = t3_idx
        .iter()
        .filter_map(|&i| candidates[i].t3.as_deref().map(wrap))
        .collect();
    let ok3 = batch_compile(oracle, &t3);

    // Resolve final code per candidate.
    let mut final_code: Vec<Option<String>> = vec![None; candidates.len()];
    for (i, ok) in ok1.iter().enumerate() {
        if *ok {
            final_code[i] = Some(candidates[i].t1.clone());
        }
    }
    for (slot, &i) in t2_idx.iter().enumerate() {
        if ok2[slot] {
            final_code[i] = Some(candidates[i].t2.clone());
        }
    }
    let mut t3_slot = 0usize;
    for &i in &t3_idx {
        let has_t3 = candidates[i].t3.is_some();
        if has_t3 {
            if ok3[t3_slot] {
                final_code[i] = candidates[i].t3.clone();
            }
            t3_slot += 1;
        }
    }
    let compiled: Vec<usize> = (0..candidates.len()).filter(|&i| final_code[i].is_some()).collect();
    eprintln!(
        "# {} / {} carve(s) compile (T2 rescues {}, T3 rescues {})",
        compiled.len(),
        candidates.len(),
        t2_idx.iter().zip(&ok2).filter(|(_, ok)| **ok).count(),
        ok3.iter().filter(|ok| **ok).count(),
    );

    // Label every compiled carve.
    let mut labels: Vec<Option<(BTreeSet<String>, String)>> = vec![None; candidates.len()];
    for chunk in compiled.chunks(CHUNK) {
        let snippets: Vec<String> = chunk
            .iter()
            .map(|&i| wrap(final_code[i].as_ref().unwrap_or(&String::new())))
            .collect();
        let refs: Vec<&str> = snippets.iter().map(String::as_str).collect();
        let verdicts = oracle.lints_for_batch(&refs);
        for (&ci, v) in chunk.iter().zip(verdicts) {
            if v.ran && v.compiles {
                labels[ci] = Some((
                    v.lints.intersection(vocab).cloned().collect(),
                    final_code[ci].clone().unwrap_or_default(),
                ));
            }
        }
    }
    labels
}

struct Cand {
    id: String,
    path: String,
    /// T1: header (use+type+const) + candidate.
    t1: String,
    /// T2: T1 + macros + impls.
    t2: String,
    /// T3: T2 + sibling fns (None when over the size cap).
    t3: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn build_cand(
    stem: &str,
    path: &str,
    idx: &mut usize,
    kind: &str,
    cand_code: &str,
    header: &str,
    macros: &str,
    impls: &str,
    fns: &[String],
) -> Cand {
    let id = format!("gen6-{stem}-{}{}", kind, {
        *idx += 1;
        *idx
    });
    let t1 = format!("{header}{cand_code}");
    let t2 = format!("{t1}{macros}{impls}");
    let others: String = fns.iter().filter(|f| *f != cand_code).take(6).cloned().collect();
    let t3_full = format!("{t2}{others}");
    let t3 = if t3_full.len() <= CARVE_MAX_BYTES * 2 {
        Some(t3_full)
    } else {
        None
    };
    Cand {
        id,
        path: path.to_string(),
        t1: if t1.len() <= CARVE_MAX_BYTES * 2 { t1 } else { String::new() },
        t2,
        t3,
    }
}

fn batch_compile(oracle: &riir_clippy::draft::ClippyOracle, snippets: &[String]) -> Vec<bool> {
    let mut out = Vec::with_capacity(snippets.len());
    for chunk in snippets.chunks(CHUNK) {
        let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
        out.extend(oracle.compiles_batch(&refs));
    }
    out
}

/// Rank sweep-healed files by diff magnitude (changed lines), optionally
/// truncated to `limit` — or, in `--tree` mode, list tracked .rs files
/// under a path prefix (up to a sane probe budget, sorted by path for
/// determinism). `None` limit = the whole universe (the margin arm ranks
/// EVERY file before applying the `--files` budget).
fn sweep_files(cfg: &Cfg, limit: Option<usize>) -> Vec<String> {
    if let Some(prefix) = &cfg.tree {
        let out = Command::new("git")
            .arg("-C")
            .arg(&cfg.repo)
            .args(["ls-tree", "-r", "--name-only", &cfg.sha, "--", prefix])
            .output()
            .unwrap_or_else(|e| {
                eprintln!("git spawn failed: {e}");
                std::process::exit(1);
            });
        if !out.status.success() {
            eprintln!("git ls-tree {} -- {prefix} failed", cfg.sha);
            std::process::exit(1);
        }
        let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.ends_with(".rs"))
            .take(60)
            .map(ToString::to_string)
            .collect();
        if let Some(limit) = limit {
            files.truncate(limit);
        }
        return files;
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(&cfg.repo)
        .args(["diff-tree", "--no-commit-id", "--stat", "-r", &cfg.sweep])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("git spawn failed: {e}");
            std::process::exit(1);
        });
    if !out.status.success() {
        eprintln!("git diff-tree {} failed", cfg.sweep);
        std::process::exit(1);
    }
    let mut rows: Vec<(usize, String)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let (path, changes) = l.rsplit_once("|")?;
            let n: usize = changes
                .split('|')
                .next_back()
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse().ok())?;
            let path = path.trim().to_string();
            if path.ends_with(".rs") && !path.contains("tests/") && !path.contains("/tests") {
                Some((n, path))
            } else {
                None
            }
        })
        .collect();
    rows.sort_by_key(|a| std::cmp::Reverse(a.0));
    if let Some(limit) = limit {
        rows.truncate(limit);
    }
    rows.into_iter().map(|(_, p)| p).collect()
}

fn git_show(repo: &str, sha: &str, path: &str) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["show", &format!("{sha}:{path}")])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Emit one `RawFixture` entry (paste into generations.rs). `code` is the
/// oracle-adjudicated tier text — never a T1/T2 guess.
#[allow(clippy::too_many_arguments)]
fn emit(
    repo: &str,
    sha: &str,
    cand: &Cand,
    lints: &BTreeSet<String>,
    code: &str,
    idx: usize,
) {
    let lints: Vec<String> = lints.iter().map(|s| format!("{s:?}")).collect();
    let file = &cand.path;
    let id = format!("{}-{idx:03}", cand.id);
    eprintln!(
        "    // CANDIDATE {id} | provenance: {repo}@{sha}:{file} (file-header carve)\n    RawFixture {{\n        id: \"{id}\",\n        provenance: \"{repo}@{sha}:{file} (file-header carve)\",\n        code: {},\n        expect: &[{}],\n    }},\n",
        rust_string_literal(code),
        lints.join(", ")
    );
}

/// Render source as a Rust string literal (raw string with enough `#`s).
fn rust_string_literal(code: &str) -> String {
    let mut hashes = 1;
    while code.contains(&format!("\"{}", "#".repeat(hashes))) {
        hashes += 1;
    }
    let h = "#".repeat(hashes);
    format!("r{h}\"{code}\"{h}")
}
