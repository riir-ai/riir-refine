//! Entries slice 04 of 06 — rules 76–87 of 126 — moved VERBATIM from the
//! former corpus.rs monolith (2026-09-10 split, scripted by
//! `scripts/split_rust_perf_corpus.py`). Append new batches to the
//! LAST slice or open a new one past ~1800 lines.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── iterators: sortedness early-exit → branchless accumulate ────
        // Source: polars SortedSlice::is_sorted_ascending (Batch 66,
        // Research 073 — polars-utils/src/slice.rs: "don't early stop or
        // branch so it autovectorizes"). The buggy early-exit is the
        // natural idiom but the per-element branch defeats vectorization;
        // the accumulate form scans all N as compare+AND and
        // auto-vectorizes. The mirrored regime of `filter-count-to-any`
        // (same vectorization mechanism, opposite direction: REMOVING an
        // early exit buys vectorization when the predicate rarely fires).
        PerfRule {
            rule_id: "sortedness-check-branchless-accumulate".to_string(),
            name: "sortedness early-exit validation → branchless accumulate full scan".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn validate_sorted(vals: &[u64]) -> bool {\n    let mut prev = u64::MIN;\n    for &v in vals {\n        if v < prev {\n            return false; // early exit blocks vectorization\n        }\n        prev = v;\n    }\n    true\n}".to_string(),
            fixed_code: "fn validate_sorted(vals: &[u64]) -> bool {\n    let mut sorted = true;\n    let mut prev = u64::MIN;\n    for &v in vals {\n        sorted &= prev <= v; // no branch: compare + AND, auto-vectorizes\n        prev = v;\n    }\n    sorted\n}".to_string(),
            context_notes: Some(
                "Regime-split (Batch 66 bench, M3 Max aarch64, load-recorded ~4-6): ParetoDominant 2.98x@100K / 2.72x@1M on USUALLY-SORTED data — the early-exit branch never fires but still forces scalar compare+branch per element, while the accumulate form vectorizes (batched compare+AND). On data with an EARLY violation the branchless form is ParetoDominated (it scans all N; the early exit stops at the first violation) — the mirrored regime of `filter-count-to-any`. Suggest only for usually-sorted validation (post-sort invariant checks, monotonic timestamps); prefer the early-exit form when violations are expected."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sortedness_check_branchless_bench_spec()),
        },
        // ── iterators: byte-find position scan → memchr primitive ──────
        // Source: regex-automata (Batch 67, Research 074 — rust-lang/regex,
        // regex-automata/src/util/memchr.rs + dfa/accel.rs; the engine's own
        // naive fallback IS this rule's buggy shape). `xs.iter().position(|&b| b == lit)`
        // on a `&[u8]` cannot dispatch to a byte-search primitive — the predicate
        // is opaque to the iterator machinery + early-exit blocks vectorization —
        // so std runs a scalar compare-per-byte scan. The memchr crate's
        // memchr/memrchr (and 2/3-needle variants for `||` chains, the engine's
        // exact arity ladder) scan 16-32B blocks with SIMD + movemask. Same
        // `Option<usize>` — bit-identical. EXCLUDES `xs.contains(&b)`: std
        // ALREADY specializes contains for u8 via `SliceContains` -> core's SWAR
        // memchr (core/src/slice/cmp.rs) — contains is already fast; the
        // position/rposition shape is where the win lives (the asymmetry IS the
        // rule). Dep-gated (the fxhashmap precedent): fire only when the target
        // manifest declares memchr, else suggestion-only. Regime: wash below
        // ~16-32B haystacks (call overhead eats the win; the engine's own
        // fallback concedes naive is fine small).
        PerfRule {
            rule_id: "byte-find-position-to-memchr".to_string(),
            name: "u8 slice iter().position(|&b| b == lit) → memchr::memchr".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn find_delim(buf: &[u8]) -> Option<usize> {\n    buf.iter().position(|&b| b == b';')\n}".to_string(),
            fixed_code: "fn find_delim(buf: &[u8]) -> Option<usize> {\n    memchr::memchr(b';', buf) // SIMD block scan + movemask early exit\n}".to_string(),
            // BATCH 172 RIDER (mezura, Research 202 candidate 2 — the
            // anchor-byte greedy set-cover for multi-NEEDLE scans): when
            // the scan targets a set of multi-byte SYMBOLS (comment
            // openers, raw-string fences, string terminators), do not run
            // one search per symbol — derive the FEWEST finding bytes that
            // reach every symbol (greedy min-cover over the symbols'
            // bytes), disfavouring LETTERS (visited on every word),
            // anchoring multi-byte symbols at an offset BEHIND the found
            // byte (a `r#"` is found by its quote; verify the full symbol
            // at the hit), and pack the chosen 2-3 anchor bytes into one
            // memchr2/memchr3 pass. The selection algorithm is the rider's
            // content — the memchr-packing primitive is this rule's own
            // ladder, and a cover computed once per needle set amortizes
            // like the finder's bool table.
            context_notes: Some("DEP-GATED suggestion (fire only when the target manifest declares memchr, else suggestion-only); the single-needle rule's own ParetoDominant bench verdict is unchanged by this rider. Wash below ~16-32B haystacks (call overhead eats the win). EXCLUDES xs.contains(&b): std already specializes u8 contains via SWAR -- the position/rposition shape is where the win lives. BATCH 172 RIDER -- the multi-NEEDLE selection half (mezura ScanPlan/anchors_of, Research 202 cand 2, adjudicated rider-not-standalone: the arity ladder above already names the 2/3-needle packing; the DISTINCT content is WHICH bytes feed it): for a set of multi-byte SYMBOLS, derive the minimal ANCHOR COVER -- greedy fewest finding bytes reaching every symbol, letters disfavoured (hit on every word), multi-byte symbols anchored at an offset BEHIND the found byte (find the quote of r#\", verify the fence at the hit) -- then ONE memchr2/memchr3 pass over the anchors, verifying full symbols only at hits. Compute the cover ONCE per needle set (it amortizes the way a finder's tables do); re-deriving per line re-pays the selection and is the per-line shape this rule family exists to kill. Source: subamanis/mezura file_parser.rs (ScanPlan, anchors_of) @ 2dbacb7, MIT OR Apache-2.0. Batch 172 / Plan 166. BATCH 245 RIDER -- the CHAR face (char-search-encode-once-stack, the staged lead converted count-neutral: the decision is the same class this rule owns -- route the search to the byte primitive -- with a thin encode-once prelude; an own rule would stand on one 3-line body with no source rationale, the rider fits): a char needle is encoded ONCE into a 4-byte stack buffer and handed to the byte machinery -- never decode haystack characters per position, never allocate a String per call. SOURCE (BurntSushi/bstr @ tag 1.13.1, commit 134195be38c7c9a4887980fd0f6f8bae042dd91e, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone): the forward encode-once (src/ext_slice.rs:897-900, verbatim): '#[inline]' + 'fn find_char(&self, ch: char) -> Option<usize> {' + 'self.find(ch.encode_utf8(&mut [0; 4]))' + '}' and the reverse twin (ext_slice.rs:921-924, verbatim): 'fn rfind_char(&self, ch: char) -> Option<usize> {' + 'self.rfind(ch.encode_utf8(&mut [0; 4]))' -- the route to the byte search (ext_slice.rs:711-713, verbatim): 'fn find<B: AsRef<[u8]>>(&self, needle: B) -> Option<usize> {' + 'Finder::new(needle.as_ref()).find(self.as_bytes())' -- and the machinery's own arity dispatch means the caller never special-cases ASCII (memchr/src/memmem/searcher.rs:59-63, verbatim): 'trace!(\"building one-byte substring searcher\");' + 'debug_assert_eq!(1, needle.len());' + 'Searcher {' + 'call: searcher_kind_one_byte,' -- 1-byte (ASCII) chars ride the memchr arm inside the crate. THE ONE SEMANTIC CAVEAT IN-SOURCE (ext_slice.rs:882-884, verbatim): 'Note that if one searches for the replacement codepoint, \u{FFFD},' + 'then only explicit occurrences of that encoding will be found. Invalid' + 'UTF-8 sequences will not be matched.' Regime (labeled inference -- the source has no prose here): any char-keyed scan over byte/str content (delimiters, separators, glyph finds) that today walks chars() or decodes per boundary; anti-regimes: needle semantics needing char-boundary validation beyond first-occurrence position, allocation-bearing encodes (String::from(ch) per call inverts the law -- the buffer is the STACK's [0; 4]). Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byte_find_position_to_memchr_bench_spec()),
        },
        // ── io: per-byte Write::write_all varint loop → ladder compose + one call ──
        // Source: tantivy (Batch 68, Research 075 — common/src/vint.rs
        // `write_u32_vint`/`serialize_vint_u32`; coordinator quote-verified
        // 2026-08-19). **MEASURED HONEST LOSER on this box (M3 Max aarch64,
        // load ~25-33, 3 regimes: Vec-push / inline-Cursor writer /
        // Box<dyn Write> vtable writer — ParetoDominated 0/3 sizes in EACH,
        // alloc axis equal + stable, outputs byte-identical).** The folk
        // mechanism story dies twice: (1) 128 is a compile-time CONSTANT —
        // LLVM strength-reduces `x % 128` / `x / 128` to `& 127` / `>> 7`
        // (the Batch 59 two-regime note applied in reverse — there is no
        // hardware-div chain to eliminate), and (2) even N vtable calls
        // into a dyn writer are hidden by out-of-order execution behind the
        // ladder's 5-deep branch chain. The entry stays as an ANNOTATED
        // honest finding (the `precompute-table` class): it exists to STOP
        // the plausible-looking "compose into a word, store once" port
        // from tantivy's codebase — measured, it REGRESSES on modern
        // aarch64 AND on x86 (re-gated 2026-08-19 on the 4090 box: dominated
        // 0/3 there too, 0.40-0.50x — HARDER than aarch64). Regime facts
        // that remain true: the ladder
        // is convention-faithful + bit-identical (exhaustive gate below),
        // and the u64 width boundary (10 bytes > word) keeps the loop.
        PerfRule {
            rule_id: "varint-compose-word-single-store".to_string(),
            name: "Per-byte write_all varint loop (N writer calls) → threshold-ladder compose + single write_all".to_string(),
            category: "io".to_string(),
            buggy_code: "use std::io::{Write, Result};\nfn write_vint<W: Write>(w: &mut W, mut val: u32) -> Result<()> {\n    loop {\n        let byte = (val % 128) as u8;\n        val /= 128;\n        if val == 0 {\n            w.write_all(&[byte | 128])?; // a writer call PER BYTE\n            return Ok(());\n        }\n        w.write_all(&[byte])?;\n    }\n}".to_string(),
            fixed_code: "use std::io::{Write, Result};\nfn write_vint<W: Write>(w: &mut W, val: u32) -> Result<()> {\n    let v = u64::from(val);\n    // compose ALL bytes into one u64 (masked shifts), ONE writer call\n    let (res, len) = if v < (1 << 7) {\n        (v | 128, 1)\n    } else if v < (1 << 14) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)\n    } else if v < (1 << 21) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)\n    } else if v < (1 << 28) {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)\n    } else {\n        ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)\n    };\n    w.write_all(&res.to_le_bytes()[..len]) // ONE store, ONE call\n}".to_string(),
            context_notes: Some(
                "ParetoDominated (Batch 68 bench, M3 Max aarch64, load-recorded ~25-33, THREE regimes tested): the ladder LOSES 0/3 sizes on the Vec-push shape AND the inline-Cursor writer shape AND the Box<dyn Write> vtable shape — alloc axis equal + stable, outputs byte-identical. The folk mechanism story dies twice: (1) 128 is a compile-time constant, so LLVM strength-reduces x % 128 / x / 128 to & 127 / >> 7 — there is NO hardware-div chain to eliminate (the division-by-invariant-multiplication two-regime note applied in reverse); (2) even N dyn-dispatch writer calls are hidden by out-of-order execution behind the ladder's 5-deep branch chain. The rule exists to STOP the plausible-looking 'compose into a word, store once' port from tantivy's codebase — measured, it REGRESSES on modern aarch64 AND x86 (4090-box re-gate 2026-08-19, quiet, load 10: dominated 0/3 there TOO, HARDER — 0.40x/0.43x/0.50x at 1K/100K/1M, the ladder 2.0-2.5x slower than the per-byte loop; alloc axis equal, G-det stable). Bit-identity remains proven (exhaustive gate: full u16 + 128^i boundaries + LCG u32, both conventions)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(varint_compose_word_single_store_bench_spec()),
        },
        // ── io: shift/or byte-assembly word read → one unaligned from_le_bytes load ──
        // Source: tantivy bitpacker (Batch 69, Research 075 §Secondary — the
        // Plan 013 T6 defer MEASURED-REFUTED: the varint honest-loser was
        // CALL-COUNT (OoO hides N dyn-dispatch writer calls); this is
        // LOAD-COUNT (8 separately-loaded bytes vs 1 fused unaligned load)
        // — a different mechanism class, and OoO cannot hide 8 L1 loads
        // behind 1. LLVM does NOT fuse the shift/or assembly chain into a
        // byte copy (the dependency chain + per-byte bounds checks block
        // it). Cross-arch probe 2026-08-19 (M3 aarch64 load ~118 interleaved
        // + 4090-box x86 load 7-31, median-of-5): u64 2.17-2.23x / 1.39-1.47x,
        // u32 1.27-1.38x / 1.20-1.29x — ParetoDominant BOTH arches BOTH
        // widths, G1 bit-identical at EVERY byte alignment. u16 excluded
        // (2 bytes, unmeasured, diminishing). The bit-packed variant
        // (tantivy `BitUnpacker::get`: + shift+mask after the load) is the
        // same rule applied further along.
        PerfRule {
            rule_id: "unaligned-word-load-from-bytes".to_string(),
            name: "Shift/or byte-assembly word read (8 loads + 7 shifts + 7 ors) → one unaligned slice from_le_bytes load".to_string(),
            category: "io".to_string(),
            buggy_code: "fn read_u64_le(data: &[u8], addr: usize) -> u64 {\n    // 8 indexed loads + 7 shifts + 7 ors — a serial assembly chain\n    (data[addr] as u64)\n        | ((data[addr + 1] as u64) << 8)\n        | ((data[addr + 2] as u64) << 16)\n        | ((data[addr + 3] as u64) << 24)\n        | ((data[addr + 4] as u64) << 32)\n        | ((data[addr + 5] as u64) << 40)\n        | ((data[addr + 6] as u64) << 48)\n        | ((data[addr + 7] as u64) << 56)\n}".to_string(),
            fixed_code: "fn read_u64_le(data: &[u8], addr: usize) -> u64 {\n    // ONE unaligned 8-byte load — LLVM lowers from_le_bytes over a\n    // slice copy to a single unaligned u64 load\n    let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();\n    u64::from_le_bytes(bytes)\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(unaligned_word_load_from_bytes_bench_spec()),
        },
        // ── caching: comparator lock+clone per invocation → maintained sort-key
        // cache refreshed once per data update ──────────────────────────────
        // Source: mnt.io "Memory pressure, lock contention, and Data-oriented
        // Design" (Batch 73, Research 085/086 — the Element X frozen-room-list
        // hunt: 322,042 allocations / 743 MiB inside `eyeball_im_util`
        // `SortBy` for ~500-600 rooms; 53.027 ms → 676.29 µs = 78× / −98.7%,
        // +7718.5% throughput on the `room_list` bench). Each sorter
        // comparison called `Room::latest_event()` —
        // `self.info.read().latest_event.clone()`, a read-lock acquire + a
        // 144-byte clone per call, twice per comparison, across 2 of 3
        // sorters; the eyeball-im-util `SortBy` machinery re-invokes the
        // comparator from `binary_search_by` on EVERY reactive diff and does
        // a fresh full sort on every filter change (`stream!` loop in
        // `entries_with_dynamic_adapters_impl`) — the sorts-per-update > 1
        // regime is structural. Fix: the production `RoomListItem` facade
        // caches the 6 sort/filter-relevant fields (verified in-source:
        // `matrix_sdk_ui::room_list::RoomListItem` + `refresh_cached_data()`
        // on every `RoomInfoNotableUpdate`; the sorter doc comments carry
        // the warning verbatim: "Using `Room::new_latest_event` would be
        // dramatic as it returns a clone of the `LatestEventValue").
        // REGIME: sorts-per-update ≥ 1 (reactive re-sort on diff/filter) —
        // the refresh cost (1 narrow read per changed item) amortizes over
        // N·log N comparator calls per sort. At sorts-per-update ≈ 0 (updates
        // never sorted) the maintained cache is pure refresh overhead.
        // Composes with `lock-guard-narrow-field-accessor` (the refresh reads
        // through narrow accessors, not whole-payload clones). Distinct from
        // `sort-by-key` (that still invokes the key fn N times per sort —
        // with the lock+clone inside it) and from `regex-hoist-lazylock`
        // (once-per-program vs once-per-data-update).
        PerfRule {
            rule_id: "sort-key-cache-precompute-out-of-lock".to_string(),
            name: "comparator lock+clone per invocation → maintained sort-key cache refreshed once per update RIDER (Batch 112 external confirmation, nnethercote 2026-07: rustc #157179 — rustdoc sorted impls by LONG GENERATED HTML STRINGS; a short text repr as the sort key won >6% icount — never sort by expensive payloads, derive a cheap key)".to_string(),
            category: "caching".to_string(),
            buggy_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event_timestamp(&self) -> Option<u64> {\n        // read-lock acquire + clone of the WHOLE LatestEventValue (144 B)\n        // to read one u64 — paid O(N log N) times per sort, twice per\n        // comparison, by every sorter AND every binary_search_by in the\n        // reactive diff machinery\n        self.info.read().unwrap().latest_event.clone().map(|ev| ev.timestamp)\n    }\n}\nfn sort_rooms(rooms: &mut [Room]) {\n    rooms.sort_by(|a, b| b.latest_event_timestamp().cmp(&a.latest_event_timestamp()));\n}".to_string(),
            fixed_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event_timestamp(&self) -> Option<u64> {\n        // narrow read through the guard — no clone (composes with\n        // lock-guard-narrow-field-accessor)\n        self.info.read().unwrap().latest_event.as_ref().map(|ev| ev.timestamp)\n    }\n}\n/// The production `RoomListItem` pattern: cache the sort/filter-relevant\n/// fields, refresh once per data update (only the CHANGED item).\nstruct RoomListItem {\n    inner: Room,\n    cached_latest_event_timestamp: Option<u64>,\n}\nimpl RoomListItem {\n    fn refresh_cached_data(&mut self) { // called on each RoomInfoNotableUpdate\n        self.cached_latest_event_timestamp = self.inner.latest_event_timestamp();\n    }\n}\nfn sort_rooms(rooms: &mut [RoomListItem]) {\n    // comparator reads the maintained cache — no lock, no clone\n    rooms.sort_by(|a, b| b.cached_latest_event_timestamp.cmp(&a.cached_latest_event_timestamp));\n}".to_string(),
            context_notes: Some("Measured ParetoDominant 24.3-37.2x across the sorts-per-update axis (1/4/16 sorts per refresh cycle, N=512; M3 Max release, load ~7.1, 2026-08-26; allocs 164497->17 + bytes 8.1MB->532KB @16 sorts, alloc-det stable; second invocation 28.1-35.1x — verdict stable). REGIME: the win GROWS with sorts-per-update (reactive re-sort on diff/filter — eyeball-im-util's SortBy re-invokes the comparator from binary_search_by on every diff, and every filter change does a fresh full sort); at sorts-per-update ~= 0 (data updated but never re-sorted) the maintained cache is pure refresh overhead — N narrow reads per Reset with no consumer. MAGNITUDE HONESTY: the source's 78x/-98.7% (53.027ms -> 676.29us, +7718.5% throughput, the matrix-rust-sdk room_list bench, ~500-600 rooms) is END-TO-END including allocation-pressure relief on a mobile allocator; our isolated-mechanism bench reads 24-37x — never cite 78x as our bench claim. Bench protocol note: the harness permutes via a fixed Fisher-Yates swap list before every sort — a plain reverse() is ONE detectable run for the adaptive driftsort (measured ~512 comparisons/sort vs the shuffle's ~5100 at N=512); the comparison count is the load-bearing axis. Composes with lock-guard-narrow-field-accessor (the refresh reads through narrow accessors).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_key_cache_bench_spec()),
        },
        // ── concurrency: whole-payload clone under read-guard → narrow field
        // accessor returning only the consumed stack-sized value ───────────
        // Source: same article (Batch 73, Research 085/086 — the +18%
        // throughput patch on the same `room_list` bench from splitting the
        // wide accessors alone). `fn field(&self) -> BigValue { self.info
        // .read().field.clone() }` where consumers read ONE sub-field (a
        // bool, a timestamp) pays a full 144 B clone per call for a 1-8 B
        // result. The production fix added narrow siblings —
        // `Room::new_latest_event_is_local() -> bool`, `new_latest_event_
        // timestamp() -> Option<Ms>` — verified in-source: the sorter doc
        // comments carry the rule verbatim ("It's better to use the more
        // specific method … where the value is cached"). Keeps the lock,
        // shrinks the guarded payload crossing — composable with (and
        // distinct from) `papaya-lockfree`/`mutex-to-atomic` (replace the
        // lock) and `avoid-clone` (borrow at fn boundary, no guard).
        PerfRule {
            rule_id: "lock-guard-narrow-field-accessor".to_string(),
            name: "fn field(&self) -> BigValue { guard.clone() } → narrow accessor returning only the consumed sub-field".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, sender_is_local: bool, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event(&self) -> Option<LatestEvent> {\n        self.info.read().unwrap().latest_event.clone() // 144 B clone under the guard\n    }\n    fn latest_event_is_local(&self) -> bool {\n        // clones the WHOLE guarded payload to compute one bool\n        self.latest_event().is_some_and(|ev| ev.sender_is_local)\n    }\n}".to_string(),
            fixed_code: "use std::sync::RwLock;\nstruct LatestEvent { timestamp: u64, sender_is_local: bool, body: String /* …144 B total */ }\nstruct RoomInfo { latest_event: Option<LatestEvent> /* …many other fields */ }\nstruct Room { info: RwLock<RoomInfo> }\nimpl Room {\n    fn latest_event_is_local(&self) -> bool {\n        // compute the ONE consumed sub-field under the guard — no clone\n        self.info.read().unwrap().latest_event.as_ref().is_some_and(|ev| ev.sender_is_local)\n    }\n}".to_string(),
            context_notes: Some("Measured ParetoDominant 11.5-12.7x at 1K-100K calls (M3 Max release, load ~7.1, 2026-08-26; allocs 100000->0, bytes 4.7MB->0 @100K, alloc-det stable across invocations). HONEST MARGIN NOTE: the source's +18% throughput (the same matrix-rust-sdk room_list bench) is END-TO-END — the narrow-accessor patch was one of several stacked optimizations; the isolated mechanism (uncontended read-lock + ~47-64B guarded-payload clone with a heap alloc, vs read-lock + bool read) reads 11-13x on this box because the clone dominates the lock; under real read contention the gap widens further (the clone holds the guard longer). The alloc axis (exact N->0) is the deterministic half of the verdict. Type-blind for a parse-only proposer (cannot see payload width or which sub-field a consumer reads) — suggestion-only, the soa-hot-field precedent.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(lock_guard_narrow_field_accessor_bench_spec()),
        },
        // ── concurrency: one lock guarding a large item array →
        // K-items-per-cache-line-block locks ──────────────────────────
        // Source: Datadog RTDB/Monocle (Batch 78, Research 092 — the
        // group-by aggregation buffer: "we leverage this pattern by
        // grouping eight consecutive points into a cache line-aligned
        // block protected by a single mutex. This fine-grained locking
        // confines contention to within each small block and allows
        // parallel updates with little synchronization overhead" —
        // their per-worker-buffer replacement after the 2,100 GiB
        // DDSketch duplication). ADJACENT-BUT-DISTINCT from
        // align-64-false-sharing (Batch 65): that PADS independent
        // slots so unrelated writes stop sharing a line (eliminates
        // FALSE sharing between independent slots); THIS partitions
        // one logical array's TRUE sharing at line granularity — the
        // items ARE related (same buffer), the lock granularity is the
        // tuning axis (global → per-block). Both live on the coherence-
        // granularity axis; the detection shapes differ (per-thread
        // ordinal slots vs one guarded array under a single lock).
        PerfRule {
            rule_id: "cache-line-block-partitioned-lock".to_string(),
            name: "one Mutex guarding a large item array → K-items-per-cache-line-block locks (partition TRUE sharing at line granularity)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "// One lock guards the whole point array: every bump serializes.\nstruct Series {\n    points: std::sync::Mutex<Vec<u64>>, // all points under ONE lock\n}\nimpl Series {\n    fn bump(&self, i: usize) {\n        self.points.lock().unwrap()[i] += 1; // full serialization per bump\n    }\n}".to_string(),
            fixed_code: "// K points per cache-line-aligned block, one lock per block: bumps to\n// different blocks proceed in parallel; contention drops by the block\n// count (here 8 points/block — the Datadog group-by buffer).\n#[repr(align(128))] // the ARCH coherence line (64 x86 / 128 Apple Silicon)\nstruct Block {\n    lock: std::sync::Mutex<[u64; 8]>, // 8 consecutive points per block\n}\nstruct Series {\n    blocks: Vec<Block>,\n}\nimpl Series {\n    fn bump(&self, i: usize) {\n        let b = &self.blocks[i / 8];\n        b.lock.lock().unwrap()[i % 8] += 1; // contention ~ /8\n    }\n}".to_string(),
            context_notes: Some(
                "MEASURED ParetoDominant 1.12x/3.91x/2.98x/3.03x at 2/4/8/12 threads, 4096 points, 20K bumps/thread (M3 Max release, 2026-08-27; verdict-stable x2; second invocation 1.14/4.02/2.95/3.03 — same verdict band). The mechanism: one Mutex<Vec<u64>> serializes EVERY bump across all threads; 512 aligned Mutex<[u64; 8]> blocks confine contention to within-block pairs (~8x fewer collision opportunities — measured 2.5-4x end-to-end at 4-12t; the 2t 1.12x is the honest low-contention edge). REGIME: multi-threaded bump workloads over a shared array; a single-threaded bump loop pays the block-index division for nothing (do not apply). DIFFERENTIATION vs align-64-false-sharing (Batch 65): that PADS independent per-thread slots to eliminate FALSE sharing between unrelated writes; THIS partitions the TRUE sharing of ONE logical array at lock granularity — the items are related (same buffer), the lock scope is the tuning axis. The two compose: block-partition first (this rule), pad the per-thread stats after (that one). ALLOC AXIS EXEMPT (the first such rule): contended std::sync::Mutex on macOS allocates nondeterministically in the pthread slow path (probed: global-mutex 12t measured 41-44 allocs across runs; uncontended construction exactly 513/512 locks) — the exemption is probe-evidenced in bench.rs ALLOC_AXIS_EXEMPT, not a silent skip. Suggestion-only (type-blind). Source: Datadog RTDB/Monocle group-by aggregation buffer (datadoghq.com blog, 2025-08-04, Batch 78 / Research 092)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(cache_line_block_partitioned_lock_bench_spec()),
        },
        // ── arithmetic: per-digit decimal parse loop → SWAR 8-digit chunk ─────
        // Source: simd-lite/simd-json `numberparse.rs` (Batch 74, Research
        // 088 — the Research 085 pool-drop re-probe; the MCP fetch failure
        // was an indexing gap, the repo is alive at v0.18.1, 1.4k stars).
        // `parse_eight_digits_swar` folds 8 ASCII digits into a u32 with
        // three and/mul/shift steps (magic multipliers 2561 = 10*256+1,
        // 6553601 = 100*2^16+1, 42949672960001 = 10^4*2^32+1 — each step
        // merges adjacent digit pairs/quads/halves, byte 0 = most
        // significant, verified by construction + the exhaustive gate);
        // `is_made_of_eight_digits_fast` validates 8 bytes as digits with
        // one SWAR test (every byte in 0x30..=0x39 iff `(val & 0xF0F0..) |
        // ((val+0x0606..) & 0xF0F0..) >> 4 == 0x3333..` — high nibble must
        // be 3 AND low nibble +6 must not carry, Muła's 0x80.pl
        // swar-digits-validate, cited in-source). The per-digit loop pays a
        // serial dependency chain (`val = val*10 + d` — LLVM cannot
        // reassociate) of ~6 ops/digit; the chunk path is ~10 ops per 8
        // digits. Provenance: Muła 0x80.pl ← simdjson C++ ← simd-json port.
        // REGIME (the source's own README, `swar-number-parsing` feature):
        // "comes at a slight performance hit if most of the floats have
        // less than 8 digits" — gate the chunk path on >=8 digits
        // remaining; the tail stays per-digit. Composes with
        // `unaligned-word-load-from-bytes` (the chunk load IS that
        // pattern); sibling of `division-by-invariant-multiplication`
        // (magic-multiply family — radix folding vs divisor inversion).
        // The SSSE3 escalation (`_mm_maddubs_epi16`, computes 16 values,
        // "we are being wasteful") exists when target_feature("ssse3") is
        // STATIC — non-portable, citation only.
        PerfRule {
            rule_id: "swar-digit-chunk-parse".to_string(),
            name: "per-digit decimal parse loop → SWAR 8-digit chunk fold (3 multiplies per 8 digits)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn parse_u64(digits: &[u8]) -> u64 {\n    let mut val: u64 = 0;\n    let mut i = 0;\n    while i < digits.len() && digits[i].is_ascii_digit() {\n        // serial dependency chain: 1 iteration per digit, ~6 ops each —\n        // LLVM cannot reassociate val = val*10 + d\n        val = val * 10 + u64::from(digits[i] - b'0');\n        i += 1;\n    }\n    val\n}".to_string(),
            fixed_code: "fn parse_u64(digits: &[u8]) -> u64 {\n    let mut val: u64 = 0;\n    let mut i = 0;\n    while i + 8 <= digits.len() {\n        let chunk: [u8; 8] = digits[i..i + 8].try_into().unwrap();\n        if !is_made_of_eight_digits_fast(&chunk) {\n            break; // a non-digit byte sits in this window — take the tail path\n        }\n        // 8 digits in 3 multiplies (magic radix fold, Mula/simdjson lineage)\n        val = val.wrapping_mul(100_000_000).wrapping_add(u64::from(parse_eight_digits_swar(&chunk)));\n        i += 8;\n    }\n    while i < digits.len() && digits[i].is_ascii_digit() {\n        val = val * 10 + u64::from(digits[i] - b'0'); // tail: < 8 digits remain\n        i += 1;\n    }\n    val\n}\n\n/// True iff every byte is in 0x30..=0x39 (high nibble 3, low nibble +6 no carry).\nfn is_made_of_eight_digits_fast(chars: &[u8; 8]) -> bool {\n    let val = u64::from_ne_bytes(*chars);\n    ((val & 0xF0F0_F0F0_F0F0_F0F0)\n        | (((val.wrapping_add(0x0606_0606_0606_0606)) & 0xF0F0_F0F0_F0F0_F0F0) >> 4))\n        == 0x3333_3333_3333_3333\n}\n\n/// Folds 8 ASCII digits (byte 0 = most significant) into a u32 via three\n/// and/mul/shift steps — the pairs/quads/halves merge of the SWAR radix fold.\nfn parse_eight_digits_swar(chars: &[u8; 8]) -> u32 {\n    let val = u64::from_le_bytes(*chars); // byte 0 at the LOW position on every target\n    let val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;\n    let val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;\n    ((val & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32) as u32\n}".to_string(),
            context_notes: Some("Measured ParetoDominant 2.35x/1.75x/2.52x at 8/12/19 digits-per-run (N_RUNS=2000; M3 Max release, load ~7.1, 2026-08-26; 0/0 allocs both arms, alloc-det stable; second invocation 2.35x/1.75x/2.52x-equivalent band — verdict stable). REGIME: 1.01x TIE at 4 digits — the chunk loop never fires below 8 digits; this is the source's own README tradeoff verbatim ('comes at a slight performance hit if most of the floats have less than 8 digits' — gate on >=8 digits remaining). The mechanism: the per-digit loop is a serial dependency chain (val = val*10 + d — LLVM cannot reassociate in strict FP... integer here, but the chain is still latency-bound at ~1 imul+add per digit); the SWAR fold breaks it with 3 and/mul/shift steps per 8 digits. G1: exhaustive equivalence gate (all lengths 0..=19 x boundaries x 10K LCG strings x terminator positions + the is_made_of_eight_digits_fast validation matrix + 100K fold checks) — bit-identical values + stop positions vs the u128 reference. Domain: digit runs whose value fits u64 (intermediate chunk multiples never exceed the final value — proven + gated). Type-blind for a parse-only proposer (cannot prove a loop parses digits semantically) — suggestion-only.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(swar_digit_chunk_parse_bench_spec()),
        },
        // ── dispatch: CPU feature detection inside a per-chunk hot fn ────────
        // Source: simd-lite/simd-json `parse_eight_digits_unrolled` (Batch 74,
        // Research 088) — the in-source measured decision comment: "Only use
        // the SSSE3 version when the feature is statically available: a
        // per-call `is_x86_feature_detected!` check costs more than the SWAR
        // fallback (cached-atomic load + two outlined `target_feature` calls
        // per 8-digit group)". The detection macro resolves the feature-name
        // STRING at runtime (std::detect) + reads the cached detector — for
        // small work units (their 8-digit groups; any per-chunk dispatch in
        // the wild) the detection rivals the work itself. Fix: resolve once
        // (cached static / OnceLock consulted per chunk, or hoisted to the
        // caller) or gate statically via `#[cfg(target_feature)]` when the
        // target set is compile-time known.
        PerfRule {
            rule_id: "runtime-feature-detect-per-chunk".to_string(),
            name: "is_x86_feature_detected! inside a per-chunk hot fn → resolve once (cached static / static cfg gate)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "fn sum_chunk(data: &[u8]) -> u64 {\n    // CPU feature detection paid PER CHUNK: the feature-name string is\n    // resolved + the cached detector read on every call — for a work unit\n    // of a few ns the detection rivals the work itself\n    if is_x86_feature_detected!(\"avx2\") {\n        data.iter().map(|&b| u64::from(b)).sum()\n    } else {\n        data.iter().fold(0u64, |acc, &b| acc + u64::from(b))\n    }\n}\nfn process(chunks: &[Vec<u8>]) -> u64 {\n    chunks.iter().map(|c| sum_chunk(c)).sum()\n}".to_string(),
            fixed_code: "use std::sync::atomic::{AtomicU8, Ordering};\n\n// detection hoisted: the string is resolved ONCE, then consulted per chunk\n// via one cached byte (0 = unknown, 1 = yes, 2 = no)\nstatic HAS_AVX2: AtomicU8 = AtomicU8::new(0);\nfn has_avx2() -> bool {\n    match HAS_AVX2.load(Ordering::Relaxed) {\n        0 => {\n            let v = is_x86_feature_detected!(\"avx2\");\n            HAS_AVX2.store(if v { 1 } else { 2 }, Ordering::Relaxed);\n            v\n        }\n        1 => true,\n        _ => false,\n    }\n}\nfn sum_chunk(data: &[u8]) -> u64 {\n    if has_avx2() {\n        data.iter().map(|&b| u64::from(b)).sum()\n    } else {\n        data.iter().fold(0u64, |acc, &b| acc + u64::from(b))\n    }\n}\nfn process(chunks: &[Vec<u8>]) -> u64 {\n    chunks.iter().map(|c| sum_chunk(c)).sum()\n}".to_string(),
            context_notes: Some("MEASURED Mixed — 0/0/3 TIES on this box: 1.01x/1.04x/0.99x at 16/256/4096 B-per-chunk (M3 Max release, rustc 1.97, 2026-08-26; 0/0 allocs, alloc-det stable, verdict stable x2 invocations). HONEST FINDING: on current rustc the feature-name match on a string literal const-folds/optimize away to near-zero even through an #[inline(never)] boundary — the per-chunk detection machinery (std::detect string resolution + cached detector read) measured ~sub-0.1ns marginal on aarch64. The cost class the rule encodes SURVIVES in: (a) outlined #[target_feature] dispatch (the source's measured x86 finding — 'a per-call is_x86_feature_detected! check costs more than the SWAR fallback (cached-atomic load + two outlined target_feature calls per 8-digit group)', simd-json parse_eight_digits_unrolled), (b) non-literal/runtime-composed feature strings, (c) heavier probe libraries. The rule exists to encode the granularity DECISION (resolve once: cached static, hoist to caller, or static cfg gate) with an honest measured-Tie on the modern-toolchain baseline — the B68 honest-annotation precedent; do NOT cite it as a default-config hot-path win.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(runtime_feature_detect_per_chunk_bench_spec()),
        },
        // ── allocation: dictionary-encoded column consumed through a generic ──
        // to_string materializer → borrowing layout-enum view ────────────────
        // Source: GreptimeDB "Just One Function, 10x Faster?" (PR #8587,
        // `recordbatches_to_timeseries` in src/servers/src/prom_store.rs;
        // Batch 75, Research 089). THE DICTIONARY TRAP, quoted: "the
        // dictionary-encoded group is 35% slower than the plain string
        // group... Dictionary encoding exists to save memory and copies, so
        // why is it slower?" Root cause: "`iter_column_as_string` does not
        // recognize dictionary columns. It falls back to the generic path and
        // calls `to_string()` row by row. The 3 strings become 1000, every
        // allocation upstream avoided is made again here, plus one extra
        // dictionary lookup per row." (A Dictionary<UInt32, Utf8> column
        // stores few distinct strings + a u32 index per row; a label layout
        // like `host` has few distinct values across many rows.) Fix: an enum
        // `LabelValues<'a>` with one variant per layout whose
        // `value(row) -> Option<&str>` "borrows straight from the array — no
        // copy and no allocation"; the dictionary branch does key(row) then
        // value(key). Measured in-source END-TO-END over the whole rewrite:
        // dictionary columns improved the most (55.6ms → 3.38ms at
        // adjacent/1000×100 = 16.5×) — never cite as this rule's isolated
        // number. Composes with `group-keys-borrow-allocate-on-first-sight`
        // (the source fix was both); distinct from `avoid-clone` (one clone
        // at a fn boundary; this is R clones per pass through a generic
        // accessor that erases the encoding) and `collect-avoidance`
        // (intermediate Vec staging; the expansion happens INSIDE the
        // accessor). Suggestion-only (type-blind: a parse-only proposer
        // cannot prove a column is dictionary/interned-encoded — the
        // soa-hot-field precedent).
        PerfRule {
            rule_id: "encoded-layout-borrowed-view".to_string(),
            name: "generic to_string() column materializer over dictionary-encoded data → borrowing layout-enum view (Option<&str>)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "struct DictColumn {\n    values: Vec<String>, // few distinct strings — what dictionary encoding is FOR\n    keys: Vec<u32>,     // one u32 per row indexing into values\n}\nimpl DictColumn {\n    fn lookup(&self, row: usize) -> Option<&str> {\n        let k = *self.keys.get(row)?;\n        self.values.get(k as usize).map(String::as_str)\n    }\n}\n// The generic accessor does not recognize the dictionary layout: it\n// materializes owned Strings row by row — \"the 3 strings become 1000\",\n// every allocation the encoding saved is made again here, plus one extra\n// dictionary lookup per row.\nfn column_as_strings(col: &DictColumn) -> Vec<Option<String>> {\n    (0..col.keys.len()).map(|r| col.lookup(r).map(|s| s.to_string())).collect()\n}\nfn label_len_total(col: &DictColumn) -> usize {\n    let cells = column_as_strings(col); // R heap allocs\n    cells.iter().map(|c| c.as_ref().map_or(0, |s| s.len())).sum()\n}".to_string(),
            fixed_code: "struct DictColumn {\n    values: Vec<String>,\n    keys: Vec<u32>,\n}\n/// One variant per column layout. `value` borrows straight from the array:\n/// \"no copy and no allocation\" — the dictionary branch does key(row) then\n/// value(key), reusing the same S distinct strings for all R rows instead\n/// of expanding into R owned Strings. Holding nothing but &refs makes the\n/// borrow contract compiler-enforced.\nenum ColumnView<'a> {\n    Utf8(&'a [String]),\n    Dictionary { values: &'a [String], keys: &'a [u32] },\n}\nimpl ColumnView<'_> {\n    fn value(&self, row: usize) -> Option<&str> {\n        match self {\n            ColumnView::Utf8(v) => v.get(row).map(String::as_str),\n            ColumnView::Dictionary { values, keys } => keys\n                .get(row)\n                .and_then(|&k| values.get(k as usize).map(String::as_str)),\n        }\n    }\n}\nfn label_len_total(col: &DictColumn) -> usize {\n    let view = ColumnView::Dictionary { values: &col.values, keys: &col.keys };\n    (0..col.keys.len()).map(|r| view.value(r).map_or(0, str::len)).sum()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 41.98x/39.38x/29.95x at R=10K/100K/1M, allocs 4,000,004 -> 0 + bytes 156.0 MB -> 0 at R=1M (S=100, L=4 (M3 Max release, rustc 1.97, 2026-08-26, load ~5.9 of 16; alloc-det stable). REGIME (distinct-values ratio S << R): the win is R·L owned-String materializations becoming S reused borrows — it grows with rows-per-distinct-value. At S ≈ R (mostly-distinct values) the encoding itself is pointless and the borrowed view saves ~nothing. Distinct from `avoid-clone` (one clone at a fn boundary; this is R clones per pass through a generic accessor that erases the encoding) and from `collect-avoidance` (the expansion happens INSIDE the accessor, not at a collect). The bench CONSUMES the view (black_box'd length sum) — an unread `&str` costs zero and the ratio inflates. The source's \"dictionary 35% slower than plain strings\" anomaly is its generic-fallback path (extra dictionary lookup per row on top of the expansion) — our bench measures the expansion cost only; never cite 35% as the expected win. Suggestion-only (type-blind: a parse-only proposer cannot prove a column is dictionary-encoded). Source: GreptimeDB PR #8587 walkthrough (2026-07-31), Steps 3-4 — the dictionary encoding upstream deliberately kept (#8541) was silently expanded downstream by one `iter_column_as_string`.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(encoded_layout_borrowed_view_bench_spec()),
        },
        // ── allocation: per-row owned group key → hash borrowed labels, ──────
        // allocate on first sight, one final sort ─────────────────────────────
        // Source: same article, THE COST MODEL + THE GROUPING REWRITE. Old:
        // "R·L calls to to_string()... one Vec<Label> per row, so R heap
        // allocations; one clone each for every label name and value, another
        // R·L string copies; and one BTreeMap lookup per row, each on the
        // order of log(S) comparisons." Fix: "A row only reaches
        // `new_timeseries` once it is judged to be a new series... R·L copies
        // become S·L copies. The whole optimization moves allocation from
        // 'every iteration' to 'the iteration that finds something new.'"
        // The decision content: std `HashMap::entry`/`BTreeMap::entry`
        // require an OWNED key, so the natural map APIs FORCE per-row key
        // materialization — the manual hash index (hash borrowed labels →
        // `HashMap<u64, Vec<usize>>` candidate list → FULL compare →
        // materialize only on first sight) keeps one hash per row and S
        // total allocations. CORRECTNESS CONTRACT: (1) the map value is a
        // candidate LIST — `HashMap<u64, usize>` merges distinct groups on
        // hash collision ("Merging two different series for the sake of
        // speed is a mistake this function cannot make"); (2) output order
        // flips from key-sorted to first-appearance unless the final
        // `sort_unstable_by` ships; (3) with optional fields skipped, one
        // key can be a PREFIX of another — the compare must verify
        // exhaustiveness. Suggestion-only (architectural — the
        // soa-hot-field class; a parse-only proposer cannot see that keys
        // repeat across rows).
        PerfRule {
            rule_id: "group-keys-borrow-allocate-on-first-sight".to_string(),
            name: "per-row owned group key (clone into map key every row) → hash borrowed labels, clone key only on first sight + one final sort".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use std::collections::BTreeMap;\nstruct Row {\n    labels: Vec<(String, String)>, // the series' (name, value) label set\n    value: u64,\n}\nfn group_rows(rows: &[Row]) -> Vec<(Vec<(String, String)>, (u64, u64))> {\n    let mut map: BTreeMap<Vec<(String, String)>, (u64, u64)> = BTreeMap::new();\n    for row in rows {\n        // per row: 1 Vec + 2·L String clones — R heap allocations where S\n        // would do — plus a BTreeMap insert at O(log S) ordered-string\n        // comparisons\n        let key: Vec<(String, String)> = row.labels.clone();\n        let e = map.entry(key).or_insert((0, 0));\n        e.0 += 1;\n        e.1 += row.value;\n    }\n    map.into_iter().collect() // BTreeMap gives sorted output for free\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nuse std::collections::hash_map::DefaultHasher;\nuse std::hash::{Hash, Hasher};\nstruct Row {\n    labels: Vec<(String, String)>,\n    value: u64,\n}\nstruct Group {\n    key: Vec<(String, String)>,\n    count: u64,\n    sum: u64,\n}\n// exhaustive compare — with optional fields skipped one key can be a\n// PREFIX of another; the length check prevents {host=a} matching\n// {host=a, env=prod}\nfn key_matches(key: &[(String, String)], row: &Row) -> bool {\n    key.len() == row.labels.len()\n        && key.iter().zip(&row.labels).all(|(a, b)| a == b)\n}\nfn group_rows(rows: &[Row]) -> Vec<Group> {\n    // hash → candidate index list, FULL label comparison on each candidate\n    // — \"merging two different series for the sake of speed is a mistake\n    // this function cannot make\"\n    let mut index: HashMap<u64, Vec<usize>> = HashMap::new();\n    let mut groups: Vec<Group> = Vec::new();\n    for row in rows {\n        let mut h = DefaultHasher::new();\n        for (n, v) in &row.labels {\n            n.hash(&mut h); // borrowed — no alloc\n            v.hash(&mut h);\n        }\n        let hash = h.finish();\n        let idx = match index.get(&hash).and_then(|c| c.iter().find(|&&i| key_matches(&groups[i].key, row))) {\n            Some(&i) => i,\n            None => {\n                let i = groups.len();\n                // allocated ON FIRST SIGHT only — \"the whole optimization\n                // moves allocation from 'every iteration' to 'the iteration\n                // that finds something new': R·L copies become S·L\n                groups.push(Group { key: row.labels.clone(), count: 0, sum: 0 });\n                index.entry(hash).or_default().push(i);\n                i\n            }\n        };\n        groups[idx].count += 1;\n        groups[idx].sum += row.value;\n    }\n    // one S·log(S) sort is negligible against what was saved — restores\n    // the BTreeMap's ordered output contract\n    groups.sort_unstable_by(|a, b| a.key.cmp(&b.key));\n    groups\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 — 1.92x/2.27x/2.37x at R=10K/100K/300K (S=1000, L=3; M3 Max release, rustc 1.97, 2026-08-26, load ~5.9 of 16), allocs 70,133/700,133/2,100,133 -> 8,019 FLAT (the 7R+2S -> 8S+30 model, exact), bytes 55.9 MB -> 435 KB at 300K (alloc-det stable). REGIME (dedup ratio R/S): the win is R·L owned-key clones becoming S·L — it grows linearly with rows-per-group. At S ≈ R (mostly-distinct keys — every row a new group) first-sight allocation IS per-row allocation and the fixed form adds the hash pass + the final S·log(S) sort as pure overhead; gate on repeated-key grouping. The decision content: std `HashMap::entry`/`BTreeMap::entry` require an OWNED key, forcing per-row key materialization — the manual hash index (hash borrowed → `HashMap<u64, Vec<usize>>` candidates → FULL compare → materialize on first sight) keeps one hash per row and S total allocations. CORRECTNESS CONTRACT: (1) the map value is a candidate LIST — `HashMap<u64, usize>` merges distinct groups on hash collision; (2) output order flips from key-sorted to first-appearance unless the final `sort_unstable_by` ships — include it unless nothing downstream reads the order; (3) with optional fields skipped one key can be a PREFIX of another — the compare must verify exhaustiveness. Mirror regime of `btreemap-sorted-iteration` (repeated sorted reads → maintain a BTreeMap; single grouping pass → build + sort once) and distinct from `sort-key-cache-precompute-out-of-lock` (maintained comparator cache vs first-sight allocation in a scan). Bench isolates the grouping mechanism (borrowed inputs in both arms) — the source's 10.7-16.5x adjacent / 4.1-5.5x interleaved headline includes the borrowing + fast-path mechanisms stacked; never cite it for this rule alone. The map-mechanics swap alone is S-dependent (small S: short-circuiting BTreeMap compares can beat SipHash) — the load-bearing half is the R·L → S·L copy elimination. DefaultHasher (SipHash) is the faithful baseline; hasher swaps are an experiment, not the rule. Suggestion-only (architectural, the soa-hot-field class).".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(group_keys_borrow_allocate_on_first_sight_bench_spec()),
        },
        // ── complexity: hash-every-row grouping → previous-row group check ───
        // first, hash fallback on miss ────────────────────────────────────────
        // Source: same article, THE FAST PATH: "Comparing the current row
        // against the series the previous row landed on is enough to reuse
        // it on a hit, with no hash to compute. This only optimizes a common
        // local ordering. It can break across partitions and across ranges,
        // and correctness does not depend on it." Measured axis (END-TO-END,
        // all p=0.00): adjacent 10.7-16.5x vs interleaved 4.1-5.5x — the
        // fast-path hit rate decides (adjacent = samples of one series
        // contiguous, the real query shape: rows sorted by primary key).
        // DERIVED (not stated in the source): the fixed arm's own
        // adjacent-vs-interleaved ratio is ~2.1-2.8x — the honest isolated
        // bound for this mechanism. Sibling of `bsearch-sorted-slice` +
        // `sortedness-check-branchless-accumulate` (all exploit input order).
        // Composes AFTER `group-keys-borrow-allocate-on-first-sight` (the
        // source shipped both). Suggestion-only (a parse-only proposer
        // cannot prove consecutive rows share keys).
        PerfRule {
            rule_id: "sorted-input-previous-row-fast-path".to_string(),
            name: "hash-every-row grouping → previous-row group check first (no hash on sorted-run hit, hash fallback on miss)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\nstruct Row {\n    series: [u64; 2], // the label-set key\n    value: u64,\n}\nstruct Group {\n    series: [u64; 2],\n    count: u64,\n    sum: u64,\n}\nfn group_rows(rows: &[Row]) -> Vec<Group> {\n    let mut index: HashMap<[u64; 2], usize> = HashMap::new();\n    let mut groups: Vec<Group> = Vec::new();\n    for row in rows {\n        // hash EVERY row — even when it lands on the same series as the\n        // previous row, the common case for primary-key-sorted input\n        let idx = match index.get(&row.series) {\n            Some(&i) => i,\n            None => {\n                let i = groups.len();\n                groups.push(Group { series: row.series, count: 0, sum: 0 });\n                index.insert(row.series, i);\n                i\n            }\n        };\n        groups[idx].count += 1;\n        groups[idx].sum += row.value;\n    }\n    groups\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nstruct Row {\n    series: [u64; 2],\n    value: u64,\n}\nstruct Group {\n    series: [u64; 2],\n    count: u64,\n    sum: u64,\n}\nfn group_rows(rows: &[Row]) -> Vec<Group> {\n    let mut index: HashMap<[u64; 2], usize> = HashMap::new();\n    let mut groups: Vec<Group> = Vec::new();\n    let mut prev: Option<usize> = None;\n    for row in rows {\n        // compare against the series the PREVIOUS row landed on first —\n        // no hash to compute on hit. \"This only optimizes a common local\n        // ordering. It can break across partitions and across ranges, and\n        // correctness does not depend on it\" — the hash fallback with\n        // full-key equality is the semantic contract.\n        let idx = match prev {\n            Some(i) if groups[i].series == row.series => i,\n            _ => match index.get(&row.series) {\n                Some(&i) => i,\n                None => {\n                    let i = groups.len();\n                    groups.push(Group { series: row.series, count: 0, sum: 0 });\n                    index.insert(row.series, i);\n                    i\n                }\n            },\n        };\n        prev = Some(idx);\n        groups[idx].count += 1;\n        groups[idx].sum += row.value;\n    }\n    groups\n}".to_string(),
            context_notes: Some("MEASURED Mixed 1/0/1 — adjacent (run=100) 12.92x FASTER, interleaved (run=1) 0.99x tie (R=100K, S=1000; M3 Max release, rustc 1.97, 2026-08-26, load ~5.9 of 16; allocs 19/19 equal by design, alloc-det stable). REGIME (fast-path hit rate = input locality): adjacent (sorted-run) input pays ~S hashes instead of R — the previous-row comparison reuses the group with no hash to compute; interleaved input pays one failed comparison per row before the hash — expected Tie-to-slightly-Slower, the honest bound of the regime. DERIVED bound (not stated in the source; derived from the source's fixed-arm adjacent-vs-interleaved ratios): ~2.1-2.8x isolated — never cite the source's 10-16x END-TO-END totals for this mechanism alone (they stack the borrowing + grouping + fast-path optimizations). The win scales with the hash-vs-compare cost ratio: long composite string keys maximize it (SipHash over all L labels vs a compare short-circuiting at the first diff); short/scalar keys shrink it toward a tie. CORRECTNESS CONTRACT: the fast path uses the SAME full-equality predicate as the fallback (one predicate, no divergent comparisons), the predicate is exhaustive on variable-length keys (a skipped-field key can be a PREFIX of another), `previous` updates every row, and a miss always falls through to hash+full-compare — the fast path is deletable without changing output (\"correctness does not depend on it\"). Family note: this ADDS a cheap pre-check before expensive per-row work (hash+probe) — it does not contradict `sortedness-check-branchless-accumulate`/`filter-count-to-any` (those remove per-element early exits inside vectorizable scans; the branch there guards ns of work, here it guards the dominant per-row cost). Suggestion-only (a parse-only proposer cannot prove consecutive rows share keys). BATCH 361 FACE — THE SORTED POLE (count-neutral, Plan 372 / Research 405 residue, pola-rs/polars @ 5c36cdd75): when the keys are already sorted (just sorted by this code, or carried sorted), the previous-row check NEVER misses and the hash fallback NEVER fires — the hash structure can go entirely: group ids advance only at value changes (polars-core create_groups_from_sorted: 'group_by keys are sorted; running sorted key fast path'; upstream PRESENT ×2 — the streaming hot-table's run-start insertion, re-worded upstream since grade: 'A missed key that is followed by a key with the same hash is inserted even if that evicts another key ... This keeps sorted keys from first going cold', the run-start law generalized to hash-locality). This rule owns the run-LOCALITY pole (unsorted input, per-row pre-check); this face is its limit case — at perfect locality the pre-check IS the grouping. The statement-sequence kill risk (a sort call + a grouping loop) resolves WITH the face: the sorted pole rides a host that is already suggestion-only, never a standalone detector.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sorted_input_previous_row_fast_path_bench_spec()),
        },
    ]
}

/// Bench spec for `sortedness-check-branchless-accumulate`: early-exit
/// branch-per-element validation vs branchless compare+AND accumulate
/// on USUALLY-SORTED data.
///
/// Strictly increasing u64s (the applicable regime — the early-exit
/// branch never fires, so the buggy form pays scalar compare+branch per
/// element for nothing; the accumulate form has no exit and vectorizes).
/// On early-violation data the regime flips (early exit wins) — the
/// mirrored regime of `filter-count-to-any`, recorded in the annotation.
#[cfg(feature = "rust_perf_bench")]
fn sortedness_check_branchless_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000],
        harness: "\nmod buggy {\n    pub fn validate_sorted(vals: &[u64]) -> bool {\n        let mut prev = u64::MIN;\n        for &v in vals {\n            if v < prev { return false; }\n            prev = v;\n        }\n        true\n    }\n}\n\nmod fixed {\n    pub fn validate_sorted(vals: &[u64]) -> bool {\n        let mut sorted = true;\n        let mut prev = u64::MIN;\n        for &v in vals {\n            sorted &= prev <= v;\n            prev = v;\n        }\n        sorted\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u64> {\n    // Strictly increasing (usually-sorted regime — the applicable window:\n    // the early-exit branch never fires, so the buggy form pays a scalar\n    // compare+branch per element for nothing).\n    (0..n).map(|i| (i as u64).wrapping_mul(2)).collect()\n}\n\npub fn run_buggy(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let ok = buggy::validate_sorted(input);\n    std::hint::black_box(ok);\n}\n\npub fn run_fixed(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let ok = fixed::validate_sorted(input);\n    std::hint::black_box(ok);\n}\n".to_string(),
    }
}

/// Bench spec for `byte-find-position-to-memchr`: scalar early-exit
/// position scan vs the memchr crate's SIMD block scan.
///
/// Needle at n-16 (~90% position on the larger sizes — both arms do a
/// ~full scan; stray delimiters scrubbed in setup so both arms find the
/// same index). 32B = the honest small-regime arm (the Teddy-bound wash
/// where call overhead eats the win); 4K/64K = the SIMD win regime.
/// Deterministic positions; `Option<usize>` identical (bit-identical).
#[cfg(feature = "rust_perf_bench")]
fn byte_find_position_to_memchr_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![32, 256, 4_096, 65_536],
        harness: "// [dep: memchr = \"2\"]\nmod buggy {\n    pub fn find_delim(buf: &[u8]) -> Option<usize> {\n        buf.iter().position(|&b| b == b';')\n    }\n}\n\nmod fixed {\n    pub fn find_delim(buf: &[u8]) -> Option<usize> {\n        memchr::memchr(b';', buf)\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u8> {\n    // Pseudo-random bytes; stray delimiters scrubbed so the FIRST ';' sits\n    // at n-16 (both arms do ~full scan — the representative regime; the\n    // scrub runs in setup, never measured).\n    let mut v: Vec<u8> = (0..n)\n        .map(|i| ((i as u32).wrapping_mul(2654435761) >> 24) as u8)\n        .collect();\n    for b in &mut v {\n        if *b == b';' { *b = b':'; }\n    }\n    if n >= 16 {\n        v[n - 16] = b';';\n    }\n    v\n}\n\npub fn run_buggy(input: &[u8]) {\n    let input = std::hint::black_box(input);\n    let at = buggy::find_delim(input);\n    std::hint::black_box(at);\n}\n\npub fn run_fixed(input: &[u8]) {\n    let input = std::hint::black_box(input);\n    let at = fixed::find_delim(input);\n    std::hint::black_box(at);\n}\n".to_string(),
    }
}

/// Bench spec for `varint-compose-word-single-store` (Batch 68, Research
/// 075 — the tantivy distill): per-byte `write_all` writer calls vs the
/// 5-threshold ladder composing all bytes into one u64 + ONE `write_all`.
///
/// THE WRITER SHAPE IS THE RULE'S REGIME (first bench run, load-recorded:
/// the Vec-push shape measures `ParetoDominated` 0/3 — 128 is a compile-time
/// constant so LLVM strength-reduces the div/mod, and `Vec::push-per-byte` is
/// cheap enough that the ladder's if-chain loses; the honest win is N
/// writer calls → 1). Sink: `std::io::Cursor<&mut Vec<u8>>` — a real
/// generic `Write` impl with per-call branching but no syscall noise; on a
/// virtual `Box<dyn Write>` or a real file/socket writer the per-call cost
/// — and therefore the win — only grows. Input mixes ALL five serialized
/// widths deterministically (i % 5 selects the magnitude band). Stop-bit-
/// on-last convention; outputs bit-identical by the exhaustive corpus gate.
#[cfg(feature = "rust_perf_bench")]
fn varint_compose_word_single_store_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
use std::io::Write;

// Dyn-dispatch sink: the write path is IDENTICAL work (extend_from_slice)
// for both arms — the measured differential is purely N vtable calls vs 1
// (the production shape for trait-object writers; fully-inlined generic
// sinks measure ~parity, see the rule's context note).
struct Sink(Vec<u8>);
impl std::io::Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
fn new_sink(cap: usize) -> Box<dyn std::io::Write> {
    Box::new(Sink(Vec::with_capacity(cap)))
}

mod buggy {
    use std::io::{Result, Write};
    pub fn write_vint<W: Write>(w: &mut W, mut val: u32) -> Result<()> {
        loop {
            let byte = (val % 128) as u8;
            val /= 128;
            if val == 0 {
                w.write_all(&[byte | 128])?; // a writer call PER BYTE
                return Ok(());
            }
            w.write_all(&[byte])?;
        }
    }
}

mod fixed {
    use std::io::{Result, Write};
    pub fn write_vint<W: Write>(w: &mut W, val: u32) -> Result<()> {
        let v = u64::from(val);
        let (res, len) = if v < (1 << 7) {
            (v | 128, 1)
        } else if v < (1 << 14) {
            ((v & 127) | ((v & (127 << 7)) << 1) | (128 << 8), 2)
        } else if v < (1 << 21) {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | (128 << 16), 3)
        } else if v < (1 << 28) {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | (128 << 24), 4)
        } else {
            ((v & 127) | ((v & (127 << 7)) << 1) | ((v & (127 << 14)) << 2) | ((v & (127 << 21)) << 3) | ((v & (127 << 28)) << 4) | (128u64 << 32), 5)
        };
        w.write_all(&res.to_le_bytes()[..len]) // ONE store, ONE call
    }
}

pub struct Input {
    values: Vec<u32>,
    total_bytes: usize,
}

pub fn make_input(n: usize) -> Input {
    // Deterministic mix across all five magnitude bands (every ladder arm +
    // every loop trip count): band i%5 selects a value in the 1..5-byte range.
    let mut values = Vec::with_capacity(n);
    let mut total_bytes = 0usize;
    for i in 0..n {
        let band = (i % 5) as u32;
        let val: u32 = if band == 4 {
            u32::MAX - ((i as u32) % 64) // >= 2^28: 5-byte band
        } else {
            (1u32 << (7 * band)) + ((i as u32) & ((1u32 << (7 * band)) - 1))
        };
        values.push(val);
        total_bytes += band as usize + 1;
    }
    Input { values, total_bytes }
}

pub fn run_buggy(input: &Input) {
    let mut cur = new_sink(input.total_bytes + 16);
    for &v in std::hint::black_box(&input.values) {
        buggy::write_vint(&mut cur, v).unwrap();
    }
    std::hint::black_box(&cur);
}

pub fn run_fixed(input: &Input) {
    let mut cur = new_sink(input.total_bytes + 16);
    for &v in std::hint::black_box(&input.values) {
        fixed::write_vint(&mut cur, v).unwrap();
    }
    std::hint::black_box(&cur);
}"#
            .to_string(),
    }
}

/// Bench spec for `unaligned-word-load-from-bytes` (Batch 69, Research
/// 075 §Secondary — the Plan 013 T6 refutation): the 8-term
/// shift/or byte-assembly chain vs ONE unaligned `from_le_bytes` load.
///
/// **Cross-arch probe 2026-08-19 (median-of-5, interleaved arms): u64
/// 2.17-2.23x on aarch64 (M3, load ~118 shared) / 1.39-1.47x on x86 (4090
/// box, load 7-31); u32 1.27-1.38x / 1.20-1.29x — `ParetoDominant` BOTH
/// arches BOTH widths.** The mechanism is LOAD-COUNT (8 L1 loads + the
/// serial shift/or chain vs 1 fused load) — distinct from the varint
/// honest-loser's call-count class (`OoO` hides N dyn-dispatch calls; it
/// cannot hide 8 loads behind 1). The u32 family shares the buffer; the
/// harness measures the u64 headline (the family's worst-case-for-buggy
/// shape). G1: bit-identical at EVERY byte alignment (the exhaustive corpus
/// test below).
#[cfg(feature = "rust_perf_bench")]
fn unaligned_word_load_from_bytes_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000, 10_000_000],
        harness: r#"
mod buggy {
    pub fn read_u64_le(data: &[u8], addr: usize) -> u64 {
        (data[addr] as u64)
            | ((data[addr + 1] as u64) << 8)
            | ((data[addr + 2] as u64) << 16)
            | ((data[addr + 3] as u64) << 24)
            | ((data[addr + 4] as u64) << 32)
            | ((data[addr + 5] as u64) << 40)
            | ((data[addr + 6] as u64) << 48)
            | ((data[addr + 7] as u64) << 56)
    }
}

mod fixed {
    pub fn read_u64_le(data: &[u8], addr: usize) -> u64 {
        let bytes: [u8; 8] = (&data[addr..addr + 8]).try_into().unwrap();
        u64::from_le_bytes(bytes)
    }
}

pub struct Input {
    data: Vec<u8>,
    idxs: Vec<usize>,
}

pub fn make_input(n: usize) -> Input {
    // 4 KiB buffer (L1-resident — the load-count mechanism isolated from
    // cache-miss noise) + a deterministic address scatter hitting every
    // byte alignment.
    const BUF: usize = 4096;
    let data: Vec<u8> = (0..BUF as u32)
        .map(|i| (i.wrapping_mul(2_654_435_761) >> 24) as u8)
        .collect();
    let idxs: Vec<usize> = (0..n).map(|i| i.wrapping_mul(48_271) % (BUF - 8)).collect();
    Input { data, idxs }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for &a in &input.idxs {
        acc = acc.wrapping_add(buggy::read_u64_le(&input.data, a));
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for &a in &input.idxs {
        acc = acc.wrapping_add(fixed::read_u64_le(&input.data, a));
    }
    std::hint::black_box(acc);
}"#
        .to_string(),
    }
}

/// Bench spec for `sort-key-cache-precompute-out-of-lock` (Batch 73,
/// Research 085/086 — the mnt.io room-list distill): comparator that
/// acquires a read-lock + clones a 144 B `LatestEventValue` per invocation
/// vs a maintained sort-key cache refreshed once per data-update cycle
/// (the production `RoomListItem` pattern — refresh via NARROW reads, the
/// narrow-accessor rule composed in).
///
/// sizes = SORTS PER REFRESH CYCLE (the amortization axis — the reactive
/// stream regime: every filter change / pagination reset does a fresh full
/// sort over the same data, and eyeball-im-util's `SortBy` re-invokes the
/// comparator from `binary_search_by` on every diff). `N_ROOMS` = 512 is the
/// article's 500-600 room scale. Every sort is preceded by a fixed
/// deterministic Fisher-Yates permutation (in-place, no allocs, the SAME
/// swap list in both arms) — a pseudo-random permutation has ~N/2 runs so
/// the adaptive driftsort pays the full ~N·log N comparisons every sort
/// (a plain reverse would be ONE detectable run, O(N) — measured: the
/// reverse-protocol run showed only ~512 comparisons/sort vs the ~4600
/// the shuffle protocol pays). G1: the final order is always desc —
/// position-weighted checksum asserted against a make_input-computed
/// expectation, in both arms.
/// Alloc axis: buggy ≈ 2 allocs/comparison (the String body clone ×2
/// accessor calls) vs fixed ≈ 1/sort (the sort's internal scratch).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn sort_key_cache_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 4, 16],
        harness: r#"
use std::cell::RefCell;
use std::sync::RwLock;

mod buggy {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        pub timestamp: u64,
        pub body: String, // ~64 B heap — the guarded payload footprint
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // The article's `Room::latest_event` shape: read-lock acquire +
        // clone of the WHOLE value per call. The comparator invokes it
        // twice per comparison, ~N·log N comparisons per sort.
        pub fn latest_event_timestamp(&self) -> Option<u64> {
            self.info.read().unwrap().latest_event.clone().map(|ev| ev.timestamp)
        }

        // Verification-only narrow reader (NOT the measured sort path).
        pub fn verify_ts(&self) -> Option<u64> {
            self.info.read().unwrap().latest_event.as_ref().map(|ev| ev.timestamp)
        }
    }

    pub fn sort_desc(rooms: &mut [Room]) {
        rooms.sort_by(|a, b| b.latest_event_timestamp().cmp(&a.latest_event_timestamp()));
    }
}

mod fixed {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        pub timestamp: u64,
        // The payload width IS the mechanism — only the buggy arm's clone
        // ever reads it; the narrow refresh reads just the timestamp.
        #[allow(dead_code)]
        pub body: String,
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // Narrow read through the guard — no clone (the narrow-accessor
        // rule composed into the refresh path).
        pub fn latest_event_timestamp(&self) -> Option<u64> {
            self.info.read().unwrap().latest_event.as_ref().map(|ev| ev.timestamp)
        }
    }

    /// The production `RoomListItem` facade: cached sort key refreshed
    /// once per data update (a Reset cycle refreshes all N — the
    /// amortization cost side).
    pub struct RoomListItem {
        pub inner: Room,
        pub cached_ts: Option<u64>,
    }

    impl RoomListItem {
        pub fn refresh_cached_data(&mut self) {
            self.cached_ts = self.inner.latest_event_timestamp();
        }
    }

    pub fn refresh_all(items: &mut [RoomListItem]) {
        for it in items.iter_mut() {
            it.refresh_cached_data();
        }
    }

    pub fn sort_desc(items: &mut [RoomListItem]) {
        items.sort_by(|a, b| b.cached_ts.cmp(&a.cached_ts));
    }
}

const N_ROOMS: usize = 512; // the article's 500-600 room scale

pub struct Input {
    // Interior mutability: the working vec persists across validator calls
    // (no per-call clone — both arms sort in place).
    buggy_rooms: RefCell<Vec<buggy::Room>>,
    fixed_items: RefCell<Vec<fixed::RoomListItem>>,
    // Deterministic Fisher-Yates swap list (fixed LCG seed), applied
    // in-place before every sort in BOTH arms — the same permutation
    // sequence, so comparison counts are identical across arms.
    shuffle: Vec<(usize, usize)>,
    sorts_per_cycle: usize,
    expected_desc: u64,
}

fn make_body(i: u64) -> String {
    format!("event body {i:04} padding padding padding padding")
}

pub fn make_input(n: usize) -> Input {
    let mut buggy_rooms = Vec::with_capacity(N_ROOMS);
    let mut rooms = Vec::with_capacity(N_ROOMS);
    let mut keys: Vec<u64> = Vec::with_capacity(N_ROOMS);
    for i in 0..N_ROOMS as u64 {
        let ts = i * 7 + 3; // distinct → total order, G1-deterministic
        rooms.push(fixed::Room {
            info: RwLock::new(fixed::RoomInfo {
                latest_event: Some(fixed::LatestEvent { timestamp: ts, body: make_body(i) }),
            }),
        });
        buggy_rooms.push(buggy::Room {
            info: RwLock::new(buggy::RoomInfo {
                latest_event: Some(buggy::LatestEvent { timestamp: ts, body: make_body(i) }),
            }),
        });
        keys.push(ts);
    }
    // Reverse both so call #1's entry state is anti-sorted for its first sort.
    // Reverse both so call #1's entry state is unsorted.
    buggy_rooms.reverse();
    rooms.reverse();
    let fixed_items: Vec<fixed::RoomListItem> = rooms
        .into_iter()
        .map(|inner| fixed::RoomListItem { inner, cached_ts: None })
        .collect();

    // Deterministic Fisher-Yates swap list (fixed LCG seed). A PLAIN
    // reverse() before each sort would be WRONG here: a perfectly
    // reversed input is ONE run for the adaptive driftsort — it detects
    // + reverses it in O(N) with ~N comparisons. A fixed pseudo-random
    // permutation has ~N/2 runs → the full ~N·log N merge comparisons,
    // the honest reactive-stream re-sort cost.
    let mut swap: Vec<(usize, usize)> = Vec::with_capacity(N_ROOMS);
    let mut rng: u64 = 0x243F_6A88_85A3_08D3;
    for i in (1..N_ROOMS).rev() {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let j = ((rng >> 33) as usize) % (i + 1);
        swap.push((i, j));
    }

    // G1 expectation: position-weighted checksum of the desc order,
    // computed from the key set alone (independent of either arm).
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    let expected_desc: u64 =
        sorted.iter().rev().enumerate().map(|(i, &k)| k * (i as u64 + 1)).sum();

    Input {
        buggy_rooms: RefCell::new(buggy_rooms),
        fixed_items: RefCell::new(fixed_items),
        shuffle: swap,
        sorts_per_cycle: n,
        expected_desc,
    }
}

// Sort protocol (deterministic full-work sorts, no RNG at run time): every
// sort is preceded by applying the fixed Fisher-Yates swap list (in-place,
// O(N) moves, no allocs) + sorts DESCENDING. The permuted input has ~N/2
// runs → the adaptive driftsort pays the full ~N·log N comparisons for
// EVERY sort, in BOTH arms, every call (a plain reverse would be a single
// detectable run — O(N)). End state: always desc → one G1 expectation.
fn apply_shuffle<T>(v: &mut [T], swap: &[(usize, usize)]) {
    for &(i, j) in swap {
        v.swap(i, j);
    }
}

fn checksum_weighted(ts: &[Option<u64>]) -> u64 {
    ts.iter().enumerate().map(|(i, k)| k.unwrap_or(0) * (i as u64 + 1)).sum()
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut rooms = input.buggy_rooms.borrow_mut();
    for _ in 0..input.sorts_per_cycle {
        apply_shuffle(&mut rooms, &input.shuffle);
        buggy::sort_desc(&mut rooms);
    }
    let ts: Vec<Option<u64>> = rooms.iter().map(|r| r.verify_ts()).collect();
    let cs = checksum_weighted(&ts);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, input.expected_desc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut items = input.fixed_items.borrow_mut();
    // The refresh cycle: N narrow reads (the maintained cache's cost side).
    fixed::refresh_all(&mut items);
    for _ in 0..input.sorts_per_cycle {
        apply_shuffle(&mut items, &input.shuffle);
        fixed::sort_desc(&mut items);
    }
    let ts: Vec<Option<u64>> = items.iter().map(|it| it.cached_ts).collect();
    let cs = checksum_weighted(&ts);
    std::hint::black_box(cs);
    debug_assert_eq!(cs, input.expected_desc);
}
"#
        .to_string(),
    }
}

