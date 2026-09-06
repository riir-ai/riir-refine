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
//! Run (from the riir-clippy repo root):
//!
//! ```text
//! cargo run --example score_bench_mine_v6 --features score_bench -- \
//!     ../riir-chain 7421245a~1 7421245a --min-lints 2
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use syn::spanned::Spanned;

use riir_clippy::score_bench::{vocabulary, wrap};

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
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |name: &str| -> Option<usize> {
        args.iter()
            .position(|a| a == name)
            .and_then(|p| args.get(p + 1))
            .and_then(|v| v.parse().ok())
    };
    let min_lints = opt("--min-lints").unwrap_or(2);
    let max_files = opt("--files").unwrap_or(30);
    let tree = args
        .iter()
        .position(|a| a == "--tree")
        .and_then(|p| args.get(p + 1))
        .cloned();
    let positional: Vec<String> = args
        .into_iter()
        .filter(|a| !a.starts_with("--") && a.parse::<usize>().is_err())
        .filter(|a| Some(a.as_str()) != tree.as_deref())
        .collect();
    let [repo, sha, sweep] = positional.as_slice() else {
        eprintln!(
            "usage: score_bench_mine_v6 <repo> <sha> <sweep-commit> [--min-lints N] [--files N]"
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

    let files = sweep_files(&cfg);
    eprintln!(
        "# {repo_name}: {} file(s) from {}; mining at {}",
        files.len(),
        cfg.tree.as_ref().map_or_else(|| format!("sweep {}", cfg.sweep), |t| format!("tree {t}")),
        cfg.sha
    );

    // Carve candidates: (id, provenance-path, tiered code variants).
    let mut candidates: Vec<Cand> = Vec::new();
    for path in &files {
        let Some(src) = git_show(&cfg, path) else { continue };
        let Ok(file) = syn::parse_file(&src) else {
            eprintln!("# {path}: parse failed — skipped");
            continue;
        };
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
            continue; // nothing to scaffold with — the bare crust
        }
        let mut idx = 0usize;
        for f in &fns {
            candidates.push(build_cand(&repo_name, stem, path, &mut idx, "fn", f, &header, &macros, &impls, &fns));
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
            candidates.push(build_cand(&repo_name, stem, path, &mut idx, "impl", &text, &header, &macros, &impls, &fns));
        }
    }
    eprintln!("# {} carve candidate(s)", candidates.len());

    // Probe tiers: T1 for all; T2 for T1 failures; T3 for T2 failures.
    let t1: Vec<String> = candidates.iter().map(|c| wrap(&c.t1)).collect();
    let ok1 = batch_compile(&oracle, &t1);
    let t2_idx: Vec<usize> = (0..candidates.len()).filter(|&i| !ok1[i]).collect();
    let t2: Vec<String> = t2_idx.iter().map(|&i| wrap(&candidates[i].t2)).collect();
    let ok2 = batch_compile(&oracle, &t2);
    let t3_idx: Vec<usize> = t2_idx
        .iter()
        .zip(&ok2)
        .filter(|(_, &ok)| !ok)
        .map(|(&i, _)| i)
        .collect();
    let t3: Vec<String> = t3_idx
        .iter()
        .filter_map(|&i| candidates[i].t3.as_ref().map(|c| wrap(c)))
        .collect();
    let ok3 = batch_compile(&oracle, &t3);

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
    let mut labels: Vec<Option<BTreeSet<String>>> = vec![None; candidates.len()];
    let mut slot = 0usize;
    for chunk in compiled.chunks(CHUNK) {
        let snippets: Vec<String> = chunk
            .iter()
            .map(|&i| wrap(final_code[i].as_ref().unwrap_or(&String::new())))
            .collect();
        let refs: Vec<&str> = snippets.iter().map(String::as_str).collect();
        let verdicts = oracle.lints_for_batch(&refs);
        for (&ci, v) in chunk.iter().zip(verdicts) {
            if v.ran && v.compiles {
                labels[ci] = Some(v.lints.intersection(&vocab).cloned().collect());
            }
        }
        slot += chunk.len();
    }
    let _ = slot;

    // Census + emission.
    let mut census: BTreeMap<usize, usize> = BTreeMap::new();
    let mut class_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut emitted = 0usize;
    let mut controls: Vec<(usize, &BTreeSet<String>)> = Vec::new();
    for (i, lints) in labels.iter().enumerate() {
        let Some(lints) = lints else { continue };
        let k = lints.len();
        *census.entry(k).or_insert(0) += 1;
        for l in lints {
            *class_count.entry(l.clone()).or_insert(0) += 1;
        }
        if k >= cfg.min_lints {
            emit(&repo_name, &cfg.sha, &candidates[i], final_code[i].as_deref().unwrap_or(""), lints, emitted);
            emitted += 1;
        } else if k == 1 {
            controls.push((i, lints));
        }
    }
    eprintln!("\n# carve census (distinct-vocab-lint count → carves): {census:?}");
    eprintln!("# lint-class frequency: {class_count:?}");
    eprintln!("# emitted {emitted} multi carve(s) (min-lints {})", cfg.min_lints);
    eprintln!("# single control candidates (k=1, first 15):");
    for (n, (i, lints)) in controls.iter().take(15).enumerate() {
        emit(&repo_name, &cfg.sha, &candidates[*i], final_code[*i].as_deref().unwrap_or(""), lints, n);
    }
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
    repo: &str,
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

/// Rank sweep-healed files by diff magnitude (changed lines), top N — or,
/// in `--tree` mode, list tracked .rs files under a path prefix (up to a
/// sane probe budget, sorted by path for determinism).
fn sweep_files(cfg: &Cfg) -> Vec<String> {
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
        return String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| l.ends_with(".rs"))
            .take(60)
            .map(ToString::to_string)
            .collect();
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
                .and_then(|s| s.trim().split_whitespace().next())
                .and_then(|s| s.parse().ok())?;
            let path = path.trim().to_string();
            if path.ends_with(".rs") && !path.contains("tests/") && !path.contains("/tests") {
                Some((n, path))
            } else {
                None
            }
        })
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.truncate(cfg.max_files);
    rows.into_iter().map(|(_, p)| p).collect()
}

fn git_show(cfg: &Cfg, path: &str) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(&cfg.repo)
        .args(["show", &format!("{}:{path}", cfg.sha)])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Emit one `RawFixture` entry (paste into generations.rs).
fn emit(
    repo: &str,
    sha: &str,
    cand: &Cand,
    code: &str,
    lints: &BTreeSet<String>,
    idx: usize,
) {
    let lints: Vec<String> = lints.iter().map(|s| format!("{s:?}")).collect();
    let code_lit = rust_string_literal(code);
    let file = &cand.path;
    let id = format!("{}-{idx:03}", cand.id);
    eprintln!(
        "    // CANDIDATE {id} | provenance: {repo}@{sha}:{file} (file-header carve)\n    RawFixture {{\n        id: \"{id}\",\n        provenance: \"{repo}@{sha}:{file} (file-header carve)\",\n        code: {code_lit},\n        expect: &[{}],\n    }},\n",
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
