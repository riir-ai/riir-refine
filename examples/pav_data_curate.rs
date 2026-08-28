//! Plan 356 A1 — PAV dataset curation campaign (the plan's data half).
//!
//! Two modes:
//!
//! - **Dry-run (default, CPU, seconds):** a seeded mock generator walks the
//!   pipeline end-to-end — span loading, prefix tree, MC-Q, first-pit
//!   curation, JSONL + manifest. Proves the harness before the GPU budget
//!   is spent.
//! - **Campaign (`--daemon-bin`):** the real riir-train fix daemon
//!   (`bonsai_clippy_l4_eval --serve` binary) generates every edit. This is
//!   the ~30–40 h exclusive-4090 budget from the plan — run it only when
//!   the box is free.
//!
//! # Run
//!
//! ```sh
//! # Dry-run over the frozen fixtures (CPU only):
//! cargo run --release --features pav_data --example pav_data_curate -- \
//!     --out /tmp/pav_dry.jsonl
//!
//! # Real campaign (4090, when free):
//! cargo run --release --features pav_data --example pav_data_curate -- \
//!     --out data/pav_pairs_v1.jsonl \
//!     --spans fixtures,store --store-root . \
//!     --daemon-bin ../riir-train/target/release/examples/bonsai_clippy_l4_eval \
//!     --gguf ../riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
//!     --lora <trained-ckpt.bin> --ground-truth
//! ```

#![cfg(feature = "pav_data")]

use riir_clippy::pav_data::{
    curate, extra_spans, fixture_spans, ground_truth, store_spans, write_jsonl,
    DaemonEditGenerator, EditGenerator, FnGenerator, PavDataConfig, PavDataset, PavManifest,
    PavSpan, RolloutCtx, VocabularyJudge,
};

fn usage() -> ! {
    eprintln!(
        "usage: pav_data_curate --out <pairs.jsonl> [--spans fixtures,store,<extra.jsonl>] \
         [--store-root <dir>] [--limit N] [--seed S] [--n-cov N] [--n-mc N] [--max-steps N] \
         [--eps-pit F] [--max-prefixes N] [--ground-truth] \
         [--daemon-bin <path> --gguf <path> --lora <path> [--max-new N]]"
    );
    std::process::exit(2);
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == name {
            return it.next().cloned();
        }
    }
    None
}