#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn lock_guard_narrow_field_accessor_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
use std::sync::RwLock;

mod buggy {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        pub timestamp: u64,
        pub sender_is_local: bool,
        pub body: String, // ~64 B heap — the guarded payload
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // The wide accessor: clones the WHOLE guarded payload under the
        // read-guard — 144 B + a heap alloc — so the caller can read one bool.
        pub fn latest_event(&self) -> Option<LatestEvent> {
            self.info.read().unwrap().latest_event.clone()
        }

        pub fn latest_event_is_local(&self) -> bool {
            self.latest_event().is_some_and(|ev| ev.sender_is_local)
        }
    }
}

mod fixed {
    use std::sync::RwLock;

    #[derive(Clone)]
    pub struct LatestEvent {
        // The guarded payload width IS the mechanism — the buggy arm pays
        // to clone it; the narrow accessor reads only the consumed bool.
        #[allow(dead_code)]
        pub timestamp: u64,
        pub sender_is_local: bool,
        #[allow(dead_code)]
        pub body: String,
    }
    pub struct RoomInfo {
        pub latest_event: Option<LatestEvent>,
    }
    pub struct Room {
        pub info: RwLock<RoomInfo>,
    }

    impl Room {
        // The narrow sibling (the production `new_latest_event_is_local`):
        // compute the ONE consumed sub-field under the guard — no clone.
        pub fn latest_event_is_local(&self) -> bool {
            self.info.read().unwrap().latest_event.as_ref().is_some_and(|ev| ev.sender_is_local)
        }
    }
}

