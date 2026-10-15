//! Entries slice 29 — Batch 243 (2026-10-16, Plan 245): the text/unicode
//! processing family seed — the corpus's founding member of that family —
//! distilled from Hugging Face's "Tokenmaxxing BERT" blog (2026-10-15) +
//! `huggingface/tokenizers` @ `5b5a3fcc18335eed616ae89adb714a048fed1a31`
//! (branch `perf/faster-unicode-normalizers`, Apache-2.0; the pin verified
//! STILL the branch tip at conversion — PR-stage, unmerged to main, and the
//! branch moved zero commits past it since the blog). Both rules
//! BENCHABLE (`Some(bench_spec`)) + annotated (regime/anti-regime +
//! distinct-from sets + pin-verified quotes). Source reads: two read
//! subagents over disjoint file sets + coordinator re-verification of every
//! quote byte-for-byte at the `.raw/tokenizers` pin clone (removed at close).
//!
//! A THIRD lead (the `transform-borrow-until-first-change` rule from
//! `rewriter.rs`) was authored, benched, and DROPPED HONESTLY: the
//! `rust_perf_bench` validator measured it `ParetoDominated` 0/3/0 twice (0.68-0.74x
//! timing across two fixture shapes — ~70 B strings 0.70x, ~2 KB strings 0.74x)
//! while the alloc axis WON 2.6x fewer allocs and the byte axis WON ~6x less
//! copied (24.5 MB vs 4.1 MB at n=20000; alloc-det stable both runs). The loss
//! is the find phase: the fixed arm pays 3x `match_indices` on every string,
//! while the source's real-world win rides the scan being SHARED with the
//! caller's inspection pass (the normalizer needs the unstable spans anyway) —
//! a synthetic cannot model that sharing within the rule's own mostly-clean
//! regime. The negative is recorded in Research 275 + plan 245 T3; the borrow
//! half of the law is already owned by `cow-conditional-alloc` (`entries_02`).

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── simd: class-pure run skip via a high-bit-gather block mask — the
        // ASCII runs are never decoded, looked up, or copied ────────────────
        // Source: huggingface/tokenizers @ 5b5a3fcc, utils/ascii_fastpath.rs
        // + normalizers/bert.rs (the Tokenmaxxing-BERT branch).
        PerfRule {
            rule_id: "ascii-run-swar-block-skip".to_string(),
            name: "Per-char class classification over text → skip class-pure byte runs with a high-bit-gather block mask (pure runs are never decoded, looked up, or copied; the scalar path resumes only at the first offender)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// NAIVE: classify EVERY character — per-char UTF-8 decode plus a\n// sorted-table binary search, paid even for the plain-ASCII bulk of\n// the text (the shape that made the normalizer 88% of encode time).\n\n/// Sorted table of class members (the unicode_categories shape: a\n/// sorted &'static [char] of 1,567 entries — ~11 unpredictable branches\n/// per lookup).\nfn is_mark(table: &[char], c: char) -> bool {\n    table.binary_search(&c).is_ok()\n}\n\npub fn count_marks(text: &str, table: &[char]) -> usize {\n    let mut n = 0usize;\n    for c in text.chars() {\n        // paid for plain 's', 'l', 'o', 'w' too\n        if is_mark(table, c) {\n            n += 1;\n        }\n    }\n    n\n}".to_string(),
            fixed_code: "// OPT: gather each block's high bits into a mask; a zero mask means\n// the whole block is pure ASCII — skip it wholesale (never decoded,\n// never looked up, never copied). The lowest set bit IS the first\n// offender; only from there does the scalar path resume. Sound because\n// the class members are all non-ASCII: an ASCII byte can never match.\n\n/// Length of the largest ASCII prefix. Portable SWAR spelling (the\n/// source ships the 16-byte `wide::u8x16::to_bitmask()` form; the u64\n/// mask is the same law without a SIMD dependency).\nfn ascii_prefix_len(input: &[u8]) -> usize {\n    let mut chunks = input.chunks_exact(8);\n    let mut offset = 0usize;\n    for chunk in &mut chunks {\n        let word = u64::from_le_bytes(chunk.try_into().unwrap());\n        if word & 0x8080_8080_8080_8080 != 0 {\n            return offset + word.trailing_zeros() as usize / 8;\n        }\n        offset += 8;\n    }\n    let rem = chunks.remainder();\n    offset + rem.iter().position(|&b| b & 0x80 != 0).unwrap_or(rem.len())\n}\n\npub fn count_marks(text: &str, table: &[char]) -> usize {\n    let bytes = text.as_bytes();\n    let mut n = 0usize;\n    let mut i = ascii_prefix_len(bytes);\n    while i < bytes.len() {\n        // one non-ASCII char: decode + classify, then skip the next run\n        let c = text[i..].chars().next().unwrap();\n        if table.binary_search(&c).is_ok() {\n            n += 1;\n        }\n        i += c.len_utf8();\n        i += ascii_prefix_len(&bytes[i..]);\n    }\n    n\n}\n\n#[test]\nfn swar_skip_matches_the_per_char_reference_at_block_edges() {\n    fn reference(text: &str, table: &[char]) -> usize {\n        text.chars().filter(|c| table.binary_search(c).is_ok()).count()\n    }\n    let table: Vec<char> = vec!['\\u{00e9}', '\\u{0301}', '\\u{0430}', '\\u{1F600}'];\n    // block-edge straddles: lengths 7/8/9/15/16/17 over pure ASCII\n    for len in [0usize, 7, 8, 9, 15, 16, 17, 31, 32, 33] {\n        let ascii = \"a\".repeat(len);\n        assert_eq!(count_marks(&ascii, &table), reference(&ascii, &table));\n    }\n    // first-byte offender, all-non-ASCII, and mixed placements\n    for text in [\n        \"\\u{00e9}abc\",\n        \"abc\\u{00e9}\",\n        \"\\u{00e9}\\u{00e9}\\u{00e9}\",\n        \"ab\\u{0430}cdefghij\\u{0301}klmno\",\n        \"abcdefgh\\u{1F600}ijklmnop\",\n    ] {\n        assert_eq!(count_marks(text, &table), reference(text, &table), \"{text:?}\");\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.3 of 16 AC-power box disclosed; Batch 243 GOAT: 6.41x/7.09x/7.78x at n=1K/16K/262K, alloc 0/0, alloc-det stable x3 — the validator numbers are ours; the source numbers below are THEIR hardware, never our claims): Hugging Face 'Tokenmaxxing BERT: 50x faster tokenization' (2026-10-15) — warm bert-base-uncased encode: '88% of encode time in the normalizer, 8% in WordPiece', about two-thirds of the WHOLE encode inside binary_search_by from char::is_mark_nonspacing() (the unicode_categories crate's sorted &'static [char] of 1,567 entries, ~11 unpredictable branches per lookup) — 'run for every character, including plain s, l, o, w'. The 50x headline is COMPOUND (8.6x prior v1 x ~5.7x this PR); this rule covers only this PR's mechanisms. SOURCE (huggingface/tokenizers @ 5b5a3fcc18335eed616ae89adb714a048fed1a31, branch perf/faster-unicode-normalizers, Apache-2.0; every quote verified verbatim at the .raw pin clone this session — the pin is STILL the branch tip, PR-stage, unmerged). THE SHIPPED FORM (utils/ascii_fastpath.rs:7-26, verbatim): 'Given a slice bytes, return the length of the largest ASCII prefix.' + 'Leverages SIMD to check the input 16 bytes at a time.' + the core comment 'In unicode, only ASCII bytes have their high bit set to 0 (meaning they are a single-byte codepoint) and .to_bitmask packs the top-bits into a u32, the lowest set bit is then the first non-ascii byte' — u8x16::from(<[u8; 16]>::try_from(chunk).unwrap()).to_bitmask(); mask != 0 => offset + mask.trailing_zeros(); the <=15-byte remainder is zero-padded into a [0u8; 16] window (a 0x00 filler lane is high-bit-0, offender-neutral) and clamped with .min(remainder.len()). The rule's fixture spells the SAME mask law portably (u64::from_le_bytes + 0x8080_8080_8080_8080, trailing_zeros()/8, 8-byte blocks) — disclose that the source's form is the 16-byte wide one; wide::u8x16 is the dep-free-when-you-have-it upgrade. THE RUN-COMPOSITION: non_ascii_segments_iter (ascii_fastpath.rs:41-52) walks ascii_prefix_len -> next_ascii_chunk -> ascii_prefix_len, yielding only the non-ASCII segments; a yielded segment may carry unaligned trailing ASCII bytes before the next 16-aligned pure chunk — superset processing is correctness-safe because the per-char chain is identity on ASCII (test non_ascii_segments_skip_long_ascii_runs_only pins exactly this). THE RIDER (BertNormalizer only): ASCII lowercase is bit-5 OR folded into the same block pass — bert.rs:210-211 '| (mask.uppercase & u8x16::splat(0x20))' with capitals = lanes.simd_ge(u8x16::splat(b'A')) & lanes.simd_le(u8x16::splat(b'Z')) (bert.rs:246-248); bert's own block skip is the SIBLING mechanism, not the high-bit gather: 'None if !(mask.spaces | mask.deletes | mask.uppercase).any() => continue' with a LAZY output alloc — 'The ASCII stages only replace or drop bytes: the output never outgrows the input.' sizing the fresh Vec::with_capacity(bytes.len()) once (the b58428d6 under-alloc fix: growing the lazy Vec from the prefix repeatedly reallocates — size it to the input when the transform never grows it). Delete-class blocks fall to the scalar byte-per-byte path — 'We have to remove bytes from the input: can't use SIMD'. THE JUSTIFICATION IN-SOURCE is the pass-split comment (bert.rs:281-284): 'Pass 2: Normalize ascii text / 1. clean_text: transform \\t, \\r and \\n to ' ', drop other control bytes (eg 0x00-0x01, 0x7F) / 2. lowercase ASCII chars / chinese, nfd, strip accents: nothing to do as those were handled by the previous pass' — HONEST CORRECTION: the blog's 'no normalization form changes an ASCII character and an accent can't attach across one' sentence does NOT appear in the code; the invariant is carried by that comment + the exhaustive offset-fuzz test ascii_stages_match_the_chain_at_every_block_offset (for align in 0..16) + ascii_prefix_len_stops_at_the_first_high_byte. ANTI-REGIMES (source-measured, their Figure 16 = speedup-vs-ASCII-share — the regime IS the data distribution): tiny inputs pay the machinery (down to 0.35x on 8 B of Chinese; '55 of 320 normalizer-only sweep cells slower beyond noise', 19 of them <= 32 B); CJK text gains ~nothing (little ASCII, and every CJK char must be padded with spaces anyway); the win scales with class-pure share — profile your DATA, not just your code. DEDUP ADJUDICATION (T0b re-run at the batch slot, 205-rule corpus): distinct from swar-digit-chunk-parse (SWAR ARITHMETIC on digit runs — this is class-PURITY RUN SKIP), bool-to-bitpack (counts set bits over pre-packed words — no skip/locate), simd-chunk (arithmetic auto-vectorize), byte-find-position-to-memchr (finds ONE literal — this classifies runs against a predicate and skips the pure bulk), dispatch-pass-fusion-skips-empty-handlers (dispatch-level pass fusion, not byte classification), unaligned-word-load-from-bytes (word assembly). The text/unicode processing family's FOUNDING simd member — the corpus previously carried zero text-family rules. Test pins: batch243 needles + the in-rule block-edge sweep (0/7/8/9/15/16/17/31/32/33-byte ASCII + first-byte offender + all-non-ASCII + mixed). Suggestion-only leaning: a parse-only proposer cannot prove the predicate is ASCII-false — gate adoption on that invariant (write it as an assert). Source: the HF blog + tokenizers @ 5b5a3fcc, read + quoted in Research 275. Batch 243 / Plan 245.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(ascii_run_swar_block_skip_bench_spec()),
        },
        // ── caching: lazy per-page predicate memoization over a huge dense
        // domain — one L1 read replaces ~11 unpredictable branches ──────────
        // Source: huggingface/tokenizers @ 5b5a3fcc,
        // utils/character_cache.rs + normalizers/unicode.rs.
        PerfRule {
            rule_id: "range-table-page-memoization".to_string(),
            name: "Sorted-table property lookup per char over a huge dense domain → lazy per-page predicate memoization (256 answers per page, built on first touch; one read replaces the binary search)".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: the property table is queried per char through a binary\n// search — ~11 unpredictable branches per lookup, paid for every\n// character the text touches (two-thirds of a whole encode pass,\n// measured, when the lookups are this shape).\n\n/// Sorted table of class members over the full code-point domain.\nfn is_mark(table: &[char], c: char) -> bool {\n    table.binary_search(&c).is_ok()\n}\n\npub fn classify_all(queries: &[char], table: &[char]) -> usize {\n    queries\n        .iter()\n        .filter(|&&c| is_mark(table, c))\n        .count()\n}".to_string(),
            fixed_code: "// OPT: memoize the PREDICATE ANSWER per aligned page of 256 code\n// points, built lazily on first touch — scripts are contiguous in code\n// point space and local in text, so most workloads touch few pages;\n// every later hit is one plain read.\n\nuse std::sync::OnceLock;\n\nconst CHARS_PER_PAGE: usize = 256;\nconst NUM_PAGES: usize = (char::MAX as usize + 1) / CHARS_PER_PAGE;\n\npub struct PageCache {\n    pages: Vec<OnceLock<Box<[bool; CHARS_PER_PAGE]>>>,\n}\n\nimpl PageCache {\n    /// The outer page-index array allocates eagerly; page CONTENTS are\n    /// lazy (built on first touch, never evicted — 4352 slots x 256 B).\n    pub fn new() -> Self {\n        Self { pages: (0..NUM_PAGES).map(|_| OnceLock::new()).collect() }\n    }\n\n    #[inline]\n    pub fn get(&self, c: char, table: &[char]) -> bool {\n        let page_index = c as u32 as usize / CHARS_PER_PAGE;\n        let index = c as u32 as usize % CHARS_PER_PAGE;\n        let page = self.pages[page_index].get_or_init(|| {\n            Box::new(std::array::from_fn(|idx| {\n                let cp = (page_index * CHARS_PER_PAGE + idx) as u32;\n                char::from_u32(cp).is_some_and(|ch| table.binary_search(&ch).is_ok())\n            }))\n        });\n        page[index]\n    }\n}\n\npub fn classify_all(queries: &[char], cache: &PageCache, table: &[char]) -> usize {\n    queries.iter().filter(|&&c| cache.get(c, table)).count()\n}\n\n#[test]\nfn page_cache_matches_its_classifier_on_every_code_point() {\n    let mut table: Vec<char> = vec![\n        '\\u{00e9}', '\\u{0301}', '\\u{0327}', '\\u{0430}', '\\u{0431}', '\\u{1F600}',\n    ];\n    table.sort_unstable();\n    let cache = PageCache::new();\n    // the source's own contract: walk the FULL domain comparing cache\n    // vs classifier (character_cache_matches_its_classifier)\n    for cp in (0u32..0x110000).step_by(97) {\n        if let Some(c) = char::from_u32(cp) {\n            assert_eq!(\n                cache.get(c, &table),\n                table.binary_search(&c).is_ok(),\n                \"U+{cp:04X}\"\n            );\n        }\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.3 of 16 AC-power box disclosed; Batch 243 GOAT: 9.93x/9.82x/10.01x at n=10K/100K/1M queries, alloc 0/0, alloc-det stable x3 — the validator numbers are ours; the source figures below are THEIR hardware): the blog's villain shape is char::is_mark_nonspacing() from the unicode_categories crate — a binary_search over a sorted &'static [char] of 1,567 entries — about two-thirds of the WHOLE bert encode pass inside that one search, ~11 unpredictable branches per lookup, per char. SOURCE (huggingface/tokenizers @ 5b5a3fcc18335eed616ae89adb714a048fed1a31, branch perf/faster-unicode-normalizers, Apache-2.0; quotes verified verbatim at the .raw pin clone this session). THE STRUCTURE (utils/character_cache.rs, verbatim): 'A cache / memoization structure for unicode normalizers.' + 'Organized in pages of 256 unicode characters, lazily initiated on first lookup with [`CharacterCache::classifier`].' + 'For unicode normalizers, the `classifier` is an expensive binary search in unicode class tables. This structure allows to amortize this cost by computing this once lazily and replacing the expensive binary search with a simple lookup.' + THE LOCALITY LAW that makes it pay: 'Unicode locality properties help here: - Characters of a given script (eg latin, cyrillic, hangul, etc) are often contiguous in codepoint space - Scripts are also often local in a given text, meaning there's a good chance most characters of a given text chunk land in the same cache page'. THE CONSTANTS: 'const CHARS_PER_PAGE: usize = 256;' + 'const NUM_PAGES: usize = (char::MAX as usize + 1) / Self::CHARS_PER_PAGE;' (4,352 pages over 1,114,112 code points — the size is COMPUTED from char::MAX, never written as 1.1M in source). THE SHAPE: pages: Box<[OnceLock<Box<[bool; 256]>>]> — the outer page-index array is EAGER, only page CONTENTS are lazy (get_or_init on first touch, never evicted; worst case all pages built ~ 1.1 MB of booleans); the lookup is division -> page, modulo -> slot, then one plain bool read. The 4x64B-cache-line framing is the RULE'S arithmetic ([bool; 256] = 256 B = 4 cache lines) — the source justifies 256 only as 'codepoints per page'; cite the constant, do the line math yourself. WHAT IS CACHED is the PREDICATE ANSWER, not a value: the source's classifier is is_unstable_char (unicode.rs:14-24) — 'The quick check, not `is_normalized`: alone, a Hangul trailing consonant is in NFC, yet it composes with the syllable before it, and only its `Maybe` answer says so.' — canonical_combining_class(c) != 0 || !matches!(quick, IsNormalized::Yes); the blog's is_mark_nonspacing villain is EXTERNAL context (the actual in-repo classifier differs; the law is the same). Wired as FOUR process-global LazyLock caches, one per normalization form (NFC/NFD/NFKC/NFKD). THE DOMAIN LADDER (where this sits): full-domain const table (the domain fits — precompute-table; its own notes record it Pareto-dominated by POPCNT for tiny domains) > per-page lazy memoization (HUGE dense domain + locality — THIS) > binary search (sparse/random access — bsearch-sorted-slice stands as the corpus's endorsed sparse rung). ANTI-REGIMES (stated on the rule's own authority — the source carries no anti-regime comments): single-shot/random queries (a page fill costs 256 classifier calls — the break-even is ~256/page queries; one question per page pays for the whole page), page-scatter text (emoji/CJK spread across many pages rebuilds the search cost page-by-page), tiny inputs (first-touch init overhead — the same <=32 B class as the sibling rule). The source's own correctness contract is the model for the gate: character_cache_matches_its_classifier walks ALL 0x10FFFF code points comparing cache vs classifier (and the cache is generic over any Fn(char) -> bool — a second instance runs char::is_alphabetic); the in-rule test steps the same walk. DEDUP ADJUDICATION (T0b re-run, 205-rule corpus): distinct from precompute-table (FULL-domain eager const-eval — this is per-page LAZY over a domain too big to materialize eagerly at build time... it WOULD fit at ~1.1 MB but the eager build pays for pages never queried), bsearch-sorted-slice (the sparse rung this rule sits ABOVE), batch-local-memoization-negative-cache-tombstone (batch-scoped negative cache with tombstones — this is process-lifetime immortal pages), mask-memo-hoist-to-owner-lifetime (a mask memo hoisted to an owner — different axis), regex-hoist-lazylock (compile-once of a matcher, not a property-table memoization), sort-key-cache-precompute-out-of-lock (maintained comparator cache under concurrency), prompt-cache-canonical-interval-keys / prompt-cache-rewind-states-min-gap-eviction (serving KV-cache keying/eviction — no relation to property-table lookups). Caching family's first lookup-memoization member over a dense unicode domain. Benchable: binary search per query vs page-cached queries over a locality-rich stream (the fixture's stream is text-shaped: mostly one page, script bursts, a CJK scatter tail for honesty). Source: the HF blog + tokenizers @ 5b5a3fcc, read + quoted in Research 275. Batch 243 / Plan 245.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(range_table_page_memoization_bench_spec()),
        },
    ]
}