fn num<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    flag(args, name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// The dry-run generator: deterministic, CPU, marker-free — rewrites each
/// state into a parseable variant so the tree walks and the budget/parse
/// paths are exercised without a model. NOT a quality signal; it exists so
/// the plumbing is proven before the GPU budget is spent.
fn dry_run_generator() -> FnGenerator<impl FnMut(&RolloutCtx<'_>) -> Option<String>> {
    FnGenerator::new("dry-run", |ctx| {
        // Every 3rd rollout declines; every 5th emits an unparseable
        // fragment (the L2-gate path); the rest produce a parseable state
        // the vocabulary judge will keep scanning (real lint sites are
        // exercised by the fixture spans themselves).
        match (ctx.rollout_id + u64::from(ctx.call_index)) % 5 {
            0 => None,
            1 => Some("fn broken( {{{".to_string()),
            k => Some(format!(
                "// dry-run step {k}\nfn dryrun_{}() {{}}\n",
                ctx.depth
            )),
        }
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(out) = flag(&args, "--out") else {
        usage();
    };
    let out = std::path::PathBuf::from(out);

    let cfg = PavDataConfig {
        n_cov: num(&args, "--n-cov", 8usize),
        n_mc: num(&args, "--n-mc", 5usize),
        max_steps: num(&args, "--max-steps", 6u32),
        max_prefixes_per_span: num(&args, "--max-prefixes", 64usize),
        eps_pit: num(&args, "--eps-pit", 0.10f32),
        seed: num(&args, "--seed", 0u64),
        ground_truth: args.iter().any(|a| a == "--ground-truth"),
    };

    // ── Spans ────────────────────────────────────────────────────────────
    let spec = flag(&args, "--spans").unwrap_or_else(|| "fixtures".to_string());
    let mut spans: Vec<PavSpan> = Vec::new();
    for part in spec.split(',') {
        match part {
            "fixtures" => {
                let n = fixture_spans().len();
                println!("[spans] fixtures: {n}");
                spans.extend(fixture_spans());
            }
            "store" => {
                let root = flag(&args, "--store-root")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("."));
                match store_spans(&root) {
                    Ok((s, skipped)) => {
                        println!(
                            "[spans] store: {} spans ({skipped} skipped: withdrawn/unparseable)",
                            s.len()
                        );
                        spans.extend(s);
                    }
                    Err(e) => eprintln!("[spans] store SKIPPED: {e}"),
                }
            }
            p if p.ends_with(".jsonl") => match extra_spans(std::path::Path::new(p)) {
                Ok(s) => {
                    println!("[spans] extra {p}: {}", s.len());
                    spans.extend(s);
                }
                Err(e) => eprintln!("[spans] extra {p} SKIPPED: {e}"),
            },
            p => eprintln!("[spans] unknown source '{p}' (expected fixtures|store|<*.jsonl>)"),
        }
    }
    if let Some(limit) = flag(&args, "--limit").and_then(|v| v.parse::<usize>().ok()) {
        spans.truncate(limit);
    }
    if spans.is_empty() {
        eprintln!("[spans] no spans loaded — nothing to do");
        usage();
    }
    println!("[spans] {} total (regime: {})", spans.len(), cfg.regime());

    // ── Generator + judge ────────────────────────────────────────
    let judge = VocabularyJudge;
    let started = std::time::Instant::now();
    let (mut dataset, generator_name) = if let Some(bin) = flag(&args, "--daemon-bin") {
        let gguf = flag(&args, "--gguf").unwrap_or_else(|| usage());
        let lora = flag(&args, "--lora").unwrap_or_else(|| usage());
        let max_new = num(&args, "--max-new", 96usize);
        println!("[generator] spawning daemon: {bin} (max_new {max_new})…");
        let mut generator = DaemonEditGenerator::spawn(&bin, &gguf, &lora, max_new)
            .unwrap_or_else(|e| panic!("daemon spawn failed: {e}"));
        let ds = curate(&spans, &mut generator, &judge, &cfg);
        (ds, generator.name().to_string())
    } else {
        println!("[generator] DRY-RUN (seeded mock; no model calls)");
        let mut generator = dry_run_generator();
        let ds = curate(&spans, &mut generator, &judge, &cfg);
        (ds, generator.name().to_string())
    };

    // ── Ground truth (opt-in, slow: one cargo run per curated pair) ─
    if cfg.ground_truth {
        match ground_truth(&mut dataset.pairs) {
            Ok(n) => println!("[ground] {n} curated terminal pairs verified by real clippy"),
            Err(e) => eprintln!("[ground] SKIPPED: {e}"),
        }
    }
    finish(&out, &dataset, &generator_name, &cfg, started);
}

fn finish(
    out: &std::path::Path,
    dataset: &PavDataset,
    generator_name: &str,
    cfg: &PavDataConfig,
    started: std::time::Instant,
) {
    let manifest = PavManifest {
        schema_version: riir_clippy::pav_data::pav_schema_version(),
        bok_k: riir_clippy::pav_data::BOK_K,
        generator: generator_name.to_string(),
        judge: "vocabulary".to_string(),
        n_cov: cfg.n_cov,
        n_mc: cfg.n_mc,
        max_steps: cfg.max_steps,
        eps_pit: cfg.eps_pit,
        seed: cfg.seed,
        stats: dataset.stats.clone(),
    };
    write_jsonl(out, dataset, &manifest).unwrap_or_else(|e| panic!("write {}: {e}", out.display()));
    let s = &dataset.stats;
    println!(
        "[done] {} pairs ({} kept / {} pit-dropped) from {} spans in {:.1}s → {}",
        s.pairs,
        s.pairs_kept,
        s.pairs_dropped_pit,
        s.spans_total,
        started.elapsed().as_secs_f32(),
        out.display()
    );
    println!(
        "[done] buckets: {} low (<0.5) / {} high (>=0.5); rollouts {} {:?}",
        s.bucket_low, s.bucket_high, s.rollouts_total, s.outcomes
    );
}