pub struct Input {
    buggy_rooms: Vec<buggy::Room>,
    fixed_rooms: Vec<fixed::Room>,
    probes: Vec<usize>,
    expected_local_count: u64,
}

pub fn make_input(n: usize) -> Input {
    const ROOMS: usize = 64; // probe target pool (mix of local/not)
    let make_body = |i: u64| format!("event body {i:04} padding padding padding padding");
    let mut buggy_rooms = Vec::with_capacity(ROOMS);
    let mut fixed_rooms = Vec::with_capacity(ROOMS);
    let mut expected = 0u64;
    for i in 0..ROOMS as u64 {
        let is_local = i % 3 == 0; // deterministic mix
        buggy_rooms.push(buggy::Room {
            info: RwLock::new(buggy::RoomInfo {
                latest_event: Some(buggy::LatestEvent { timestamp: i * 11 + 1, sender_is_local: is_local, body: make_body(i) }),
            }),
        });
        fixed_rooms.push(fixed::Room {
            info: RwLock::new(fixed::RoomInfo {
                latest_event: Some(fixed::LatestEvent { timestamp: i * 11 + 1, sender_is_local: is_local, body: make_body(i) }),
            }),
        });
    }
    // Deterministic probe sequence (LCG) + expected count in ONE pass.
    let mut probes = Vec::with_capacity(n);
    let mut x = 1u64;
    for _ in 0..n {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let idx = ((x >> 33) as usize) % ROOMS;
        if idx as u64 % 3 == 0 { expected += 1; }
        probes.push(idx);
    }
    Input { buggy_rooms, fixed_rooms, probes, expected_local_count: expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut count = 0u64;
    for &p in &input.probes {
        count += input.buggy_rooms[p].latest_event_is_local() as u64;
    }
    std::hint::black_box(count);
    debug_assert_eq!(count, input.expected_local_count);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut count = 0u64;
    for &p in &input.probes {
        count += input.fixed_rooms[p].latest_event_is_local() as u64;
    }
    std::hint::black_box(count);
    debug_assert_eq!(count, input.expected_local_count);
}
"#
        .to_string(),
    }
}

