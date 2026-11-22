// Batch 398 — the mlx-serve return-visit delta (Plan 398 / Research 432,
// ddalcu/mlx-serve @ 96311b2638cf2f0ebdc7a2e39c85882a4fc4654a, tip
// 2026-11-22 "Inkling 2 Mini", MIT; the verdict pin 2fe9be83 ONE commit
// earlier — test-only delta; the controlling PRIOR visit: Research 274 /
// Batch 242 landed 31 rules @ 1015f78 2026-10-14, every candidate here
// survived the B242 dedup re-run).
//
// rust_perf 386 → 387: F13 `prefix-commit-priced-worthiness-typed-outcome`
// (caching) — the hot prefix cache's COMMIT side prices worthiness ITSELF
// (a commit-time floor over cancelled-prefill entries: chat-template
// prologues are identical across every request and "reusable" only in a
// worthless sense — below the floor an entry is LRU pollution that evicts
// genuinely hot entries) and the commit outcome is TYPED (a budget decline
// that exits via plain return reads as ok to the caller, which logged
// "committed N/M" while a 122k agent session cold-prefilled ~95k tokens
// per retry — the loud-verdict doctrine's second instance). The batch's
// kernel half (8 rules) lands in kernel_opt/corpus/entries_gpu_161.rs and
// the sec half (the DiscoveryEgress family seed) in
// src/domains/sec/corpus.rs in the same commit.
//
// Fixtures `fixtures/b398_perf_fixtures.rs` are ORIGINAL plain-Rust
// reproductions (zero repo source pasted), standalone-compiled AND
// executed BEFORE entry (debug AND -O). NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 398 — the mlx-serve delta's `rust_perf` half.
#[must_use]
pub fn b398_rules() -> Vec<PerfRule> {
    vec![PerfRule {
        rule_id: "prefix-commit-priced-worthiness-typed-outcome".to_string(),
        name: "Prefix cache: price worthiness at COMMIT time; type the outcome"
            .to_string(),
        category: "caching".to_string(),
        buggy_code: r#"/// NAIVE: every cancelled prefill commits — claim-time worthiness was
/// checked when the request started; commit re-checks nothing. And a
/// budget decline exits via plain return, so the caller logs
/// "committed N/M" for it. Two costs: chat-template prologues (8-12
/// tokens) commit as "reusable", pollute the LRU and evict genuinely
/// hot entries; and the scheduler trusts a log that lies.
pub fn commit_cancelled_prefix(&mut self, prompt_ids: &[u32]) -> bool {
    self.entries.push(CacheEntry {
        key: prompt_ids.to_vec(),
        snapshot: self.capture_snapshot(), // the expensive side
    });
    true // every commit "succeeds"
}

pub fn commit_with_budget(&mut self, prompt_ids: &[u32]) -> bool {
    if self.resident_bytes() + self.entry_bytes() > self.budget {
        return false; // silent decline — the caller cannot tell WHY,
        // and the scheduler logs a commit that never happened
    }
    self.commit_cancelled_prefix(prompt_ids)
}

struct CacheEntry {
    key: Vec<u32>,
    snapshot: Snapshot,
}

struct CacheState {
    entries: Vec<CacheEntry>,
    budget: usize,
}

impl CacheState {
    fn capture_snapshot(&self) -> Snapshot {
        Snapshot
    }
    fn resident_bytes(&self) -> usize {
        0
    }
    fn entry_bytes(&self) -> usize {
        4096
    }
}

struct Snapshot;
"#.to_string(),
        fixed_code: r#"/// The commit side prices worthiness ITSELF (a COMMIT-time floor: the
/// expensive capture is skipped for entries below the worthiness floor —
/// chat-template prologues are identical across every request and
/// "reusable" only in a worthless sense), and the outcome is TYPED so
/// callers log the truth: a decline is a value the scheduler can see,
/// never a plain return that reads as ok. Ok carries the post-trim
/// EFFECTIVE length, never the candidate's forwarded length.
pub const MIN_CANCELLED_COMMIT_TOKENS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitStatus {
    /// committed at this many tokens — the post-trim EFFECTIVE length,
    /// never the candidate's forwarded length
    Ok(usize),
    /// full state captured for the disk tier; no RAM entry retained
    DiskOnly(usize),
    /// the budget decline kept a resident entry already covering this
    /// many tokens; the longer candidate was discarded
    KeptResident(usize),
    /// declined: below the worthiness floor, or oversized with no
    /// restorable trim under the budget — the decline site names it
    Declined,
}

pub fn commit_cancelled(&mut self, prompt_ids: &[u32]) -> CommitStatus {
    // the COMMIT-time worthiness floor: below it, skip the expensive
    // capture — the entry would be LRU pollution
    if prompt_ids.len() < MIN_CANCELLED_COMMIT_TOKENS {
        return CommitStatus::Declined;
    }
    if self.resident_bytes() + self.entry_bytes() > self.budget {
        if let Some(kept) = self.longest_resident_covering(prompt_ids.len()) {
            return CommitStatus::KeptResident(kept);
        }
        return CommitStatus::Declined;
    }
    self.entries.push(CacheEntry {
        key: prompt_ids.to_vec(),
        snapshot: self.capture_snapshot(),
    });
    CommitStatus::Ok(self.trimmed_len(prompt_ids))
}

struct CacheEntry {
    key: Vec<u32>,
    snapshot: Snapshot,
}

struct CacheState {
    entries: Vec<CacheEntry>,
    budget: usize,
}

struct Snapshot;

impl CacheState {
    fn capture_snapshot(&self) -> Snapshot {
        Snapshot
    }
    fn resident_bytes(&self) -> usize {
        0
    }
    fn entry_bytes(&self) -> usize {
        4096
    }
    fn trimmed_len(&self, prompt_ids: &[u32]) -> usize {
        prompt_ids.len()
    }
    fn longest_resident_covering(&self, _tokens: usize) -> Option<usize> {
        None
    }
}
"#.to_string(),
        context_notes: Some(
            "THE INCIDENT (the typed-outcome half, verbatim at the pin, src/engines/mlx/cache/prefix_cache.zig:116-124 @ 96311b26): 'Outcome of a cache commit, so callers can log the truth. A budget decline used to exit commitWithMediaState via plain return and the scheduler logged \"committed N/M\" for it (a 122k agent session cold-prefilled ~95k tokens per retry while the log claimed cancelled-prefill commits every time).' — the perf cost of the lie: the scheduler trusted the log, the retries re-paid ~95k cold-prefill tokens each; the typed CommitStatus (ok/disk_only/kept_resident/declined — :126-136) makes the decline a VALUE the caller can see, log, and react to; ok carries 'the post-trim EFFECTIVE length, never the candidate's forwarded length' (:127-128). THE WORTHINESS FLOOR (verbatim :47-54): 'Minimum forwarded-prefix length for committing a CANCELLED prefill (client disconnect mid-prefill). Below this an entry is LRU pollution — chat-template prologues (Gemma=12, Qwen=8, Llama=4 tokens) are identical across every request and \"reusable\" only in a worthless sense. Same rationale as the llama session pool's min_prefix_to_claim, applied at commit time instead of claim time.' — MIN_CANCELLED_COMMIT_TOKENS 256; the CLAIM-time twin is cited (the llama session pool gates at claim); THIS is the COMMIT-time form, and the commit is the expensive side: it captures a KVCacheSnapshot of refcount-shared GPU buffers (the module doc, :3-10). Grade B− — suggestion-only, honestly NotBenchable-by-model (the win is serving-runtime cold-prefill avoidance + LRU quality over a KV-snapshot cache — no validator hosts the snapshot semantics; the floor's LRU-pollution effect is real but priced only in the serving runtime; the metric-mismatch class, `bench_spec: None`). ADJUDICATION both directions: reserve-then-commit-capacity-accounting (entries_58) owns the CAPACITY-accounting axis (reserve before commit, the commit accounting); THIS owns the WORTHINESS-pricing + typed-outcome axis (whether the entry is worth committing at all + whether the caller can tell what happened — compose on a budgeted cache: reserve the bytes AND price the worth AND type the outcome); the loud-verdict doctrine's second instance — latency-histogram-split-on-structural-populations (entries_80) carries the gap-not-guess face ('a guess in a metric is worse than a gap'), the same doctrine at a metric; here the log that claims commits is the guess. Category: caching. Suggest-only (the floor value and the outcome taxonomy are the product's design). Fixture: fixtures/b398_perf_fixtures.rs (the below-floor commit pollutes the LRU in the buggy arm and is refused in the fixed arm; the silent decline logs a lie in the buggy arm and types KeptResident/Declined in the fixed arm; the tampered arm — an Ok status carrying the candidate's forwarded length — reds). Batch 398 / Plan 398 / Research 432."
                .to_string(),
        ),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }]
}
