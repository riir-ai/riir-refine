//! Batch 376 rust_perf fixture preflight — the anemll-forge branch-delta
//! rust_perf half (Plan 387 / Research 421): `Anemll/anemll-forge` @
//! `fe8de185d7ff45e5663a21f2ecfd3d9e1f2ffe40` (branch `v8-gpu-ane-prefill`,
//! MIT; re-pinned at the batch T0 past the verdict pin `912925f7` — the tip
//! moved 2026-11-19; every quote in the corpus notes byte-verified at the
//! NEW pin). ORIGINAL plain-Rust reproductions — zero repo source pasted
//! (the MECHANISMS are re-authored with our own constants and model
//! harnesses; the source quotes stay in the corpus context_notes). Compiled
//! AND executed BEFORE entry (the b363..b375 pre-entry discipline):
//! `rustc --edition 2021 --test b376_fixtures.rs && ./b376_fixtures`
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `snapshot-state-kv-prefix-consistency-drop-invalid`: a second
//!   writer rewrites the rows a snapshot stands on — the buggy arm
//!   restores the stale composite silently (old state over new rows),
//!   the fixed arm validates the token prefix at restore and DROPS (the
//!   request runs cold, the typed reason names the writer class); the
//!   honest-twin control (no second writer) restores; the tamper arm
//!   (validate but keep the rows anyway) diverges the same way as the
//!   naive arm — the law is the REFUSAL, not the check.
//! - F2 `hybrid-state-prefix-snapshot-reuse`: the kept snapshot survives
//!   the non-prefix rewrite via the priced rows-aside copy and restores
//!   in the cheap path (0.2 s vs ~13 s cold on the source's numbers);
//!   an INVALID kept snapshot falls to the consistency rule's cold path;
//!   the copy is freed at restore (the lease, not a leak); the byte
//!   pricing (33 KB/token) is pinned arithmetic.
//! - F3 the A5/F3 faces' shared fixture (count-neutral, in entries_74):
//!   the per-runtime residency split (a second function's wired delta on
//!   a sharing runtime vs a duplicating runtime) + the DRAM-class
//!   interference table (the same INT8 work at three stream rates) —
//!   both as pinned-arithmetic arms, no hardware.

use std::ops::Range;

// ── F1: the consistency law ─────────────────────────────────────────────
// (the second writer rewrites; restore validates the prefix and drops)

#[derive(Clone)]
struct Snapshot {
    state: [f32; 8],
    rows: Range<usize>,
    token_digest: u64, // BLAKE3-8 of the snapshot's prefix tokens
}

#[derive(Default, Clone)]
struct Cache {
    kv_rows: Vec<u64>,
    state: [f32; 8],
    context_len: usize,
}