/// Bench spec for `swar-digit-chunk-parse` (Batch 74, Research 088 —
/// the simd-json distill): per-digit decimal parse loop vs the SWAR 8-digit
/// chunk fold (`parse_eight_digits_swar` + `is_made_of_eight_digits_fast`,
/// ORIGINAL minimal reproductions of the source construction).
///
/// sizes = DIGITS PER RUN (the regime axis — the source's own README:
/// "a slight performance hit if most of the floats have less than 8
/// digits"). `N_RUNS` = 2000 fixed; runs are LCG-generated digit strings of
/// exactly n digits (values fit u64 at 19 digits = the axis max), offsets
/// precomputed in `make_input` so the measured delta isolates the parse.
/// G1: both arms wrapping-sum the SAME u64 values — the expectation is
/// accumulated in u128 during generation (== the wrapping u64 sum) and
/// `debug_assert_eq`'d in both arms. At size 4 (< 8) the chunk loop never
/// fires — the honest Tie bound of the regime; 8/12/19 exercise the fold.
/// Alloc axis: 0/0 (both arms parse from the pre-built buffer).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn swar_digit_chunk_parse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4, 8, 12, 19],
        harness: r#"
pub mod buggy {
    pub fn parse_u64(digits: &[u8]) -> u64 {
        let mut val: u64 = 0;
        let mut i = 0;
        while i < digits.len() && digits[i].is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add(u64::from(digits[i] - b'0'));
            i += 1;
        }
        val
    }
}