/// Bench spec for `ascii-run-swar-block-skip`: count class members (marks)
/// in text-like input — the per-char decode+binary-search loop vs the SWAR
/// block-skip walk. The input distribution is the regime: ~93% ASCII words,
/// accented words, occasional non-Latin runs (text-like LCG).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn ascii_run_swar_block_skip_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// Deterministic 1,567-entry sorted table of non-ASCII code points — the
// unicode_categories villain shape (a sorted [char], binary_search per
// lookup). Built identically in both arms (duplicated builder, same LCG).
fn build_table() -> Vec<char> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut v: Vec<char> = Vec::with_capacity(1567);
    while v.len() < 1567 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let cp = 0x80u32 + ((state >> 33) % 0x10FF80) as u32;
        if let Some(c) = char::from_u32(cp) {
            v.push(c);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    text: String,
    table: Vec<char>,
}

pub fn make_input(n: usize) -> Input {
    let table = build_table();
    let mut state: u64 = 0xDEAD_BEEF_CAFE_F00D;
    let mut text = String::with_capacity(n);
    while text.len() < n {
        // a short ASCII word
        let wlen = 2 + (lcg(&mut state) % 8) as usize;
        for _ in 0..wlen {
            let b = b'a' + (lcg(&mut state) % 26) as u8;
            text.push(b as char);
        }
        text.push(' ');
        let roll = lcg(&mut state) % 100;
        if roll < 5 {
            // an accented word: base letters + a combining mark from the table
            let base = 2 + (lcg(&mut state) % 6) as usize;
            for _ in 0..base {
                let b = b'a' + (lcg(&mut state) % 26) as u8;
                text.push(b as char);
            }
            let m = table[lcg(&mut state) as usize % 400.min(table.len())];
            text.push(m);
        } else if roll < 7 {
            // a non-Latin run (Cyrillic-shaped), 3-8 chars
            let rlen = 3 + (lcg(&mut state) % 6) as usize;
            for _ in 0..rlen {
                let m = table[lcg(&mut state) as usize % table.len()];
                if (m as u32) < 0x0400 || (m as u32) > 0x04FF {
                    text.push(char::from_u32(0x0430 + (lcg(&mut state) % 30) as u32).unwrap());
                } else {
                    text.push(m);
                }
            }
            text.push(' ');
        }
    }
    Input { text, table }
}