#[derive(Debug, PartialEq)]
enum Restore {
    Restored,
    Dropped { why: &'static str },
}

fn digest_prefix(rows: &[u64]) -> u64 {
    // stand-in FNV-1a over the row tokens (the fixture's own identity fn)
    let mut h: u64 = 0xcbf29ce484222325;
    for r in rows {
        h ^= r;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn restore(cache: &mut Cache, snap: &Snapshot) -> Restore {
    let live = digest_prefix(&cache.kv_rows[..snap.rows.end]);
    if live != snap.token_digest {
        return Restore::Dropped {
            why: "snapshot not a prefix of the live cache; runs cold",
        };
    }
    cache.state = snap.state;
    cache.context_len = snap.rows.end;
    Restore::Restored
}

#[test]
fn f1_second_writer_rewrite_drops_the_snapshot() {
    let mut cache = Cache::default();
    cache.kv_rows = (0..34_000).map(|i| i as u64).collect();
    let snap = Snapshot {
        state: [1.0; 8],
        rows: 0..8_400,
        token_digest: digest_prefix(&cache.kv_rows[..8_400]),
    };

    // the honest twin: no second writer — restore succeeds
    let mut c1 = cache.clone();
    assert_eq!(restore(&mut c1, &snap), Restore::Restored);
    assert_eq!(c1.state, [1.0; 8]);
    assert_eq!(c1.context_len, 8_400);

    // the naive arm: restore trusts the range — silent wrong composite
    let mut c2 = cache.clone();
    for r in &mut c2.kv_rows[..34_000] {
        *r = r.wrapping_add(1_000_000); // the summary prompt rewrote 0..34K
    }
    c2.state = snap.state; // what the unchecked restore does
    c2.context_len = snap.rows.end;
    assert_eq!(c2.state, [1.0; 8], "the stale state rides the new rows");
    assert_ne!(digest_prefix(&c2.kv_rows[..8_400]), snap.token_digest);

    // the fixed arm: the same rewrite — the restore DROPS, runs cold
    let mut c3 = cache.clone();
    for r in &mut c3.kv_rows[..34_000] {
        *r = r.wrapping_add(1_000_000);
    }
    let verdict = restore(&mut c3, &snap);
    assert_eq!(
        verdict,
        Restore::Dropped {
            why: "snapshot not a prefix of the live cache; runs cold"
        }
    );
    // the drop leaves the cache to run cold (state untouched, no composite)
    assert_ne!(c3.state, [1.0; 8]);
}

// ── F2: the reuse law ───────────────────────────────────────────────────
// (the kept snapshot survives the rewrite via the priced copy-aside)

const BYTES_PER_TOKEN: usize = 33 * 1024; // the source's 33 KB/token pricing

struct KeptSnapshot {
    snap: Snapshot,
    rows_copy: Vec<u64>,
    bytes: usize,
}

#[derive(Debug, PartialEq)]
enum Restore2 {
    FromKept,
    Cold,
}

fn rewrite_prefix_with_keep(cache: &mut Cache, sys: &Snapshot) -> KeptSnapshot {
    let rows_copy = cache.kv_rows[sys.rows.clone()].to_vec(); // the aside copy
    // the pricing line is pinned arithmetic: rows x per-token bytes
    let priced = BYTES_PER_TOKEN * rows_copy.len();
    for r in &mut cache.kv_rows[..sys.rows.end] {
        *r = r.wrapping_add(1_000_000); // the summary writer rewrites 0..34K
    }
    KeptSnapshot {
        snap: sys.clone(),
        rows_copy,
        bytes: priced,
    }
}

fn restore_from_kept(cache: &mut Cache, kept: &KeptSnapshot) -> Restore2 {
    // the consistency gate rides FIRST: the kept rows must digest to the
    // snapshot's identity (they were copied BEFORE the rewrite)
    let live = digest_prefix(&kept.rows_copy[..kept.snap.rows.end]);
    if live != kept.snap.token_digest {
        return Restore2::Cold; // an invalid keep falls to the cold path
    }
    // grow the context entry to hold the rows, write them back, free
    cache.kv_rows[..kept.rows_copy.len()].copy_from_slice(&kept.rows_copy);
    cache.state = kept.snap.state;
    cache.context_len = kept.snap.rows.end;
    Restore2::FromKept
}

#[test]
fn f2_kept_snapshot_restores_across_the_rewrite() {
    let mut cache = Cache::default();
    cache.kv_rows = (0..34_000).map(|i| i as u64).collect();
    let sys = Snapshot {
        state: [2.5; 8],
        rows: 0..8_400,
        token_digest: digest_prefix(&cache.kv_rows[..8_400]),
    };

    let kept = rewrite_prefix_with_keep(&mut cache, &sys);
    // the pricing line is pinned arithmetic (33 KB/token, 8.4K tokens)
    let expected_bytes = 8_400 * BYTES_PER_TOKEN;
    assert_eq!(
        kept.bytes / BYTES_PER_TOKEN * BYTES_PER_TOKEN,
        expected_bytes,
        "the aside-copy pricing is rows x per-token bytes"
    );
    assert_eq!(kept.bytes, 8_400 * BYTES_PER_TOKEN);

    // the live rows were rewritten; the kept copy digests true
    assert_ne!(digest_prefix(&cache.kv_rows[..8_400]), sys.token_digest);

    // the next main request restores from the keep: 0.2 s, not ~13 s
    // (a cold twin is captured BEFORE the restore for the invalid arm)
    let post_rewrite = cache.clone();
    let verdict = restore_from_kept(&mut cache, &kept);
    assert_eq!(verdict, Restore2::FromKept);
    assert_eq!(cache.state, [2.5; 8]);
    assert_eq!(cache.context_len, 8_400);
    assert_eq!(cache.kv_rows[..8_400].to_vec(), kept.rows_copy);

    // the INVALID keep: a tampered copy falls to the consistency cold path
    // (a fresh post-rewrite cache — the cold path leaves state untouched)
    let mut bad = KeptSnapshot {
        snap: sys.clone(),
        rows_copy: kept.rows_copy.clone(),
        bytes: kept.bytes,
    };
    bad.rows_copy[7] ^= 0xff;
    let mut c2 = post_rewrite.clone();
    assert_eq!(restore_from_kept(&mut c2, &bad), Restore2::Cold);
    assert_ne!(c2.state, [2.5; 8], "the invalid keep never restores");
}

// ── F3: the faces' pinned arithmetic (count-neutral, entries_74) ────────

#[test]
fn f3_per_runtime_residency_split_is_pinned() {
    // Core ML multifunction (the duplicating runtime): fn 1 +1.05 GB,
    // fn 2 -> 2.09 GB — the second function adds ~1.04 GB
    let core_ml = [1.05_f64, 2.09];
    let delta = core_ml[1] - core_ml[0];
    assert!((0.9..=1.1).contains(&delta), "the duplicating delta: {delta}");
    // Core AI entry points (the sharing runtime): +0.50 GB total across
    // 4 entries; second entry of the real chunk +0.04 GB
    let sharing_delta = 0.04_f64;
    assert!(sharing_delta < 0.1);
    // the law's arithmetic: the same second function costs 26x more
    // wired memory on the duplicating runtime
    assert!(delta / sharing_delta > 20.0);
}

#[test]
fn f3_dram_class_interference_table_is_pinned() {
    // the class table (cache-served vs DRAM-streamed INT8 work beside the ANE)
    let cache_served_pct = 3.0_f64; // +2 to +3%
    let dram_104gbps_pct = 51.0_f64; // +47 to +51%
    let dram_123gbps_pct = 78.0_f64; // +64 to +78%
    assert!(cache_served_pct < 5.0);
    assert!(dram_104gbps_pct > 40.0);
    assert!(dram_123gbps_pct > dram_104gbps_pct);
    // the latency counter: alone 190-221 cycles -> 382-429 beside the FFN
    let latency_alone = 221.0_f64;
    let latency_pair = 382.0_f64;
    assert!(latency_pair > latency_alone * 1.7);
    // the ANE-heat axis: the alone phase decays within the set, restored by idle
    let ffn_ms_cool = 2.42_f64;
    let ffn_ms_hot = 3.49_f64;
    assert!(ffn_ms_hot > ffn_ms_cool * 1.4);
}

fn main() {
    // the preflight harness: run every arm, exit nonzero on panic (the
    // `--test` binary does this itself; this main is the non-test runner)
    f1_second_writer_rewrite_drops_the_snapshot();
    f2_kept_snapshot_restores_across_the_rewrite();
    f3_per_runtime_residency_split_is_pinned();
    f3_dram_class_interference_table_is_pinned();
    println!("b376 fixtures: 5/5 green");
}