pub mod fixed {
    pub fn parse_u64(digits: &[u8]) -> u64 {
        let mut val: u64 = 0;
        let mut i = 0;
        while i + 8 <= digits.len() {
            let chunk: [u8; 8] = digits[i..i + 8].try_into().unwrap();
            if !is_made_of_eight_digits_fast(&chunk) {
                break;
            }
            val = val.wrapping_mul(100_000_000).wrapping_add(u64::from(parse_eight_digits_swar(&chunk)));
            i += 8;
        }
        while i < digits.len() && digits[i].is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add(u64::from(digits[i] - b'0'));
            i += 1;
        }
        val
    }

    fn is_made_of_eight_digits_fast(chars: &[u8; 8]) -> bool {
        let val = u64::from_ne_bytes(*chars);
        ((val & 0xF0F0_F0F0_F0F0_F0F0)
            | (((val.wrapping_add(0x0606_0606_0606_0606)) & 0xF0F0_F0F0_F0F0_F0F0) >> 4))
            == 0x3333_3333_3333_3333
    }

    fn parse_eight_digits_swar(chars: &[u8; 8]) -> u32 {
        let val = u64::from_le_bytes(*chars);
        let val = (val & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(2561) >> 8;
        let val = (val & 0x00FF_00FF_00FF_00FF).wrapping_mul(6_553_601) >> 16;
        ((val & 0x0000_FFFF_0000_FFFF).wrapping_mul(42_949_672_960_001) >> 32) as u32
    }
}

