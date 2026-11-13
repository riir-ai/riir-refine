//! Issue 233 T1.0 — the repeated-span-query census (the SparseRead
//! reading-control wiring's premise gate, arXiv:2608.22237 via
//! `.research/404`).
//!
//! **The gate:** count repeated span queries on the oracle-fed battery
//! BEFORE building the read-episode record — a near-zero count defers T1
//! with the count recorded. This example is the instrument AND its durable
//! re-run lane: it replicates the scan's chunk walk EXACTLY (the same
//! [`collect_source_files`] file walk, the same [`AstChunker`] spans the
//! matcher queries — one retrieval query per chunk), keys each chunk by
//! `blake3(content)`, and reports the repeat structure.
//!
//! **Why content-keyed:** the retrieval result for a chunk is a pure
//! function of (content, allow-mask, top_k) — identical content re-queried
//! in the same run re-pays the embed + per-domain index scan for an
//! identical answer. The T1 read-episode record's key is exactly this
//! fingerprint (blake3, not SHA-256 — the SparseRead fingerprint law).
//!
//! **What is counted** (one pass over the battery, no product patch):
//! - `chunk queries` — every chunk the scan would query;
//! - `unique contents` — distinct `blake3(content)` keys;
//! - `repeat queries` — queries whose key was already queried (the
//!   skippable class); `repeat share` = repeat/total;
//! - `query bytes` / `unique bytes` — the exposure axis (Σ content bytes
//!   queried vs the minimum a content-keyed episode record would pay);
//! - the size buckets (queries/repeats per bucket) — the cost-weighted
//!   half: a repeat's saved work is per-content work (embed + parse +
//!   index scan), so WHERE the repeats live matters as much as how many;
//! - the top repeated contents (count + bytes + a sample path pair), so a
//!   verdict can name WHAT repeats (generated code, trait stubs, …).
//!
//! **Report-only:** prints the census, exits 0. A near-zero repeat share
//! is the DEFER verdict for T1 (recorded in the issue, never retried
//! silently); a substantial share is T1's justification — read together
//! with the buckets, which price it.
//!
//! Run: `cargo run --release --example read_episode_census -- <repo-root>`
//! (default: the current directory).

use std::collections::HashMap;
use std::path::PathBuf;

use riir_refine::draft::span_source::collect_source_files;
use riir_refine::ragplane::ast_chunker::AstChunker;
use riir_refine::ragplane::chunker::Chunker as _;

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let mut files = Vec::new();
    collect_source_files(&root, ".rs", &mut files);
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!(
            "read-episode census: no .rs files under {} — a blind walk is never a green zero",
            root.display()
        );
        std::process::exit(1);
    }

    let chunker = AstChunker;
    let mut total_queries: u64 = 0;
    let mut unique_contents: u64 = 0;
    let mut repeat_queries: u64 = 0;
    let mut query_bytes: u64 = 0;
    let mut unique_bytes: u64 = 0;
    // blake3 hex → (count, bytes, first path, second path)
    let mut seen: HashMap<String, (u64, u64, String, Option<String>)> = HashMap::new();
    // (bytes, is_repeat) per query — the bucket pass reads this.
    let mut per_query: Vec<(u64, bool)> = Vec::new();
    let mut unparseable_files: u64 = 0;

    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        for chunk in chunker.chunk(&src, &f.display().to_string()) {
            let bytes = chunk.content.len() as u64;
            total_queries += 1;
            query_bytes += bytes;
            let digest = blake3::hash(chunk.content.as_bytes()).to_hex().to_string();
            let entry = seen.entry(digest).or_insert_with(|| {
                unique_contents += 1;
                unique_bytes += bytes;
                (0, bytes, f.display().to_string(), None)
            });
            entry.0 += 1;
            let is_repeat = entry.0 > 1;
            if entry.0 == 2 {
                // The FIRST repeat of this content — name the pair.
                entry.3 = Some(f.display().to_string());
            }
            if is_repeat {
                repeat_queries += 1;
            }
            per_query.push((bytes, is_repeat));
        }
        // The chunker's parse-fail shape is one whole-file chunk; count the
        // files whose content never parsed so the census discloses them.
        if syn::parse_file(&src).is_err() {
            unparseable_files += 1;
        }
    }

    let repeat_share = f64::from(repeat_queries as u32) / total_queries.max(1) as f64 * 100.0;
    let byte_share = (query_bytes - unique_bytes) as f64 / query_bytes.max(1) as f64 * 100.0;

    println!(
        "read-episode census (i233 T1.0 — report-only): {}",
        root.display()
    );
    println!(
        "files: {} walked ({unparseable_files} unparseable → whole-file chunks)",
        files.len()
    );
    println!(
        "chunk queries: {total_queries} — unique contents: {unique_contents} — repeat queries: \
         {repeat_queries} ({repeat_share:.1}%)"
    );
    println!(
        "query bytes: {query_bytes} — unique bytes: {unique_bytes} — repeat bytes: {} \
         ({byte_share:.1}%)",
        query_bytes - unique_bytes
    );

    // The cost-weighted half: a repeat's saved work is per-content work
    // (embed + syn parse + per-domain index scan); WHERE the repeats live
    // matters as much as how many.
    type Bucket = (&'static str, fn(u64) -> bool);
    const BUCKETS: [Bucket; 4] = [
        ("<64B", |b| b < 64),
        ("64B-1K", |b| (64..1024).contains(&b)),
        ("1K-8K", |b| (1024..8192).contains(&b)),
        (">8K", |b| b >= 8192),
    ];
    println!("size buckets (queries / repeats / repeat%):");
    for (name, in_bucket) in BUCKETS {
        let mut q = 0u64;
        let mut r = 0u64;
        for &(bytes, is_repeat) in &per_query {
            if in_bucket(bytes) {
                q += 1;
                if is_repeat {
                    r += 1;
                }
            }
        }
        let share = r as f64 / q.max(1) as f64 * 100.0;
        println!("  {name:>6}: {q:>6} / {r:>6} / {share:5.1}%");
    }

    // The top repeated contents, by query count then bytes.
    let mut rows: Vec<_> = seen
        .iter()
        .filter(|(_, (c, ..))| *c > 1)
        .map(|(k, (c, b, p1, p2))| (*c, *b, k.as_str(), p1.as_str(), p2.as_deref().unwrap_or("")))
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    if !rows.is_empty() {
        println!("top repeated contents (count, bytes, first path → repeat path):");
        for (c, b, k, p1, p2) in rows.iter().take(8) {
            println!(
                "  {c}x {b}B  {}…  {} → {}",
                &k[..12],
                short_path(p1),
                short_path(p2)
            );
        }
    } else {
        println!("top repeated contents: NONE — every queried chunk is content-distinct");
    }
}

/// Home-relative short path for the sample rows (readability only).
fn short_path(p: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    p.strip_prefix(&home)
        .map(|r| format!("~{r}"))
        .unwrap_or_else(|| p.to_string())
}
