//! Entries slice — Batch 334 (2026-11-05): two `rust_perf` rules
//! from the nnethercote Nov-2026 rustc-perf distill (Plan 343 / Research 372).
//!
//! Source: rust-lang/rust PRs #163628 (single-entry parent `SpanData` cache,
//! merged 2026-10-03, 46+/17− across 3 files) + #163857 (overhaul span
//! compression, merged 2026-10-13, 325+/243− across 3 files) + #163971
//! (span-op micro-opts, jttriplett, best case >2%); the prose surface is the
//! month-2 Hexcat report (nnethercote.github.io 2026-11-04). Every quote
//! re-verified at the PR API + FULL-DIFF reads at claim time (T0). Fixtures
//! are ORIGINAL plain-Rust stand-ins (a same-parent keyed-lookup walk pair +
//! a skewed-payload pack/intern-tail pair), standalone rustc-compiled AND
//! executed before landing (rule 1: equivalence at n = 0/1/15/16/17/255/1K/
//! 16K + the exact hit/miss census — misses == the same-parent run starts,
//! 93.75% hit rate at GROUP=16, release direction check 4.60x at n=262K
//! best-of-15 alternating; rule 2: round-trip 2/2 arms at n up to `16_384` +
//! exact censuses — formats IC/IP/IPair/Interned = 70004/15247/13848/901 at
//! n=100K, interned 0.90%, fixed table 465 overflow-only entries vs buggy
//! 99,521 full-record entries, hot-field read table accesses structural 0).
//! Batch 334 / Plan 343 / Research 372.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── caching: a one-slot last-key memo in front of a hot hash
        // table ──────────────────────────────────────────────────────────
        // Source: rust-lang/rust #163628 "Add a single-entry parent
        // SpanData cache" (ich.rs SourceSpanCache + the encoder/decoder
        // call sites).
        PerfRule {
            rule_id: "single-entry-last-key-lookup-cache-hash-avoidance".to_string(),
            name: "A hot loop re-resolves a keyed value from a hash table on every iteration while the queries arrive in same-key streaks (child records naming one parent, items naming one owner, spans naming one def) → put a ONE-SLOT last-key memo (`Option<(K, V)>`) in front of the table: compare the query key against the cached key (one cheap Eq) BEFORE hashing + probing; on match reuse the cached value; on miss take the normal path and refresh the slot — the memo is owned by the reading pass and is sound only across a read-stable window of the map".to_string(),
            category: "caching".to_string(),
            buggy_code: r##"// NAIVE: a pass that iterates child records resolves each child's
// parent straight from the hash table — hash + probe + copy PER
// CHILD, even inside a long same-parent run (the shape behind
// metadata encode/decode/stable-hash passes, per-item config
// resolution, and any walk where consecutive queries repeat one key).

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rec {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

pub fn walk(
    children: &[(u32, u32)],
    parents: &std::collections::HashMap<u32, Rec>,
) -> u64 {
    let mut acc = 0u64;
    for &(_child, parent) in children {
        // the hot lookup: hash the key, probe the table, copy the
        // record — paid for EVERY child even inside a same-parent run
        let r: Rec = *parents.get(&parent).expect("parent missing");
        acc = acc
            .wrapping_add(r.a as u64)
            .wrapping_add(r.b as u64)
            .wrapping_add(r.c as u64)
            .wrapping_add(r.d as u64);
    }
    acc
}"##.to_string(),
            fixed_code: r##"// OPT: a ONE-SLOT last-key memo in front of the same table —
// `Option<(last_key, last_value)>`. One cheap Eq on the query key
// before any hashing; on match reuse the cached value; on miss take
// the normal path and refresh the slot. The memo is OWNED BY THE
// READING PASS (a local of the walk / a field of the encoder,
// decoder, or hasher session state) and is sound only across a
// READ-STABLE WINDOW of the map — invalidate or drop the memo at any
// observed mutation, never share it across threads or across a
// mutation point.

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rec {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

/// The single-entry cache.
pub struct LastKeyCache(pub Option<(u32, Rec)>);

impl LastKeyCache {
    #[inline]
    pub fn lookup(&mut self, key: u32, parents: &std::collections::HashMap<u32, Rec>) -> Rec {
        if let Some((last_key, rec)) = self.0 {
            if last_key == key {
                return rec; // the hit: one u32 Eq, no hash, no probe
            }
        }
        let rec = *parents.get(&key).expect("parent missing");
        self.0 = Some((key, rec));
        rec
    }
}

/// Census counters: hits, misses — pinned exact by the test.
#[derive(Default)]
pub struct Census {
    pub hits: usize,
    pub misses: usize,
}

pub fn walk(
    children: &[(u32, u32)],
    parents: &std::collections::HashMap<u32, Rec>,
    c: &mut Census,
) -> u64 {
    let mut cache = LastKeyCache(None);
    let mut acc = 0u64;
    for &(_child, parent) in children {
        let hit = cache.0.is_some_and(|(k, _)| k == parent);
        if hit {
            c.hits += 1;
        } else {
            c.misses += 1;
        }
        let r = cache.lookup(parent, parents);
        acc = acc
            .wrapping_add(r.a as u64)
            .wrapping_add(r.b as u64)
            .wrapping_add(r.c as u64)
            .wrapping_add(r.d as u64);
    }
    acc
}

#[test]
fn last_key_cache_matches_direct_map_and_the_census_is_exact() {
    // generators inline (self-contained): the deterministic corpus shape —
    // children streamed in same-parent runs, parents spread over a
    // PARENTS-entry table so the miss path is a real hash + probe
    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }
    const PARENTS: u32 = 4096;
    fn make_children(n: usize, seed: u64) -> Vec<(u32, u32)> {
        let mut state: u64 = seed;
        let mut v = Vec::with_capacity(n);
        let mut i = 0usize;
        while i < n {
            let jitter = (lcg(&mut state) % 7) as usize;
            let run = 16 + jitter - 3;
            let parent = (lcg(&mut state) % PARENTS as u64) as u32;
            for _ in 0..run {
                if v.len() >= n {
                    break;
                }
                v.push((i as u32, parent));
                i += 1;
            }
        }
        v
    }
    fn make_parents() -> std::collections::HashMap<u32, Rec> {
        let mut m = std::collections::HashMap::with_capacity(PARENTS as usize);
        for i in 0..PARENTS {
            m.insert(
                i,
                Rec {
                    a: i.wrapping_mul(7).wrapping_add(1),
                    b: i.wrapping_mul(13).wrapping_add(2),
                    c: i.wrapping_mul(31).wrapping_add(3),
                    d: i.wrapping_mul(97).wrapping_add(4),
                },
            );
        }
        m
    }
    // the direct-map reference, spelled independently of the buggy arm
    fn direct(children: &[(u32, u32)], parents: &std::collections::HashMap<u32, Rec>) -> u64 {
        let mut acc = 0u64;
        for &(_, p) in children {
            let r = parents[&p];
            acc = acc
                .wrapping_add(r.a as u64)
                .wrapping_add(r.b as u64)
                .wrapping_add(r.c as u64)
                .wrapping_add(r.d as u64);
        }
        acc
    }
    for n in [0usize, 1, 15, 16, 17, 255, 1_000, 16_384] {
        let children = make_children(n, 0x5EED_B334_CAFE_F00D);
        let parents = make_parents();
        let want = direct(&children, &parents);
        let mut c = Census::default();
        let got = walk(&children, &parents, &mut c);
        assert_eq!(want, got, "checksums must agree at n={n}");
        // the census is exact: one miss per same-parent run start, hits
        // everywhere else — recount the runs independently
        let mut runs = 0usize;
        let mut prev: Option<u32> = None;
        for &(_, p) in &children {
            if prev != Some(p) {
                runs += 1;
            }
            prev = Some(p);
        }
        assert_eq!(c.misses, runs, "miss count at n={n}");
        assert_eq!(c.hits, n - runs, "hit count at n={n}");
    }
    // hit-rate sanity at scale: jittered same-key runs stay >90% hits
    let children = make_children(16_384, 0x5EED_B334_CAFE_F00D);
    let mut c = Census::default();
    let parents = make_parents();
    let _ = walk(&children, &parents, &mut c);
    assert!(
        c.hits * 10 > c.misses * 90,
        "hit rate collapsed: {} hits / {} misses",
        c.hits,
        c.misses
    );
}"##.to_string(),
            context_notes: Some(r##"MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-11-05, PROVENANCE: power=AC Power load=2.50 swap=985.31M canary=skipped powermode=2(high) — quoted per the box-state law; Batch 334 GOAT: 6.79x/6.24x/5.49x at n=1K/16K/262K, allocs 0/0 at every size, alloc-free by construction in both arms — the validator numbers are ours; the standalone rustc direction check agreed at 4.60x at n=262K best-of-15 alternating with the memo arm carrying the census instrumentation, so 4.60x is a conservative floor for the clean shape): the per-child table lookup re-hashes and re-probes for EVERY child of a same-parent run (std SipHash over the key + a probe into ~80 KiB of buckets per miss-class query) while the memo arm pays ONE u32 Eq per child and takes the table only at run starts — 16,389 table lookups vs 262,144 at the 100K-record shape, a 93.75% hit rate at GROUP=16. SOURCE (rust-lang/rust #163628 "Add a single-entry parent SpanData cache", nnethercote, MERGED 2026-10-03 by bors, milestone 1.101.0, 46 additions / 17 deletions across 3 files — re-verified at the PR API + full-diff read this batch; provenance nnethercote.github.io 2026-11-04, the month-2 Hexcat report). THE BLOG (verbatim): "During metadata encoding/decoding and stable hashing we typically need to get information about the same parent span from a hash table over and over. In this PR I put a single-entry cache in front of that hash table, which avoids the hash table lookup in most cases. This reduced instruction counts on many benchmarks, in the best case by more than 4%, mostly benefiting incremental compilation." THE SHIPPED FORM (compiler/rustc_middle/src/ich.rs, verbatim): "A single-entry cache for a `SpanData`, to optimize a particular scenario: when iterating through child spans (e.g. during encoding/decoding/hashing), many of those spans will have the same parent span. In the case where that parent span is interned (e.g. the parent is a const literal item exceeding MAX_LEN bytes of source code), this cache avoids having to fetch the parent's `SpanData` from the interner (which involves TLS, locking, etc.) over and over. A single entry is enough for an extremely high hit rate." — `pub(crate) struct SourceSpanCache(Option<(LocalDefId, SpanData>)`; lookup: `if let Some(cache) = self.0 && cache.0 == def_id { cache.1 } else { let span_data = source_span.get(def_id)...; self.0 = Some((def_id, span_data)); span_data }`. THREE call sites adopted it (stable hashing in ich.rs, on-disk-cache decode, on-disk-cache encode), each a same-parent iteration loop; the PR REMOVED the direct helpers (`def_span`, `source_span_untracked`) at those sites — the memo replaced, not decorated. THE LAW (the one correctness constraint, carried in-rule): a pure memoization is sound ONLY across a READ-STABLE WINDOW — the memo's value must equal what the table would return for the cached key for the whole window. The upstream caches satisfy it STRUCTURALLY: each cache is a field of a short-lived pass state (StableHashState / CacheDecoder / CacheEncoder) that owns the reads and never mutates `source_span` mid-window (an AppendOnlyIndexVec — values never change once written); the app-level rule states it as: memo owned by the reading pass, dropped or invalidated at any observed mutation, never shared across threads or across a mutation point. The memo keys on Eq exactly as the table does (the standard Hash/Eq consistency contract — a key equal to the cached key returns the cached value, which the table would also return). REGIME: hot keyed lookups inside an iteration whose queries arrive in same-key streaks — metadata encode/decode/hash passes over child records keyed by parent/owner; per-item config or style resolution where consecutive items name one provider; per-finding file/rule resolution in a fixer loop; span-like AST walks. The win scales with the STREAK LENGTH and with the miss cost (hash quality, table size, lock/TLS on the read path — upstream's interned-fetch path carried TLS + locking). ANTI-REGIMES: uniformly random keys (hit rate ~0 — the Eq is pure overhead added to every lookup); maps that mutate during the window (an insert that rehashes or overwrites the cached key silently poisons the memo — the window law is the guard, and a mutable map may leave the window every iteration, negating the rule); read paths whose value computation is cheaper than the Eq (trivial Copy of a tiny value from a tiny always-hot table); concurrent readers of one table (a 1-slot memo needs ownership or per-reader sharding — upstream's caches are per-session/per-decoder state, deliberately NOT shared). DISTINCT-FROM (cite, never merge; greps over src/domains/rust_perf/corpus/ + src/domains/kernel_opt/corpus/ re-run at the batch slot, 300-rule corpus): revalidation-skipped-by-certifying-identity-token (entries_49) skips repeated CERTIFICATION work on a repeated input via a builder-minted identity witness — a producer-side skip; THIS is a query-path lookup memo owned by the reader. range-table-page-memoization (entries_29) memoizes CLASSIFICATION through a dense per-page table over a contiguous key domain; THIS memoizes an arbitrary-keyed MAP lookup (hash + probe, no dense domain). mask-memo-hoist-to-owner-lifetime (entries_49) hoists a computation to the OWNER's lifetime (computed once per owner); THIS memoizes per-PASS within one iteration (one slot, overwritten as the streaks advance). batch-local-memoization-negative-cache-tombstone owns a batch-scoped NEGATIVE cache with tombstones (recording absence); THIS is a positive last-key memo. minimal-perfect-hash-static-lookup restructures the TABLE itself (compile-time static keys); the two COMPOSE (a static table still hashes + probes at runtime). The faster-hash family (the fxhashmap proposer) makes each hash cheaper; THIS avoids hashing entirely on hits — the two compose, never conflict. kernel_opt tokenizer-segment-memo-between-mutations (gpu_102) is the cross-domain memo kinship (tokenizer segments between mutations vs lookup handles between streak advances). DEDUP ADJUDICATION (T1 re-run at the batch slot): single.entry|one.entry|last.value|last.lookup|last.key|temporal.local|one.slot|single.slot|last.hit|hot.key|lookup.cache|hash.avoid|skip.hash over both corpora → ZERO owner of a one-slot last-key map-lookup memo (the memo-family rule_ids are negative-cache tombstones, dense page memos, identity-keyed persistent digests, owner-lifetime hoists — all different mechanisms). LANE-INTEL: compile-time domain — the gains reach the workspace PASSIVELY via `rust-toolchain.toml` pin bumps (build-hygiene owns our compile-time measurement discipline); the upstream >4% figure is rustc-benchmark provenance, never a bar. The memo SHAPE is a candidate wherever a hot loop resolves a mostly-constant keyed field — price before any change (the fixture's magnitude must stand on its own validator run at --release). P113 upstream-state check: the mechanism is upstream's OWN landed decision (merged; in rustc 1.101.0) — mineable decision content, never an outbound-PR candidate. Fixture: ORIGINAL plain-Rust pair (never paste source), standalone rustc-compiled AND executed this batch (test binary: equivalence at n = 0/1/15/16/17/255/1K/16K + the exact census — misses == the same-parent run starts recounted independently, hit-rate floor >90%; bin: 93.75% hit rate at n=262_144, direction check 4.60x best-of-15 alternating, memo arm instrumented). Test pins: batch334 needles + the in-rule equivalence/census asserts. Source: rust-lang/rust #163628, read + quoted at the API + full diff this batch. Batch 334 / Plan 343 / Research 372."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(single_entry_last_key_lookup_cache_bench_spec()),
        },
        // ── layout: skewed multi-field payload → inline pack + interned
        // overflow tail ──────────────────────────────────────────────────
        // Source: rust-lang/rust #163857 "Overhaul span compression" +
        // #163971 (span-op micro-opts) + #164291 (the abandoned niche,
        // the negative-result face).
        PerfRule {
            rule_id: "skewed-payload-inline-encode-intern-overflow-tail".to_string(),
            name: "A skewed multi-field payload record lives behind a side-table handle so every field read is a table access and the intern key includes the hot field → pack the distribution-skewed common cases INLINE into one machine word behind a variable-length high-bit prefix (first-fit across formats), intern only the overflow tail — deduped on the COLD fields so the hot field never widens the key — and keep the hot read-alone field inline in EVERY format, including the interned one".to_string(),
            category: "layout".to_string(),
            buggy_code: r##"// NAIVE: every record is interned whole into a side table behind
// a 4-byte handle — every field read is a table access, and the
// dedup key is the FULL record, so the hot read-alone field widens
// the key and multiplies the distinct values that must be stored
// (the default shape of Vec<Rec> + index handles, and of interning
// with the whole struct as the key).

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Rec {
    pub pos: u32,
    pub len: u32,
    pub ctx: u32,
    pub parent: Option<u32>,
}

pub struct Table {
    rows: Vec<Rec>,
    dedup: std::collections::HashMap<Rec, u32>,
    pub table_reads: usize,
}

pub type Handle = u32;

impl Table {
    pub fn new() -> Self {
        Table { rows: Vec::new(), dedup: std::collections::HashMap::new(), table_reads: 0 }
    }

    pub fn encode(&mut self, r: Rec) -> Handle {
        if let Some(&h) = self.dedup.get(&r) {
            return h;
        }
        let h = self.rows.len() as u32;
        self.rows.push(r);
        self.dedup.insert(r, h);
        h
    }

    pub fn decode(&mut self, h: Handle) -> Rec {
        self.table_reads += 1;
        self.rows[h as usize]
    }

    /// The hot field, read alone: STILL a table access.
    pub fn read_ctx(&mut self, h: Handle) -> u32 {
        self.table_reads += 1;
        self.rows[h as usize].ctx
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}"##.to_string(),
            fixed_code: r##"// OPT: one packed 8-byte handle. Four formats behind a
// variable-length prefix in the HIGH bits, chosen FIRST-FIT (always
// the first format whose requirements hold). The overflow tail is
// interned into a side table DEDUPED ON THE COLD FIELDS ONLY (the
// hot field rides inline in the handle, so it must not widen the
// key), and the hot read-alone field stays inline in EVERY format —
// reading it never touches the table.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Rec {
    pub pos: u32,
    pub len: u32,
    pub ctx: u32,
    pub parent: Option<u32>,
}

/// The packed 8-byte handle.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Packed(pub u64);

// Field layouts per format, top to bottom. The prefix values (0b0,
// 0b110, 0b10, 0b111) are mutually exclusive by construction.
mod ic {
    pub const PREFIX_VALUE: u64 = 0b0;
    pub const PREFIX_SHIFT: u32 = 63;
    pub const PREFIX_MASK: u64 = 1; // 1 bit
    pub const LEN_SHIFT: u32 = 48;
    pub const LEN_BITS: u32 = 15;
    pub const CTX_SHIFT: u32 = 32;
    pub const CTX_BITS: u32 = 16;
    pub const POS_SHIFT: u32 = 0; // bottom 32 bits
}
mod ip {
    pub const PREFIX_VALUE: u64 = 0b110;
    pub const PREFIX_SHIFT: u32 = 61;
    pub const PREFIX_MASK: u64 = 0b111; // 3 bits
    pub const LEN_SHIFT: u32 = 47;
    pub const LEN_BITS: u32 = 14;
    pub const PARENT_SHIFT: u32 = 32;
    pub const PARENT_BITS: u32 = 15;
    pub const POS_SHIFT: u32 = 0;
}
mod pair {
    pub const PREFIX_VALUE: u64 = 0b10;
    pub const PREFIX_SHIFT: u32 = 62;
    pub const PREFIX_MASK: u64 = 0b11; // 2 bits
    pub const LEN_SHIFT: u32 = 55;
    pub const LEN_BITS: u32 = 7;
    pub const CTX_SHIFT: u32 = 40;
    pub const CTX_BITS: u32 = 15;
    pub const PARENT_SHIFT: u32 = 24;
    pub const PARENT_BITS: u32 = 16;
    pub const POS_SHIFT: u32 = 0;
    pub const POS_BITS: u32 = 24;
}
mod interned {
    pub const PREFIX_VALUE: u64 = 0b111;
    pub const PREFIX_SHIFT: u32 = 61;
    pub const PREFIX_MASK: u64 = 0b111; // 3 bits
    pub const CTX_SHIFT: u32 = 32;
    pub const CTX_BITS: u32 = 29;
    pub const INDEX_SHIFT: u32 = 0; // bottom 32 bits
}

const fn mask(bits: u32) -> u64 {
    (1u64 << bits) - 1
}

/// The overflow table: deduped on the COLD fields only (`ctx` rides
/// inline in the handle and must NOT widen the key).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RecNoCtx {
    pos: u32,
    len: u32,
    parent: Option<u32>,
}

pub struct Table {
    rows: Vec<RecNoCtx>,
    dedup: std::collections::HashMap<RecNoCtx, u32>,
    pub table_reads: usize,
}

impl Table {
    pub fn new() -> Self {
        Table { rows: Vec::new(), dedup: std::collections::HashMap::new(), table_reads: 0 }
    }

    fn intern(&mut self, r: Rec, ctx: u32) -> Packed {
        let key = RecNoCtx { pos: r.pos, len: r.len, parent: r.parent };
        let idx = match self.dedup.get(&key) {
            Some(&i) => i,
            None => {
                let i = self.rows.len() as u32;
                self.rows.push(key);
                self.dedup.insert(key, i);
                i
            }
        };
        Packed(
            (interned::PREFIX_VALUE << interned::PREFIX_SHIFT)
                | ((ctx as u64 & mask(interned::CTX_BITS)) << interned::CTX_SHIFT)
                | (idx as u64),
        )
    }

    /// First-fit encode: InlineCtxt → InlineParent → InlinePair → intern.
    pub fn encode(&mut self, r: Rec) -> Packed {
        let parent = match r.parent {
            Some(p) if (p as u64) <= mask(ip::PARENT_BITS) => p as u64,
            _ => u64::MAX, // marks "cannot use the parent formats"
        };
        if r.parent.is_none()
            && (r.len as u64) <= mask(ic::LEN_BITS)
            && (r.ctx as u64) <= mask(ic::CTX_BITS)
        {
            return Packed(
                (ic::PREFIX_VALUE << ic::PREFIX_SHIFT)
                    | ((r.len as u64) << ic::LEN_SHIFT)
                    | ((r.ctx as u64) << ic::CTX_SHIFT)
                    | (r.pos as u64),
            );
        }
        if r.parent.is_some()
            && parent != u64::MAX
            && r.ctx == 0
            && (r.len as u64) <= mask(ip::LEN_BITS)
        {
            return Packed(
                (ip::PREFIX_VALUE << ip::PREFIX_SHIFT)
                    | ((r.len as u64) << ip::LEN_SHIFT)
                    | (parent << ip::PARENT_SHIFT)
                    | (r.pos as u64),
            );
        }
        if r.parent.is_some()
            && parent != u64::MAX
            && (r.pos as u64) <= mask(pair::POS_BITS)
            && (r.len as u64) <= mask(pair::LEN_BITS)
            && (r.ctx as u64) <= mask(pair::CTX_BITS)
        {
            return Packed(
                (pair::PREFIX_VALUE << pair::PREFIX_SHIFT)
                    | ((r.len as u64) << pair::LEN_SHIFT)
                    | ((r.ctx as u64) << pair::CTX_SHIFT)
                    | (parent << pair::PARENT_SHIFT)
                    | (r.pos as u64),
            );
        }
        self.intern(r, r.ctx)
    }

    /// Full decode: dispatch on the prefix; only Interned touches the
    /// table. An if-else chain, deliberately not an enum match — the
    /// upstream form's comment says converting to an enum and matching
    /// did not optimize away properly.
    pub fn decode(&mut self, p: Packed) -> Rec {
        let u = p.0;
        if (u >> ic::PREFIX_SHIFT) & ic::PREFIX_MASK == ic::PREFIX_VALUE {
            return Rec {
                pos: (u & mask(32)) as u32,
                len: ((u >> ic::LEN_SHIFT) & mask(ic::LEN_BITS)) as u32,
                ctx: ((u >> ic::CTX_SHIFT) & mask(ic::CTX_BITS)) as u32,
                parent: None,
            };
        }
        if (u >> ip::PREFIX_SHIFT) & ip::PREFIX_MASK == ip::PREFIX_VALUE {
            return Rec {
                pos: (u & mask(32)) as u32,
                len: ((u >> ip::LEN_SHIFT) & mask(ip::LEN_BITS)) as u32,
                ctx: 0, // InlineParent carries the ROOT context by law
                parent: Some(((u >> ip::PARENT_SHIFT) & mask(ip::PARENT_BITS)) as u32),
            };
        }
        if (u >> pair::PREFIX_SHIFT) & pair::PREFIX_MASK == pair::PREFIX_VALUE {
            return Rec {
                pos: (u & mask(pair::POS_BITS)) as u32,
                len: ((u >> pair::LEN_SHIFT) & mask(pair::LEN_BITS)) as u32,
                ctx: ((u >> pair::CTX_SHIFT) & mask(pair::CTX_BITS)) as u32,
                parent: Some(((u >> pair::PARENT_SHIFT) & mask(pair::PARENT_BITS)) as u32),
            };
        }
        self.table_reads += 1;
        let ctx = ((u >> interned::CTX_SHIFT) & mask(interned::CTX_BITS)) as u32;
        let idx = (u & mask(32)) as usize;
        let k = self.rows[idx];
        Rec { pos: k.pos, len: k.len, ctx, parent: k.parent }
    }

    /// The hot field read alone: NEVER a table access — ctx rides inline
    /// in every format (0 for InlineParent, which is root by law).
    pub fn read_ctx(&self, p: Packed) -> u32 {
        let u = p.0;
        if (u >> ip::PREFIX_SHIFT) & ip::PREFIX_MASK == ip::PREFIX_VALUE {
            0
        } else if (u >> interned::PREFIX_SHIFT) & interned::PREFIX_MASK
            == interned::PREFIX_VALUE
        {
            ((u >> interned::CTX_SHIFT) & mask(interned::CTX_BITS)) as u32
        } else if (u >> pair::PREFIX_SHIFT) & pair::PREFIX_MASK == pair::PREFIX_VALUE {
            ((u >> pair::CTX_SHIFT) & mask(pair::CTX_BITS)) as u32
        } else {
            ((u >> ic::CTX_SHIFT) & mask(ic::CTX_BITS)) as u32
        }
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

/// The format census (which prefix a handle carries).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fmt {
    InlineCtxt,
    InlineParent,
    InlinePair,
    Interned,
}

pub fn fmt_of(p: Packed) -> Fmt {
    let u = p.0;
    if (u >> ic::PREFIX_SHIFT) & ic::PREFIX_MASK == ic::PREFIX_VALUE {
        Fmt::InlineCtxt
    } else if (u >> ip::PREFIX_SHIFT) & ip::PREFIX_MASK == ip::PREFIX_VALUE {
        Fmt::InlineParent
    } else if (u >> pair::PREFIX_SHIFT) & pair::PREFIX_MASK == pair::PREFIX_VALUE {
        Fmt::InlinePair
    } else {
        Fmt::Interned
    }
}

#[test]
fn packed_roundtrip_is_exact_and_the_censuses_hold() {
    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }
    // deterministic skewed corpus: ~70% InlineCtxt-shaped, ~15%
    // InlineParent (parent, zero ctx), ~14% InlinePair (both, small),
    // ~1% overflow (one field too big) — the shape upstream measured
    fn make_records(n: usize, seed: u64) -> Vec<Rec> {
        let mut state: u64 = seed;
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            let roll = lcg(&mut state) % 100;
            let r = if roll < 70 {
                Rec { pos: (lcg(&mut state) % 1_000_000) as u32, len: (lcg(&mut state) % 4096) as u32, ctx: (lcg(&mut state) % 4096) as u32, parent: None }
            } else if roll < 85 {
                Rec { pos: (lcg(&mut state) % 1_000_000) as u32, len: (lcg(&mut state) % 8192) as u32, ctx: 0, parent: Some((lcg(&mut state) % 16_384) as u32) }
            } else if roll < 99 {
                Rec { pos: (lcg(&mut state) % 1_000_000) as u32, len: (lcg(&mut state) % 96) as u32, ctx: 1 + (lcg(&mut state) % 16_383) as u32, parent: Some((lcg(&mut state) % 32_768) as u32) }
            } else {
                // the overflow tail: one field too big; ctx stays within
                // 29 bits (the hard limit upstream asserts)
                if i % 2 == 0 {
                    Rec { pos: 20_000_000 + i as u32, len: 64, ctx: 5, parent: Some(7) }
                } else {
                    Rec { pos: 9, len: 500_000, ctx: 5, parent: Some(7) }
                }
            };
            v.push(r);
        }
        v
    }
    for n in [0usize, 1, 100, 1_000, 16_384] {
        let records = make_records(n, 0x5EED_B335_CAFE_F00D);
        let mut ft = Table::new();
        let handles: Vec<Packed> = records.iter().map(|&r| ft.encode(r)).collect();
        // round-trip exactness, EVERY record
        for (i, &r) in records.iter().enumerate() {
            assert_eq!(ft.decode(handles[i]), r, "round-trip at {i} n={n}");
        }
        // the decode census: the table is touched ONLY for Interned
        let interned_count =
            (0..n).filter(|&i| fmt_of(handles[i]) == Fmt::Interned).count();
        assert_eq!(
            ft.table_reads, interned_count,
            "decode touches the table only for interned at n={n}"
        );
        // the hot-field-alone read never touches the table
        for i in 0..n {
            assert_eq!(ft.read_ctx(handles[i]), records[i].ctx, "hot read at {i} n={n}");
        }
        assert_eq!(
            ft.table_reads, interned_count,
            "read_ctx never touches the table"
        );
        assert_eq!(std::mem::size_of::<Packed>(), 8, "handle stays 8 bytes");
        if n >= 1_000 {
            assert!(
                ft.len() <= interned_count,
                "table holds only interned records: {} vs {}",
                ft.len(),
                interned_count
            );
            let inline = n - interned_count;
            assert!(
                inline * 20 >= n * 19,
                "inline share must be >=95%: {} / {}",
                inline,
                n
            );
        }
    }
}"##.to_string(),
            context_notes: Some(r##"Grade B− — decision-content, suggestion-only, NotBenchable (the size-axis class: the win is representation + table-access SHAPE — inline share, interned share, handle size, hot-field reads that never lock — not a clock claim; no Pareto spec, the census fixture is deterministic and exact; the box-large-enum-variant / narrow-counter-overflow-promotion precedents are the class's bench_spec-None lineage). SOURCE (rust-lang/rust #163857 "Overhaul span compression", nnethercote, MERGED 2026-10-13 by bors, milestone 1.101.0, 325 additions / 243 deletions across 3 files — re-verified at the PR API + full-diff read this batch; prose surface nnethercote.github.io 2026-11-04: "For incremental builds we now typically intern around 1% of spans compared to 20-40% with the old scheme. (In non-incremental builds we typically intern fewer than 0.1%.) ... This change reduced instruction counts across many benchmarks, in the best cases by more than 3%, and the cycle count improvements were even higher."). THE SHIPPED FORM (compiler/rustc_span/src/span_encoding.rs, verbatim): `#[repr(packed(4))] pub struct Span(u64)` — four formats behind "A variable-length prefix in the high bits", first-fit ("It is an invariant that we always use the first listed format whose requirements are satisfied"): InlineCtxt `[ prefix=0 | len:15 | ctxt:16 | lo:32 ]`, InlineParent `[ prefix=110 | len:14 | parent:15 | lo:32 ]` (ROOT context only), InlinePair `[ prefix=10 | len:7 | ctxt:15 | parent:16 | lo:24 ]` (new — "handles most cases where both a context and a parent are present ... This is the big performance win"), Interned `[ prefix=111 | ctxt:29 | index:32 ]` (the old Partially/fully-interned split MERGED). THE THREE DESIGN LAWS (all verbatim from the landed module doc): (1) THE HOT FIELD STAYS INLINE IN EVERY FORMAT — "The ctxt is stored inline in every format ... This means reading the ctxt never requires checking the interner. This is good because the ctxt is often consulted by itself." — and that RESTRUCTURES THE INTERN KEY: "we don't need to store the ctxt in the interner table, which we achieve by using `SpanDataNoCtxt`, which minimizes the number of unique values that need to be interned" — dropping the hot field from the dedup key SHRINKS the table (fewer distinct values), the opposite of the naive whole-record key. (2) THE FIELD WIDTHS ARE MEASURED, NOT GUESSED — "This configuration was the best one found after many measurements of real-world crates, and replaced an earlier 8-byte design"; the per-field distribution notes ("The most common numbers of bits needed for `len` are from 0 to 7, with a peak usually at 3 or 4"; "values needing more than 15 bits are very rare") are the skew evidence. (3) ALIGNMENT IS PART OF THE CLAIM — "The use of `repr(packed(4))` means `Span` has an alignment of 4 bytes, which keeps many types that contain spans (e.g. AST nodes) smaller. `repr(packed(1))` isn't worth it; it only shrinks a few types and performance is a tiny bit worse." THE DISPATCH FACE (the #163971 refinement, jttriplett, MERGED, best case >2% instruction reduction): the format check is a cheap bitmask on the packed word, so hot ops take an INLINE-ONLY FAST PATH — `Span::to` (verbatim): "Inline-only fast path. Inline contexts are by far the most common case, checking for them is fast (just a bitmask check on the packed span representation), we know we don't have parents, and we know the context will fit back in an inline span. So, this fast path entirely skips setting up a stack frame or saving registers." — with the general path `#[inline(never)]` and the uncommon metavar resolution `#[cold]`; plus the constructor split (`new` handles lo>hi by swapping, `new_ordered` debug_asserts — hot constructors don't pay the branch) and the argument narrowing (try_metavars passes the two contexts, not two whole SpanDatas). REGIME: a skewed multi-field payload record at HIGH MULTIPLICITY where the fields' value distributions leave most records fittable in narrow formats (AST/IR metadata, position+extent+context records, event envelopes with optional extras, entity attribute records), a HOT FIELD read alone (so it must stay inline even in the interned format, and must NOT widen the intern key), and an overflow tail small enough to intern. The packed word is the handle AND the data: cache footprint per record halves, and the hot-field read path takes NO lock, NO TLS, NO table access. ANTI-REGIMES (honesty carried from #164291, the abandoned niche attempt, verbatim: "It worked but didn't help performance because we don't use `Option<Span>` much, and it made the code more complicated. So I abandoned it." — a representation trick with zero measurable win on the wrapped type's real frequency is NET NEGATIVE: measure the distribution BEFORE packing, and the complexity budget after): uniform full-width values (nothing fits the narrow formats — the prefix dispatch is pure overhead); records at low multiplicity (a handful of entries never pays back the encode/decode machinery); fields with no read-alone pattern (inline-everywhere buys nothing if every read is whole-record); mutable records (packing is an immutable-handle design — mutation re-encodes). THE INTEGRITY COSTS ARE THE PRICE OF ADMISSION: bit widths become HARD LIMITS with loud refusals at encode (upstream asserts the 29-bit ctxt as "a hard limit, with no fallback" into interning), and every format needs its decode arm tested (the first-fit invariant + prefix exclusivity is the fixture's census). DISTINCT-FROM (cite, never merge; greps re-run at the batch slot): adaptive-sparse-blocked-table-encoding (entries_60, Batch 326) — the NAMED COUNTER-PRECEDENT from the same source family (the rustc representation lane converts): that rule picks dense-vs-sparse per TABLE at flush time with a presence bitmap; THIS packs per-RECORD skew into one word with first-fit formats — different axis (table storage vs record encoding), and its landing is the evidence this lane mines honestly. order-preserving-inline-key-encoding (entries_52) packs a KEY for sort-order fidelity; THIS packs a PAYLOAD for read-path shape (no ordering claim). narrow-counter-overflow-promotion + box-large-enum-variant own the single-field width/layout axis; THIS is the multi-field skew + intern-tail composition. kernel_opt cross-check at the batch slot found only tokenizer-segment-memo-between-mutations (gpu_102) — a MEMO kinship, not layout. DEDUP ADJUDICATION (T1 re-run): intern.overflow|intern.tail|pack.common|inline.pack|overflow.tail over rust_perf → ZERO owner of the pack-common-intern-tail law. Fixture: ORIGINAL plain-Rust pair (never paste source), standalone rustc-compiled AND executed this batch (test binary: round-trip 2/2 arms at n = 0/1/100/1K/16K + the exact decode/read censuses + the 8-byte handle pin; bin at n=100K: formats IC/IP/IPair/Interned = 70004/15247/13848/901 — interned 0.90%, mirroring upstream's ~1% — fixed table 465 overflow-only entries vs buggy 99,521 full-record entries, buggy decode table accesses 100,000 vs fixed 901). Test pins: batch334 needles + the in-rule round-trip/census asserts. Source: rust-lang/rust #163857 + #163971 + #164291, read + quoted at the API + full diffs this batch. Batch 334 / Plan 343 / Research 372."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// Bench spec for `single-entry-last-key-lookup-cache-hash-avoidance`: the
/// same-parent keyed-lookup walk over a deterministic streak corpus — the
/// per-child hash + probe vs the one-slot last-key memo. Neither arm
/// allocates in the loop: the win is pure timing (the hash+probe asymmetry),
/// alloc-free by construction in both arms.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)]
fn single_entry_last_key_lookup_cache_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r##"
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rec {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

pub struct Input {
    children: Vec<(u32, u32)>,
    parents: std::collections::HashMap<u32, Rec>,
}

pub fn make_input(n: usize) -> Input {
    // children streamed in same-parent runs (jittered, deterministic);
    // parents spread over a 4096-entry table so the miss path is a real
    // hash + probe over ~80 KiB of buckets
    let mut state: u64 = 0x5EED_B334_CAFE_F00D;
    let mut children = Vec::with_capacity(n);
    let mut i = 0usize;
    while i < n {
        let jitter = (lcg(&mut state) % 7) as usize;
        let run = 16 + jitter - 3;
        let parent = (lcg(&mut state) % 4096) as u32;
        for _ in 0..run {
            if children.len() >= n {
                break;
            }
            children.push((i as u32, parent));
            i += 1;
        }
    }
    let mut parents = std::collections::HashMap::with_capacity(4096);
    for i in 0..4096u32 {
        parents.insert(
            i,
            Rec {
                a: i.wrapping_mul(7).wrapping_add(1),
                b: i.wrapping_mul(13).wrapping_add(2),
                c: i.wrapping_mul(31).wrapping_add(3),
                d: i.wrapping_mul(97).wrapping_add(4),
            },
        );
    }
    Input { children, parents }
}

mod buggy {
    use super::Rec;
    use std::collections::HashMap;

    pub fn walk(children: &[(u32, u32)], parents: &HashMap<u32, Rec>) -> u64 {
        let mut acc = 0u64;
        for &(_child, parent) in children {
            let r: Rec = *parents.get(&parent).expect("parent missing");
            acc = acc
                .wrapping_add(r.a as u64)
                .wrapping_add(r.b as u64)
                .wrapping_add(r.c as u64)
                .wrapping_add(r.d as u64);
        }
        acc
    }
}

mod fixed {
    use super::Rec;
    use std::collections::HashMap;

    pub struct LastKeyCache(pub Option<(u32, Rec)>);

    impl LastKeyCache {
        #[inline]
        pub fn lookup(&mut self, key: u32, parents: &HashMap<u32, Rec>) -> Rec {
            if let Some((last_key, rec)) = self.0 {
                if last_key == key {
                    return rec;
                }
            }
            let rec = *parents.get(&key).expect("parent missing");
            self.0 = Some((key, rec));
            rec
        }
    }

    pub fn walk(children: &[(u32, u32)], parents: &HashMap<u32, Rec>) -> u64 {
        let mut cache = LastKeyCache(None);
        let mut acc = 0u64;
        for &(_child, parent) in children {
            let r = cache.lookup(parent, parents);
            acc = acc
                .wrapping_add(r.a as u64)
                .wrapping_add(r.b as u64)
                .wrapping_add(r.c as u64)
                .wrapping_add(r.d as u64);
        }
        acc
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::walk(&input.children, &input.parents));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::walk(&input.children, &input.parents));
}
"##
        .to_string(),
    }
}