const N_RUNS: usize = 2_000;

pub struct Input {
    buf: std::cell::RefCell<Vec<u8>>,
    runs: Vec<(usize, usize)>, // (offset, len) — precomputed so the delta isolates the parse
    expected: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut buf = Vec::with_capacity(N_RUNS * (n + 1));
    let mut runs = Vec::with_capacity(N_RUNS);
    let mut expected: u128 = 0;
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    for _ in 0..N_RUNS {
        let start = buf.len();
        let mut val: u128 = 0; // u128 accumulation == the wrapping u64 sum at cast time
        for _ in 0..n {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            let d = ((rng >> 33) % 10) as u8;
            buf.push(b'0' + d);
            val = val * 10 + u128::from(d);
        }
        buf.push(b','); // run terminator — both arms stop here
        runs.push((start, n));
        expected += val;
    }
    Input { buf: std::cell::RefCell::new(buf), runs, expected: expected as u64 }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let buf = input.buf.borrow();
    let mut sum: u64 = 0;
    for &(s, l) in &input.runs {
        sum = sum.wrapping_add(buggy::parse_u64(&buf[s..s + l]));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let buf = input.buf.borrow();
    let mut sum: u64 = 0;
    for &(s, l) in &input.runs {
        sum = sum.wrapping_add(fixed::parse_u64(&buf[s..s + l]));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}
"#
        .to_string(),
    }
}