mod buggy {
    pub fn count_marks(text: &str, table: &[char]) -> usize {
        let mut n = 0usize;
        for c in text.chars() {
            if table.binary_search(&c).is_ok() {
                n += 1;
            }
        }
        n
    }
}

mod fixed {
    fn ascii_prefix_len(input: &[u8]) -> usize {
        let mut chunks = input.chunks_exact(8);
        let mut offset = 0usize;
        for chunk in &mut chunks {
            let word = u64::from_le_bytes(chunk.try_into().unwrap());
            if word & 0x8080_8080_8080_8080 != 0 {
                return offset + word.trailing_zeros() as usize / 8;
            }
            offset += 8;
        }
        let rem = chunks.remainder();
        offset + rem.iter().position(|&b| b & 0x80 != 0).unwrap_or(rem.len())
    }

    pub fn count_marks(text: &str, table: &[char]) -> usize {
        let bytes = text.as_bytes();
        let mut n = 0usize;
        let mut i = ascii_prefix_len(bytes);
        while i < bytes.len() {
            let c = text[i..].chars().next().unwrap();
            if table.binary_search(&c).is_ok() {
                n += 1;
            }
            i += c.len_utf8();
            i += ascii_prefix_len(&bytes[i..]);
        }
        n
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_marks(&input.text, &input.table));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_marks(&input.text, &input.table));
}
"#
        .to_string(),
    }
}