/// Bench spec for `runtime-feature-detect-per-chunk` (Batch 74, Research
/// 088): CPU feature detection paid PER CHUNK (the `std::detect` string
/// resolution + cached detector read, modeled via the arch-portable
/// `detected()` helper) vs resolved ONCE into a cached atomic byte.
///
/// sizes = BYTES PER CHUNK (the work-per-detection axis): at small chunks
/// the detection rivals the work itself (the source's finding — their
/// per-8-digit-group check "costs more than the SWAR fallback"); as chunk
/// work grows the fixed ratio decays toward 1. `N_CHUNKS` = 1000 fixed.
/// Both arms' chunk fns are `#[inline(never)]` — models the real-world
/// outlined dispatch (the source's "two outlined `target_feature` calls")
/// AND prevents the compiler from hoisting the detection out of the loop.
/// G1: both arms sum the same bytes — checksum `debug_assert_eq`'d.
/// Alloc axis: 0/0. HONEST SCOPE: on rustc 1.97 the feature-name match on
/// a string literal may const-fold after inlining — the measured delta is
/// whatever remains (detector read + branch + call shape); the annotation
/// records the verdict honestly either way (the B68 varint precedent).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn runtime_feature_detect_per_chunk_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![16, 256, 4096],
        harness: r#"
use std::sync::atomic::{AtomicU8, Ordering};

#[cfg(target_arch = "aarch64")]
pub fn detected() -> bool {
    std::arch::is_aarch64_feature_detected!("neon")
}
#[cfg(target_arch = "x86_64")]
pub fn detected() -> bool {
    std::arch::is_x86_feature_detected!("avx2")
}
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
pub fn detected() -> bool {
    true
}

pub mod buggy {
    use super::detected;

    #[inline(never)] // models the outlined target_feature dispatch + blocks loop hoisting
    pub fn sum_chunk(data: &[u8]) -> u64 {
        // detection paid PER CHUNK — string resolution + detector read every call
        if detected() {
            data.iter().map(|&b| u64::from(b)).sum()
        } else {
            data.iter().fold(0u64, |acc, &b| acc.wrapping_add(u64::from(b)))
        }
    }
}

pub mod fixed {
    use std::sync::atomic::{AtomicU8, Ordering};

    static CACHED: AtomicU8 = AtomicU8::new(0); // 0 = unknown, 1 = yes, 2 = no

    fn detected_once() -> bool {
        match CACHED.load(Ordering::Relaxed) {
            0 => {
                let v = super::detected();
                CACHED.store(if v { 1 } else { 2 }, Ordering::Relaxed);
                v
            }
            1 => true,
            _ => false,
        }
    }

    #[inline(never)] // symmetric with the buggy arm — the delta is detection vs cached byte
    pub fn sum_chunk(data: &[u8]) -> u64 {
        if detected_once() {
            data.iter().map(|&b| u64::from(b)).sum()
        } else {
            data.iter().fold(0u64, |acc, &b| acc.wrapping_add(u64::from(b)))
        }
    }
}

const N_CHUNKS: usize = 1_000;

pub struct Input {
    chunks: Vec<Vec<u8>>,
    expected: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut chunks = Vec::with_capacity(N_CHUNKS);
    let mut expected: u64 = 0;
    let mut rng: u64 = 0xC0FF_EE00_BA5E_BA11;
    for _ in 0..N_CHUNKS {
        let mut c = Vec::with_capacity(n);
        for _ in 0..n {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            c.push((rng >> 56) as u8);
        }
        expected = expected.wrapping_add(c.iter().map(|&b| u64::from(b)).sum::<u64>());
        chunks.push(c);
    }
    Input { chunks, expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut sum: u64 = 0;
    for c in &input.chunks {
        sum = sum.wrapping_add(buggy::sum_chunk(c));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut sum: u64 = 0;
    for c in &input.chunks {
        sum = sum.wrapping_add(fixed::sum_chunk(c));
    }
    std::hint::black_box(sum);
    debug_assert_eq!(sum, input.expected);
}
"#
        .to_string(),
    }
}

/// Bench spec for `encoded-layout-borrowed-view` (Batch 75, Research 089 —
/// the `GreptimeDB` `recordbatches_to_timeseries` distill): dictionary-encoded
/// column (S distinct strings + one u32 key per row) consumed through a
/// generic materializing accessor vs a borrowing enum view with one variant
/// per layout.
///
/// sizes = ROWS R per column; `S_DISTINCT` = 100 distinct dictionary values
/// and `L_COLS` = 4 label columns fixed (the source's Dictionary<`UInt32`, Utf8>
/// regime — a label layout like `host` has few distinct values across many
/// rows). The buggy accessor is the source's `iter_column_as_string` shape:
/// a GENERIC path that does not recognize the dictionary layout and calls
/// `to_string()` row by row. G1: both arms sum the same label lengths — the
/// expectation is computed in `make_input` straight from the keys (no view
/// involved) and `debug_assert_eq`'d in both arms.
/// Alloc axis: buggy = L + R·L per call (one collect Vec per column + one
/// String per cell — "the 3 strings become 1000"); fixed = 0 steady-state
/// (the view borrows; no copy, no allocation).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn encoded_layout_borrowed_view_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 1_000_000],
        harness: r#"
const S_DISTINCT: usize = 100; // distinct dictionary values per column
const L_COLS: usize = 4;       // label columns (host, region, ...)

mod buggy {
    pub struct DictColumn {
        pub values: Vec<String>,
        pub keys: Vec<u32>,
    }

    impl DictColumn {
        fn lookup(&self, row: usize) -> Option<&str> {
            let k = *self.keys.get(row)?;
            self.values.get(k as usize).map(String::as_str)
        }
    }

    // The source's `iter_column_as_string`: a GENERIC accessor that does
    // not recognize the dictionary layout — falls back to owned Strings
    // row by row, so the S distinct strings expand into R per column.
    pub fn column_as_strings(col: &DictColumn) -> Vec<Option<String>> {
        (0..col.keys.len()).map(|r| col.lookup(r).map(|s| s.to_string())).collect()
    }

    pub fn label_len_total(cols: &[DictColumn]) -> usize {
        let mut total = 0;
        for col in cols {
            let cells = column_as_strings(col); // R heap allocs per column
            for cell in &cells {
                total += cell.as_ref().map_or(0, |s| s.len());
            }
        }
        total
    }
}

mod fixed {
    pub struct DictColumn {
        pub values: Vec<String>,
        pub keys: Vec<u32>,
    }

    // One variant per column layout; `value` borrows straight from the
    // array — no copy and no allocation (the source's `LabelValues` enum).
    pub enum ColumnView<'a> {
        Utf8(&'a [String]),
        Dictionary { values: &'a [String], keys: &'a [u32] },
    }

    impl ColumnView<'_> {
        pub fn value(&self, row: usize) -> Option<&str> {
            match self {
                ColumnView::Utf8(v) => v.get(row).map(String::as_str),
                ColumnView::Dictionary { values, keys } => keys
                    .get(row)
                    .and_then(|&k| values.get(k as usize).map(String::as_str)),
            }
        }
    }

    pub fn label_len_total(cols: &[DictColumn]) -> usize {
        let rows = cols.first().map_or(0, |c| c.keys.len());
        let mut total = 0;
        for col in cols {
            let view = ColumnView::Dictionary { values: &col.values, keys: &col.keys };
            for row in 0..rows {
                total += view.value(row).map_or(0, str::len);
            }
        }
        total
    }
}

pub struct Input {
    buggy_cols: Vec<buggy::DictColumn>,
    fixed_cols: Vec<fixed::DictColumn>,
    expected: usize,
}

pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut buggy_cols = Vec::with_capacity(L_COLS);
    let mut fixed_cols = Vec::with_capacity(L_COLS);
    let mut expected = 0usize;
    for _ in 0..L_COLS {
        let values: Vec<String> = (0..S_DISTINCT).map(|i| format!("label-value-{i:03}")).collect();
        let mut keys = Vec::with_capacity(n);
        for _ in 0..n {
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            keys.push(((rng >> 33) % (S_DISTINCT as u64)) as u32);
        }
        // expectation straight from the keys — no view involved
        expected += keys.iter().map(|&k| values[k as usize].len()).sum::<usize>();
        buggy_cols.push(buggy::DictColumn { values: values.clone(), keys: keys.clone() });
        fixed_cols.push(fixed::DictColumn { values, keys });
    }
    Input { buggy_cols, fixed_cols, expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let total = buggy::label_len_total(&input.buggy_cols);
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let total = fixed::label_len_total(&input.fixed_cols);
    std::hint::black_box(total);
    debug_assert_eq!(total, input.expected);
}
"#
        .to_string(),
    }
}

/// Bench spec for `group-keys-borrow-allocate-on-first-sight` (Batch 75,
/// Research 089 — the `GreptimeDB` `recordbatches_to_timeseries` distill):
/// per-row owned-key grouping (clone the label set into a `BTreeMap` key
/// every row) vs hash-the-borrowed-labels + candidate index list, cloning
/// the owned key only when a NEW series is first seen + one final
/// `sort_unstable_by` restoring the `BTreeMap`'s ordered output.
///
/// sizes = ROWS R; `S_GROUPS` = 1000 distinct label sets fixed (the axis is
/// the R/S dedup ratio: R·L clones become S·L). Rows reference an interned
/// label arena by index so INPUT memory stays O(S) strings while the buggy
/// arm still clones 2·L Strings + 1 Vec per row per call; group ids are
/// assigned by a fixed LCG (pseudo-random order — no fast-path locality
/// smuggled in; this rule's win is pure clone-avoidance). G1: both arms
/// emit groups in SORTED key order (the `BTreeMap`'s free order; the fixed
/// arm's final sort restores it) — a position-weighted checksum over the
/// independently-sorted group table is `debug_assert_eq`'d in both arms.
/// Top cell 300K not 1M: the buggy arm at 1M is ~7M allocs + 1M ordered
/// `BTreeMap` string inserts per call — the R/S-ratio story is unchanged and
/// the bench stays minutes-scale (the B73 sorts-per-update sizing lesson).
/// Alloc axis: buggy ≈ 7·R + 2·S per call; fixed ≈ 8·S + ~30 (first-sight
/// key clones + candidate index lists + table/vec growth) — independent
/// of R.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn group_keys_borrow_allocate_on_first_sight_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 300_000],
        harness: r#"
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const S_GROUPS: usize = 1_000; // distinct label sets (series)
const L_LABELS: usize = 3;     // (name, value) pairs per series

// Shared input vocabulary: rows reference the interned arena by index so
// input memory stays O(S) — per-call cloning is the ARMS' business.
pub struct Row {
    pub labels: [(u32, u32); L_LABELS], // (name_idx, value_idx) into the arena
    pub value: u64,
}
pub struct Labels {
    pub names: Vec<String>,
    pub vals: Vec<String>,
}

fn hash_row(row: &Row, arena: &Labels) -> u64 {
    let mut h = DefaultHasher::new();
    for &(n, v) in &row.labels {
        arena.names[n as usize].hash(&mut h);
        arena.vals[v as usize].hash(&mut h);
    }
    h.finish()
}

fn key_matches(key: &[(String, String)], row: &Row, arena: &Labels) -> bool {
    key.len() == row.labels.len()
        && key.iter().zip(&row.labels).all(|((kn, kv), &(n, v))| {
            *kn == arena.names[n as usize] && *kv == arena.vals[v as usize]
        })
}

mod buggy {
    use super::{Labels, Row};
    use std::collections::BTreeMap;

    pub fn group_rows(rows: &[Row], arena: &Labels) -> Vec<(Vec<(String, String)>, (u64, u64))> {
        let mut map: BTreeMap<Vec<(String, String)>, (u64, u64)> = BTreeMap::new();
        for row in rows {
            // 7 heap allocs EVERY row: 1 Vec + 2·L String clones — R·L
            // copies where S·L would do
            let key: Vec<(String, String)> = row
                .labels
                .iter()
                .map(|&(n, v)| (arena.names[n as usize].clone(), arena.vals[v as usize].clone()))
                .collect();
            let e = map.entry(key).or_insert((0, 0));
            e.0 += 1;
            e.1 += row.value;
        }
        map.into_iter().collect() // BTreeMap order = sorted keys, for free
    }
}

mod fixed {
    use super::{hash_row, key_matches, Labels, Row};
    use std::collections::HashMap;

    pub struct Group {
        pub key: Vec<(String, String)>,
        pub count: u64,
        pub sum: u64,
    }