/// Bench spec for `range-table-page-memoization`: classify a locality-rich
/// query stream (text-shaped: one dominant page, script bursts, a small CJK
/// scatter) — binary search per query vs the lazy per-page predicate cache.
#[cfg(feature = "rust_perf_bench")]
fn range_table_page_memoization_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![10_000, 100_000, 1_000_000],
        harness: r#"
use std::sync::OnceLock;

fn build_table() -> Vec<char> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut v: Vec<char> = Vec::with_capacity(1567);
    while v.len() < 1567 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let cp = 0x80u32 + ((state >> 33) % 0x10FF80) as u32;
        if let Some(c) = char::from_u32(cp) {
            v.push(c);
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

fn table() -> &'static [char] {
    static T: OnceLock<Vec<char>> = OnceLock::new();
    T.get_or_init(build_table)
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    queries: Vec<char>,
    pages: Vec<OnceLock<Box<[bool; 256]>>>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x0123_4567_89AB_CDEF;
    let mut queries = Vec::with_capacity(n);
    while queries.len() < n {
        let roll = lcg(&mut state) % 10_000;
        if roll < 7_000 {
            // ASCII lowercase bulk — one page
            queries.push((b'a' + (lcg(&mut state) % 26) as u8) as char);
        } else if roll < 8_500 {
            // Latin-extended / combining burst — pages 0-3
            let cp = 0x00C0u32 + (lcg(&mut state) % 0x2B0) as u32;
            queries.push(char::from_u32(cp).unwrap_or('\u{00e9}'));
        } else if roll < 9_950 {
            // Cyrillic burst — pages 4-5
            let cp = 0x0400u32 + (lcg(&mut state) % 0x100) as u32;
            queries.push(char::from_u32(cp).unwrap_or('\u{0430}'));
        } else {
            // CJK scatter — many pages, the honesty tail
            let cp = 0x4E00u32 + (lcg(&mut state) % 0x1000) as u32;
            queries.push(char::from_u32(cp).unwrap_or('\u{4E00}'));
        }
    }
    // 4,352 empty page slots: the outer array is eager, contents lazy —
    // the fills happen INSIDE the timed queries, exactly like the source's
    // process-global LazyLock cache on first touch.
    let pages = (0..(char::MAX as usize + 1) / 256)
        .map(|_| OnceLock::new())
        .collect();
    Input { queries, pages }
}

mod buggy {
    pub fn classify(queries: &[char], table: &[char]) -> usize {
        queries
            .iter()
            .filter(|&&c| table.binary_search(&c).is_ok())
            .count()
    }
}

mod fixed {
    pub fn classify(
        queries: &[char],
        pages: &[std::sync::OnceLock<Box<[bool; 256]>>],
        table: &[char],
    ) -> usize {
        let get = |c: char| -> bool {
            let page_index = c as u32 as usize / 256;
            let index = c as u32 as usize % 256;
            let page = pages[page_index].get_or_init(|| {
                Box::new(std::array::from_fn(|idx| {
                    let cp = (page_index * 256 + idx) as u32;
                    char::from_u32(cp).is_some_and(|ch| table.binary_search(&ch).is_ok())
                }))
            });
            page[index]
        };
        queries.iter().filter(|&&c| get(c)).count()
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::classify(&input.queries, table()));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::classify(&input.queries, &input.pages, table()));
}
"#
        .to_string(),
    }
}