    pub fn group_rows(rows: &[Row], arena: &Labels) -> Vec<Group> {
        // hash → candidate index list; FULL label comparison on every
        // candidate — collisions can never merge two different series
        let mut index: HashMap<u64, Vec<usize>> = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        for row in rows {
            let hash = hash_row(row, arena); // borrowed labels — no alloc
            let idx = match index.get(&hash).and_then(|cands| {
                cands.iter().find(|&&i| key_matches(&groups[i].key, row, arena))
            }) {
                Some(&i) => i,
                None => {
                    let i = groups.len();
                    groups.push(Group {
                        key: row
                            .labels
                            .iter()
                            .map(|&(n, v)| {
                                (arena.names[n as usize].clone(), arena.vals[v as usize].clone())
                            })
                            .collect(), // allocated ON FIRST SIGHT only — S·L, not R·L
                        count: 0,
                        sum: 0,
                    });
                    index.entry(hash).or_default().push(i);
                    i
                }
            };
            groups[idx].count += 1;
            groups[idx].sum += row.value;
        }
        // one S·log(S) sort ≪ R ordered BTreeMap inserts — restores the
        // BTreeMap's sorted output contract
        groups.sort_unstable_by(|a, b| a.key.cmp(&b.key));
        groups
    }
}

// position-weighted checksum over the sorted group table — the SAME fold
// in make_input (ground truth) and both arms
fn fold_group(chk: u64, key_len: usize, count: u64, sum: u64) -> u64 {
    chk.wrapping_mul(31)
        .wrapping_add(key_len as u64)
        .wrapping_mul(31)
        .wrapping_add(count)
        .wrapping_mul(31)
        .wrapping_add(sum)
}

pub struct Input {
    rows: Vec<Row>,
    arena: Labels,
    expected_groups: usize,
    expected_chk: u64,
}

pub fn make_input(n: usize) -> Input {
    // arena: S groups × L (name, value) pairs, all distinct
    let mut names = Vec::with_capacity(S_GROUPS * L_LABELS);
    let mut vals = Vec::with_capacity(S_GROUPS * L_LABELS);
    for g in 0..S_GROUPS {
        for l in 0..L_LABELS {
            names.push(format!("n{g:04}-{l}"));
            vals.push(format!("v{g:04}-{l}"));
        }
    }
    let arena = Labels { names, vals };

    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    let mut rows = Vec::with_capacity(n);
    let mut counts = vec![0u64; S_GROUPS];
    let mut sums = vec![0u64; S_GROUPS];
    for _ in 0..n {
        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let g = ((rng >> 33) % (S_GROUPS as u64)) as usize;
        let base = g * L_LABELS;
        let labels = [
            (base as u32, base as u32),
            (base as u32 + 1, base as u32 + 1),
            (base as u32 + 2, base as u32 + 2),
        ];
        rows.push(Row { labels, value: rng >> 16 });
        counts[g] += 1;
        sums[g] = sums[g].wrapping_add(rng >> 16);
    }

    // ground truth: the sorted-key group table, folded position-weighted
    let mut table: Vec<(Vec<(String, String)>, u64, u64)> = (0..S_GROUPS)
        .filter(|&g| counts[g] > 0)
        .map(|g| {
            let base = g * L_LABELS;
            ((0..L_LABELS)
                .map(|l| (arena.names[base + l].clone(), arena.vals[base + l].clone()))
                .collect::<Vec<_>>(),
            counts[g],
            sums[g])
        })
        .collect();
    table.sort_by(|a, b| a.0.cmp(&b.0));
    let expected_groups = table.len();
    let mut expected_chk: u64 = 0;
    for (key, count, sum) in &table {
        let key_len = key.iter().map(|(n, v)| n.len() + v.len()).sum::<usize>();
        expected_chk = fold_group(expected_chk, key_len, *count, *sum);
    }
    Input { rows, arena, expected_groups, expected_chk }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = buggy::group_rows(&input.rows, &input.arena);
    let mut chk: u64 = 0;
    for (key, (count, sum)) in &groups {
        let key_len = key.iter().map(|(n, v)| n.len() + v.len()).sum::<usize>();
        chk = fold_group(chk, key_len, *count, *sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = fixed::group_rows(&input.rows, &input.arena);
    let mut chk: u64 = 0;
    for g in &groups {
        let key_len = g.key.iter().map(|(n, v)| n.len() + v.len()).sum::<usize>();
        chk = fold_group(chk, key_len, g.count, g.sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}
"#
        .to_string(),
    }
}

/// Bench spec for `sorted-input-previous-row-fast-path` (Batch 75,
/// Research 089 — the `GreptimeDB` `recordbatches_to_timeseries` distill):
/// hash-every-row grouping vs compare-against-the-previous-row's-group
/// first, hash fallback on miss.
///
/// sizes = RUN LENGTH (the hit-rate regime axis, the `sort_key_cache`
/// semantic-sizes precedent): n = 100 → adjacent input (S=1000 contiguous
/// runs of 100 — the real query shape: rows sorted by primary key so one
/// series' samples arrive together); n = 1 → interleaved (round-robin over
/// all 1000 series — the honest bound: every row pays one failed [u64; 2]
/// comparison before the hash). `R_ROWS` = `100_000` fixed; `S_SERIES` = 1000.
/// G1: both arms emit groups in FIRST-SIGHT order — identical in both arms
/// and computable in `make_input` (series 0..S ascending for BOTH layouts) —
/// and fold the same position-weighted (series, count, sum) checksum,
/// `debug_assert_eq`'d against the `make_input` ground truth. Correctness
/// never depends on the fast path: the hash fallback with full-key
/// equality is the semantic contract.
/// Alloc axis: ~equal by design (both build the same `HashMap` + groups
/// Vec; neither allocates per row) — the TIMING axis is the mechanism
/// here, and the alloc integers pin that no per-row allocation snuck in.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn sorted_input_previous_row_fast_path_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 100],
        harness: r#"
use std::collections::HashMap;

const R_ROWS: usize = 100_000;
const S_SERIES: usize = 1_000;

pub struct Row {
    pub series: [u64; 2], // the label-set key (hashed by the map)
    pub value: u64,
}
pub struct Group {
    pub series: [u64; 2],
    pub count: u64,
    pub sum: u64,
}

mod buggy {
    use super::{Group, Row};
    use std::collections::HashMap;

    pub fn group_rows(rows: &[Row]) -> Vec<Group> {
        let mut index: HashMap<[u64; 2], usize> = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        for row in rows {
            // hash EVERY row — even when it lands on the same series as
            // the previous row (the common case for sorted input)
            let idx = match index.get(&row.series) {
                Some(&i) => i,
                None => {
                    let i = groups.len();
                    groups.push(Group { series: row.series, count: 0, sum: 0 });
                    index.insert(row.series, i);
                    i
                }
            };
            groups[idx].count += 1;
            groups[idx].sum += row.value;
        }
        groups
    }
}

mod fixed {
    use super::{Group, Row};
    use std::collections::HashMap;

    pub fn group_rows(rows: &[Row]) -> Vec<Group> {
        let mut index: HashMap<[u64; 2], usize> = HashMap::new();
        let mut groups: Vec<Group> = Vec::new();
        let mut prev: Option<usize> = None;
        for row in rows {
            // compare against the series the PREVIOUS row landed on —
            // no hash on hit. The fast path only exploits local ordering;
            // correctness never depends on it.
            let idx = match prev {
                Some(i) if groups[i].series == row.series => i,
                _ => match index.get(&row.series) {
                    Some(&i) => i,
                    None => {
                        let i = groups.len();
                        groups.push(Group { series: row.series, count: 0, sum: 0 });
                        index.insert(row.series, i);
                        i
                    }
                },
            };
            prev = Some(idx);
            groups[idx].count += 1;
            groups[idx].sum += row.value;
        }
        groups
    }
}

pub struct Input {
    rows: Vec<Row>,
    expected_groups: usize,
    expected_chk: u64,
}

fn fold_group(chk: u64, series: [u64; 2], count: u64, sum: u64) -> u64 {
    chk.wrapping_mul(31)
        .wrapping_add(series[0] ^ series[1].rotate_left(17))
        .wrapping_mul(31)
        .wrapping_add(count)
        .wrapping_mul(31)
        .wrapping_add(sum)
}

fn series_of(g: usize) -> [u64; 2] {
    [g as u64, (g as u64).wrapping_mul(2_654_435_761).wrapping_add(0x9E37_79B9)]
}

pub fn make_input(n: usize) -> Input {
    let run_len = n.max(1); // n = run length: 1 = interleaved (round-robin), 100 = adjacent
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    let mut rows = Vec::with_capacity(R_ROWS);
    // emit run_len consecutive rows of one series, then the next — for
    // run_len = 1 this degenerates to round-robin over all series
    let mut g = 0usize;
    while rows.len() < R_ROWS {
        let series = series_of(g % S_SERIES);
        for _ in 0..run_len {
            if rows.len() == R_ROWS {
                break;
            }
            rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            rows.push(Row { series, value: rng >> 16 });
        }
        g += 1;
    }
    // ground truth: first-sight order is 0..S for BOTH layouts (series 0
    // is the first run / first round-robin slot either way)
    let mut counts = vec![0u64; S_SERIES];
    let mut sums = vec![0u64; S_SERIES];
    let mut present = vec![false; S_SERIES];
    for row in &rows {
        let g = row.series[0] as usize;
        counts[g] += 1;
        sums[g] = sums[g].wrapping_add(row.value);
        present[g] = true;
    }
    let expected_groups = present.iter().filter(|&&p| p).count();
    let mut expected_chk: u64 = 0;
    for g in 0..S_SERIES {
        if present[g] {
            expected_chk = fold_group(expected_chk, series_of(g), counts[g], sums[g]);
        }
    }
    Input { rows, expected_groups, expected_chk }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = buggy::group_rows(&input.rows);
    let mut chk: u64 = 0;
    for g in &groups {
        chk = fold_group(chk, g.series, g.count, g.sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let groups = fixed::group_rows(&input.rows);
    let mut chk: u64 = 0;
    for g in &groups {
        chk = fold_group(chk, g.series, g.count, g.sum);
    }
    std::hint::black_box(chk);
    debug_assert_eq!(groups.len(), input.expected_groups);
    debug_assert_eq!(chk, input.expected_chk);
}
"#
        .to_string(),
    }
}

/// Bench spec for `cache-line-block-partitioned-lock` (Batch 78,
/// Research 092 — Datadog RTDB/Monocle): one `Mutex<Vec<u64>>` guarding
/// all 4096 points vs 512 aligned `Mutex<[u64; 8]>` blocks. Thread
/// counts are the size axis (the align-64 convention). G1: bump totals
/// are the closed form `threads * iters` — lock granularity cannot
/// change totals (Mutex guarantees).
#[cfg(feature = "rust_perf_bench")]
fn cache_line_block_partitioned_lock_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![2, 4, 8, 12],
        harness: "\nmod buggy {\n    use std::sync::Mutex;\n    // One lock guards the whole point array: every bump serializes.\n    pub struct Series { pub points: Mutex<Vec<u64>> }\n    pub fn make(n_points: usize) -> Series {\n        Series { points: Mutex::new(vec![0u64; n_points]) }\n    }\n    pub fn run(s: &Series, n_threads: usize, iters: u64, n_points: usize) -> u64 {\n        std::thread::scope(|sc| {\n            for t in 0..n_threads {\n                sc.spawn(move || {\n                    let mut rng: u64 = (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);\n                    for _ in 0..iters {\n                        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);\n                        let i = (rng >> 33) as usize % n_points;\n                        s.points.lock().unwrap()[i] += 1; // full serialization\n                    }\n                });\n            }\n        });\n        let g = s.points.lock().unwrap();\n        g.iter().sum()\n    }\n}\n\nmod fixed {\n    use std::sync::Mutex;\n    // 8 points per cache-line-aligned block, one lock per block.\n    #[repr(align(128))] // the ARCH coherence line (64 x86 / 128 Apple Silicon)\n    pub struct Block { pub lock: Mutex<[u64; 8]> }\n    pub struct Series { pub blocks: Vec<Block> }\n    pub fn make(n_points: usize) -> Series {\n        let n_blocks = n_points.div_ceil(8);\n        Series { blocks: (0..n_blocks).map(|_| Block { lock: Mutex::new([0u64; 8]) }).collect() }\n    }\n    pub fn run(s: &Series, n_threads: usize, iters: u64, n_points: usize) -> u64 {\n        std::thread::scope(|sc| {\n            for t in 0..n_threads {\n                sc.spawn(move || {\n                    let mut rng: u64 = (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);\n                    for _ in 0..iters {\n                        rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);\n                        let i = (rng >> 33) as usize % n_points;\n                        let b = &s.blocks[i / 8];\n                        b.lock.lock().unwrap()[i % 8] += 1; // contention ~ /8\n                    }\n                });\n            }\n        });\n        s.blocks.iter().map(|b| b.lock.lock().unwrap().iter().sum::<u64>()).sum()\n    }\n}\n\nconst POINTS: usize = 4096; // 512 blocks x 8 (fixed arm)\nconst ITERS: u64 = 20_000;  // bumps per thread\n\npub struct Input { n_threads: usize }\n\npub fn make_input(n: usize) -> Input {\n    // n = THREAD COUNT (the contention axis is the size axis, the align-64\n    // bench convention).\n    Input { n_threads: n }\n}\n\nfn expected(n_threads: usize) -> u64 {\n    // Every bump adds exactly 1 — the closed-form total (lock granularity\n    // cannot change it; Mutex guarantees mutual exclusion, G1).\n    n_threads as u64 * ITERS\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let s = buggy::make(POINTS);\n    let cs = buggy::run(&s, input.n_threads, ITERS, POINTS);\n    std::hint::black_box(cs);\n    debug_assert_eq!(cs, expected(input.n_threads));\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let s = fixed::make(POINTS);\n    let cs = fixed::run(&s, input.n_threads, ITERS, POINTS);\n    std::hint::black_box(cs);\n    debug_assert_eq!(cs, expected(input.n_threads));\n}\n".to_string(),
    }
}
