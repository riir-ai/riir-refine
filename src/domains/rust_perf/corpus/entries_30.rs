//! Entries slice 30 — Batch 244 (2026-10-16, Plan 247): the text/unicode
//! processing family's FOLLOW-UP — the byte-scan/search mechanism batch —
//! distilled from three sources read at release pins: `rust-lang/regex` @
//! tag `regex-automata-0.4.18` (commit `e6ad938198e68339a593f85d228f2d44cb54718f`,
//! Apache-2.0/MIT), `BurntSushi/memchr` @ tag `2.8.3` (commit
//! `5fdb40c054e1fff359a2f7bdf7f87a13b34b465d`, Unlicense/MIT), and
//! `BurntSushi/bstr` @ tag `1.13.1` (commit
//! `134195be38c7c9a4887980fd0f6f8bae042dd91e`, Apache-2.0/MIT). All three
//! pins checked out at their release tags before any read; every quote in
//! this slice re-verified byte-for-byte at the `.raw` pin clones this
//! session (the Batch 31 law). Sources read by three disjoint read
//! subagents (regex-automata class machinery / memchr internals / bstr
//! internals) + coordinator dedup re-run at the 207-rule corpus.
//!
//! THE BATCH'S SHAPE: +10 rules, ALL benchable + annotated — dispatch +4
//! (prefilter-choice-shape-ladder, prefilter-skip-density-demotion,
//! byteset-cardinality-dispatch, utf8-fast-lane-cold-diagnostics), caching
//! +2 (byte-rank-select-rarest-probe, mod64-byte-membership-skip), layout
//! +1 (byte-class-equivalence-table-shrink), complexity +1
//! (alternation-prefix-collapse-at-build), iterators +1
//! (iter-count-bulk-specialization), io +1
//! (bufread-scan-in-buffer-scratch-fragment). The regex byte-class law is
//! the family's canonical upstream of Batch 243's ASCII-run-skip mechanism.
//!
//! HONEST DROPS + DEFERS (coordinator adjudication, recorded in plan 247 +
//! Research 276): `rare-byte-rank-prefilter-gate` (regex-syntax) and
//! `pair-rank-select-by-frequency` (memchr packedpair) are ONE law in two
//! repos — MERGED as `byte-rank-select-rarest-probe` with both citations;
//! the bstr twin of memmem Finder hoisting folded into the same
//! adjudication (deferred with the memchr half to Batch 245, adjacent to
//! `regex-hoist-lazylock`). DEFERRED to Batch 245 as staged leads with
//! reasons: self-loop-state-memchr-skip (the dfa/accel.rs composition —
//! high value, fixture needs long homogeneous runs + a negative arm),
//! pair-probe-confirm-fused (offset-pair AND mask — dedup vs
//! dual-mask-fused-block-scan needs its own pass), align-probe-overlap-tail
//! (alignment half washes on modern unaligned-load µarchs; the tail
//! conversion is real but offset-dependent), match-branch-cold-confirm
//! (attribute-only; the source records its own counter-case),
//! char-search-encode-once-stack (thin sibling of
//! byte-find-position-to-memchr), canonical-set-guarded-rebuild (adjacent
//! to sortedness-check-branchless-accumulate), nonmax-sentinel-option-niche
//! (timing-only-at-scale). DROPPED: sentinel-run-word-broadcast-skip (same
//! genus as ascii-run-swar-block-skip — run-skip via word-wide ops),
//! cross-product-budget-degrade (the Batch-243 fixture-artificiality
//! class).

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── layout: collapse a byte-keyed table's key space to equivalence
        // classes before the table, not after ─────────────────────────────
        // Source: rust-lang/regex @ regex-automata-0.4.18,
        // regex-automata/src/util/alphabet.rs.
        PerfRule {
            rule_id: "byte-class-equivalence-table-shrink".to_string(),
            name: "Byte-keyed transition tables indexed by all 256 byte values → collapse the key space to equivalence classes first (bytes that never discriminate share a class id; the live table shrinks states×256 → states×classes and goes cache-resident)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// NAIVE: one table column per byte value. Most columns are\n// identical — every byte outside the token alphabet behaves the same\n// in every state — but a 256-wide table cannot know that, so the live\n// table is states x 256 cells and the scan walks a fresh row per byte.\n\nconst NSTATES: usize = 1024;\n\nstatic TRANS: [[u8; 256]; NSTATES] = build_full_table();\n\npub fn scan(src: &[u8]) -> usize {\n    let mut state = 0usize;\n    let mut accepts = 0usize;\n    for &b in src {\n        let next = TRANS[state][b as usize] as usize;\n        if state != 0 && next == 0 {\n            accepts += 1;\n        }\n        state = next;\n    }\n    accepts\n}\n\n// The builder fills every one of the 256 columns; only a handful are\n// distinct.\nfn build_full_table() -> [[u8; 256]; NSTATES] {\n    let mut t = [[0u8; 256]; NSTATES];\n    for byte in 0u8..=255 {\n        let class = classify(byte);\n        for (s, row) in t.iter_mut().enumerate() {\n            row[byte as usize] = step_full(s, class);\n        }\n    }\n    t\n}\n\nfn classify(b: u8) -> u8 {\n    match b {\n        b'a'..=b'z' => 1,\n        b'A'..=b'Z' => 2,\n        b'0'..=b'9' => 3,\n        b'_' => 4,\n        b'.' => 5,\n        b'-' => 6,\n        _ => 0,\n    }\n}\n\nfn step_full(state: usize, class: u8) -> u8 {\n    // a real lexer's per-class transition fn (fixture-shaped)\n    match (state, class) {\n        (0, 1..=4) => 1,\n        (1, 1..=4) => 1,\n        (1, 5) => 2,\n        (2, 1..=4) => 3,\n        (3, 1..=4) => 3,\n        (0, 3) => 4,\n        (4, 3) => 4,\n        _ => 0,\n    }\n}".to_string(),
            fixed_code: "// OPT: bytes that never discriminate share an equivalence class\n// id; the class map is ONE 256-entry array built once from the\n// discriminating range boundaries, and the live table shrinks to\n// states x classes. The per-byte cost stays two array reads; the table\n// the scan actually walks goes cache-resident.\n\nconst NCLASSES: usize = 8;\n\n/// The class of every byte: walk 0..=255, bump the id at each\n/// discriminating boundary. Boundaries come from the ranges themselves:\n/// a range [start, end] that can discriminate marks start-1 and end\n/// (regex-automata's ByteClassSet::set_range).\nstatic CLASS_OF: [u8; 256] = {\n    let mut map = [0u8; 256];\n    let ranges: [(u8, u8); 7] = [\n        (b'a', b'z'),\n        (b'A', b'Z'),\n        (b'0', b'9'),\n        (b'_', b'_'),\n        (b'.', b'.'),\n        (b'-', b'-'),\n        (0, 0), // the everything-else class closes the space\n    ];\n    let mut boundaries = [false; 256];\n    let mut n = 0usize;\n    for &(start, end) in ranges.iter().take(6) {\n        if start > 0 {\n            boundaries[(start - 1) as usize] = true;\n            n += 1;\n        }\n        boundaries[end as usize] = true;\n        n += 1;\n    }\n    let _ = n;\n    let mut class = 0u8;\n    let mut b = 0usize;\n    loop {\n        map[b] = class;\n        if b == 255 {\n            break;\n        }\n        if boundaries[b] {\n            class += 1;\n        }\n        b += 1;\n    }\n    map\n};\n\nstatic TRANS: [[u8; NCLASSES]; 1024] = {\n    let mut t = [[0u8; NCLASSES]; 1024];\n    for class in 0u8..NCLASSES as u8 {\n        for (s, row) in t.iter_mut().enumerate() {\n            row[class as usize] = step_full(s, class);\n        }\n    }\n    t\n};\n\npub fn scan(src: &[u8]) -> usize {\n    let mut state = 0usize;\n    let mut accepts = 0usize;\n    for &b in src {\n        let next = TRANS[state][CLASS_OF[b as usize] as usize] as usize;\n        if state != 0 && next == 0 {\n            accepts += 1;\n        }\n        state = next;\n    }\n    accepts\n}\n\nfn classify(b: u8) -> u8 {\n    match b {\n        b'a'..=b'z' => 1,\n        b'A'..=b'Z' => 2,\n        b'0'..=b'9' => 3,\n        b'_' => 4,\n        b'.' => 5,\n        b'-' => 6,\n        _ => 0,\n    }\n}\n\nfn step_full(state: usize, class: u8) -> u8 {\n    match (state, class) {\n        (0, 1..=4) => 1,\n        (1, 1..=4) => 1,\n        (1, 5) => 2,\n        (2, 1..=4) => 3,\n        (3, 1..=4) => 3,\n        (0, 3) => 4,\n        (4, 3) => 4,\n        _ => 0,\n    }\n}\n\n#[test]\nfn classed_table_matches_the_full_table_on_every_byte() {\n    for byte in 0u8..=255 {\n        for state in 0..5usize {\n            assert_eq!(\n                TRANS[state][CLASS_OF[byte as usize] as usize],\n                step_full(state, classify(byte)),\n                \"byte {byte} state {state}\"\n            );\n        }\n    }\n    // identical accepts on mixed input\n    let src = b\"foo.bar_baz-99 QUX 12.34\";\n    assert_eq!(scan(src), {\n        let mut state = 0usize;\n        let mut accepts = 0usize;\n        for &b in src {\n            let next = step_full(state, classify(b)) as usize;\n            if state != 0 && next == 0 {\n                accepts += 1;\n            }\n            state = next;\n        }\n        accepts\n    });\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 1.39x/1.08x/tie (2/0/1) at n=1K/16K/262K over a 1024-state lexer FSM -- the 256-wide table is 256 KiB (L2-bound) vs the classed table 8 KiB (L1-resident); FIRST fixture shape (24 states, 6 KiB both arms L1-resident) measured Mixed 1/0/2 -- the table must exceed L1 for the law to breathe, that IS the rule regime, the restructure is disclosed here (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; every quote verified byte-for-byte at the .raw/regex pin clone this session, checked out AT the release tag before any read). THE LAW (regex-automata/src/util/alphabet.rs:9-19, verbatim): 'To a first approximation, the alphabet of all automata in this crate is just a `u8`. Namely, every distinct byte value. All 256 of them. In practice, this can be quite wasteful when building a transition table for a DFA, since it requires storing a state identifier for each element in the alphabet. Instead, we collapse the alphabet of an automaton down into equivalence classes, where every byte in the same equivalence class never discriminates between a match or a non-match from any other byte in the same class.' + the worked example: 'in the regex `[a-z]+`, then you could consider it having an alphabet consisting of two equivalence classes: `a-z` and everything else.' THE COST MODEL (alphabet.rs:24-26, verbatim — why the per-byte map is free): 'In practice, doing this for every state transition has negligible impact on modern CPUs. Moreover, it helps make more efficient use of the CPU cache by (possibly considerably) shrinking the size of the transition table.' THE WHOLE MAP IS ONE 256-ENTRY U8 ARRAY (alphabet.rs:214-215, verbatim): '#[derive(Clone, Copy)]' + 'pub struct ByteClasses([u8; 256]);' — the runtime cost is one index (alphabet.rs:292-296, verbatim): '/// Get the equivalence class for the given byte.' + 'pub fn get(&self, byte: u8) -> u8 { self.0[usize::from(byte)] }'. THE BUILDER MARKS RANGE BOUNDARIES ONLY (alphabet.rs:702-710, verbatim): '/// Indicate the range of byte given (inclusive) can discriminate a match between it and all other bytes outside of the range.' + set_range marks 'self.0.add(start - 1)' and 'self.0.add(end)'; byte_classes() then walks 0..=255 bumping the id at each boundary (alphabet.rs:722-737). THE STRIDE RIDER (alphabet.rs:331-335, verbatim): 'The stride is always the smallest power of 2 that is greater than or equal to the alphabet length' — 'converting between premultiplied state IDs and indices can be done with shifts alone, which is much faster than integer division' (pad the class count to next_power_of_two, premultiply state ids, index with one shift). REGIME: byte-keyed state machines / dispatch tables with class-like semantics (lexers, tokenizers, protocol FSMs, char-class recognizers) where the live table is states x 256 and exceeds L1 — 1024 states x 256 = 256 KiB vs 1024 x 8 = 8 KiB. ANTI-REGIMES: every byte genuinely discriminates (exact-match/perm tables — classes collapse back to 256, the map is pure overhead); a single-row table already cache-resident (the win needs MULTIPLE rows); the table rebuilt per call (build cost dominates); u16+ alphabets where the class map itself stops being one array. DEDUP ADJUDICATION (T0b, 207-rule corpus): distinct from precompute-table (a const LUT over the WHOLE key domain — this SHRINKS the key domain before the table exists; they compose: the class map is itself a precompute-table), range-table-page-memoization (runtime LAZY cache over a huge dense domain — this is BUILD-TIME key-space collapse over a bounded 256 domain), bsearch-sorted-slice (sparse lookup, no state machine), unaligned-word-load-from-bytes (word assembly), repr-u8-enum (enum width, not key-space collapse). Layout family: the win is cache-density (timing-only Pareto; both arms alloc-free — the alloc axis is 0/0, disclosed). The source's own equivalence test (byte_classes round-trips) is the model for the in-rule test. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byte_class_equivalence_table_shrink_bench_spec()),
        },
        // ── dispatch: choose the search accelerator by target-set shape,
        // refuse shapes that match everywhere ──────────────────────────────
        // Source: rust-lang/regex @ regex-automata-0.4.18,
        // regex-automata/src/util/prefilter/mod.rs.
        PerfRule {
            rule_id: "prefilter-choice-shape-ladder".to_string(),
            name: "One generic scan loop over every search target → key the matcher on the target-set SHAPE at build time (1–3 single bytes ride memchr/memchr2/memchr3, one substring rides a prebuilt Finder, refuse shapes that match everywhere)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: one scan loop that treats every target the same — a\n// byte-wise window compare per target per position, no matter what\n// the targets look like.\n\npub fn first_hit(hay: &[u8], targets: &[&[u8]]) -> Option<usize> {\n    let mut best: Option<usize> = None;\n    for t in targets {\n        if t.is_empty() {\n            continue;\n        }\n        if hay.len() < t.len() {\n            continue;\n        }\n        for i in 0..=(hay.len() - t.len()) {\n            if &hay[i..i + t.len()] == *t {\n                best = Some(match best {\n                    Some(b) if b <= i => b,\n                    _ => i,\n                });\n                break;\n            }\n        }\n    }\n    best\n}".to_string(),
            fixed_code: "// OPT: the matcher is keyed ONCE at build on the target-set shape:\n// all-1-byte sets ride memchr/memchr2/memchr3 (SIMD), a lone substring\n// rides a prebuilt Finder, mixed sets split — the single bytes seed the\n// scan by SIMD, the substring is only confirmed from there. Shapes that\n// match everywhere (empty target) refuse the whole build.\n// [dep: memchr = \"2\"]\n\nuse memchr::{memchr, memchr2};\nuse memchr::memmem::Finder;\n\npub enum TargetMatcher {\n    One(u8),\n    Two(u8, u8),\n    Sub(Finder<'static>),\n    Mixed { byte: u8, sub: Finder<'static> },\n}\n\nimpl TargetMatcher {\n    /// None = the set can never prefilter (empty target).\n    pub fn build(targets: &[Vec<u8>]) -> Option<TargetMatcher> {\n        if targets.iter().any(|t| t.is_empty()) {\n            return None;\n        }\n        match targets.len() {\n            1 if targets[0].len() == 1 => Some(TargetMatcher::One(targets[0][0])),\n            2 if targets.iter().all(|t| t.len() == 1) => {\n                Some(TargetMatcher::Two(targets[0][0], targets[1][0]))\n            }\n            1 => Some(TargetMatcher::Sub(Finder::new(targets[0].as_slice()).into_owned())),\n            _ => {\n                // mixed: seed with a 1-byte target, confirm the substring\n                let byte = targets.iter().find(|t| t.len() == 1).map(|t| t[0])?;\n                let sub = targets.iter().find(|t| t.len() > 1)?;\n                Some(TargetMatcher::Mixed {\n                    byte,\n                    sub: Finder::new(sub.as_slice()).into_owned(),\n                })\n            }\n        }\n    }\n\n    pub fn first_hit(&self, hay: &[u8]) -> Option<usize> {\n        match self {\n            TargetMatcher::One(b) => memchr(*b, hay),\n            TargetMatcher::Two(a, b) => memchr2(*a, *b, hay),\n            TargetMatcher::Sub(f) => f.find(hay),\n            TargetMatcher::Mixed { byte, sub } => {\n                let from = memchr(*byte, hay).unwrap_or(hay.len());\n                sub.find(hay).filter(|i| *i <= from).or((from < hay.len()).then_some(from))\n            }\n        }\n    }\n}\n\n#[test]\nfn shape_keyed_matcher_agrees_with_the_generic_scan() {\n    let hay = b\"\\tWARN boot ok\\n\\tERROR: disk 3\\n\\tINFO done\";\n    let targets = vec![b\"\\t\".to_vec(), b\"ERROR:\".to_vec()];\n    let m = TargetMatcher::build(&targets).unwrap();\n    assert_eq!(m.first_hit(hay), first_hit(hay, &targets));\n    // single-byte pair rides the memchr2 arm\n    let bytes = vec![b'x'.to_vec(), b'z'.to_vec()];\n    let m2 = TargetMatcher::build(&bytes).unwrap();\n    assert!(matches!(m2, TargetMatcher::Two(_, _)));\n    assert_eq!(m2.first_hit(b\"aaz\"), Some(1));\n    // an empty target refuses the build\n    assert!(TargetMatcher::build(&[vec![]]).is_none());\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 46.62x/44.50x/44.00x (3/0/0) at n=1K/16K/262K, earliest-of over a mixed target set (one 1-byte + one 6-byte substring) over log-like text -- memchr seed + prebuilt Finder vs per-target window scans (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone). THE SELECTION LAW (regex-automata/src/util/prefilter/mod.rs:575-577, verbatim): 'What this selection algorithm does is attempt to use the fastest prefilter that works for the literals given. So if `[a, b]`, is given, then `memchr2` is selected.' THE LADDER (prefilter/mod.rs:606-633, verbatim structure): an ordered cheapest-first try-chain — 'if let Some(pre) = Memchr::new(kind, needles) { return Some(Choice::Memchr(pre)); }' then Memchr2, Memchr3, Memmem, Teddy, ByteSet, AhoCorasick — each constructor returns Option (None = shape not applicable) and the FIRST applicable arm wins. THE REFUSAL GUARDS (prefilter/mod.rs:588-600, verbatim): 'An empty set means the regex matches nothing, so no sense in building a prefilter.' + 'If the regex can match the empty string, then the prefilter will by definition match at every position. This is obviously completely ineffective.' THE FP PING-PONG COST MODEL (prefilter/mod.rs:12-18, verbatim): 'if a prefilter has a high false positive rate and it produces lots of candidates, then a prefilter can overall make a regex search slower. It can run more slowly because more time is spent ping-ponging between the prefilter search and the regex engine attempting to confirm each candidate as a match.' THE IS_FAST GATE (prefilter/mod.rs:437-444, verbatim): 'Implementations might return true here if they believe themselves to be \"fast.\" ... some prefilter tricks tend to come with their own bits of overhead, and so might only make sense if we know that a scan will be *much* faster than the regex engine itself.' (the source hoists the polymorphic is_fast query into a plain bool field at construction — the transferable half for code that must keep trait objects). REGIME: the target set is fixed at build time and searched many times; haystacks large relative to the scan constant. ANTI-REGIMES: the target set changes per call (selection + builder cost per call); tiny haystacks where the naive scan is already sub-microsecond; is_fast-false shapes (a common-byte seed inside a mixed set — the FP ping-pong above). DEDUP ADJUDICATION (T0b): distinct from byte-find-position-to-memchr (ONE needle, always memchr — this selects AMONG accelerators by shape), dual-mask-fused-block-scan (fuses predicates over ALL positions in blocks — this is earliest-position dispatch), Box-dyn-to-enum dispatch (devirtualizing a FIXED operation — this is algorithm SELECTION with Option-returning constructors + a refuse arm), dispatch-handler-pair-hoist (loop-invariant hoisting). Composes with prefilter-skip-density-demotion (the runtime demotion of a chosen arm). Bench: mixed target set (one 1-byte + one substring) over log-like text — the fixed arm seeds by memchr and confirms the substring only from the seed; alloc 0 at search time (all builders run once in make_input). Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(prefilter_choice_shape_ladder_bench_spec()),
        },
        // ── complexity: collapse a long common prefix to one literal at
        // build time — single-substring search is the fastest prefilter ──
        // Source: rust-lang/regex @ regex-automata-0.4.18,
        // regex-syntax/src/hir/literal.rs.
        PerfRule {
            rule_id: "alternation-prefix-collapse-at-build".to_string(),
            name: "Scanning for every literal in an alternation → collapse a long common prefix to ONE literal at build time (single-substring search is the fastest prefilter; shadowed literals and oversized sets get minimized too; empty literals poison)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// NAIVE: the keyword list is scanned one literal at a time — N\n// full passes over the haystack for N literals that all share a\n// long prefix.\n\npub fn count_keyword_hits(hay: &[u8], keywords: &[Vec<u8>]) -> usize {\n    let mut total = 0usize;\n    for kw in keywords {\n        if kw.is_empty() || hay.len() < kw.len() {\n            continue;\n        }\n        for i in 0..=(hay.len() - kw.len()) {\n            if &hay[i..i + kw.len()] == kw.as_slice() {\n                total += 1;\n            }\n        }\n    }\n    total\n}".to_string(),
            fixed_code: "// OPT: at build time — drop literals shadowed by an earlier prefix,\n// collapse a long common prefix to ONE literal (every literal is then\n// equivalent to the prefix; the suffix is rechecked per hit), and\n// truncate oversized sets to short prefixes. The search is one pass.\n// [dep: memchr = \"2\"]\n\nuse memchr::memmem::Finder;\n\npub struct KeywordMatcher {\n    prefix: Finder<'static>,\n    prefix_len: usize,\n    keywords: Vec<Vec<u8>>,\n}\n\nimpl KeywordMatcher {\n    pub fn build(keywords: &[Vec<u8>]) -> Option<KeywordMatcher> {\n        // an empty literal matches every position — poison (squash)\n        if keywords.iter().any(|k| k.is_empty()) {\n            return None;\n        }\n        let mut kws = keywords.to_vec();\n        // preference-trie shadowing: a later literal sharing an earlier\n        // literal as a strict prefix can never win a leftmost-first scan\n        kws.sort();\n        kws.dedup_by(|a, b| b.starts_with(a.as_slice()));\n        let lcp = longest_common_prefix(&kws)?;\n        // usefix: long enough to beat N separate scans\n        if lcp.len() > 4 {\n            return Some(KeywordMatcher {\n                prefix: Finder::new(lcp).into_owned(),\n                prefix_len: lcp.len(),\n                keywords: kws,\n            });\n        }\n        None // short prefix: keep the per-literal lane (regime boundary)\n    }\n\n    pub fn count_hits(&self, hay: &[u8]) -> usize {\n        let mut total = 0usize;\n        let mut start = 0usize;\n        while let Some(at) = self.prefix.find(&hay[start..]) {\n            let pos = start + at;\n            if self.keywords.iter().any(|kw| {\n                hay.len() >= pos + kw.len() && &hay[pos..pos + kw.len()] == kw.as_slice()\n            }) {\n                total += 1;\n            }\n            start = pos + 1;\n        }\n        total\n    }\n}\n\nfn longest_common_prefix(lits: &[Vec<u8>]) -> Option<&[u8]> {\n    let base = &lits[0];\n    let mut len = base.len();\n    for m in lits.iter().skip(1) {\n        len = m\n            .iter()\n            .zip(base[..len].iter())\n            .take_while(|&(a, b)| a == b)\n            .count();\n        if len == 0 {\n            return None;\n        }\n    }\n    Some(&base[..len])\n}\n\n#[test]\nfn collapsed_matcher_agrees_with_per_literal_scan() {\n    let kws: Vec<Vec<u8>> = [\"user_admin\", \"user_ban\", \"user_banip\", \"user_kick\"]\n        .iter()\n        .map(|s| s.as_bytes().to_vec())\n        .collect();\n    let hay = b\"..user_ban then user_kick, user_admin! user_banip?\";\n    let m = KeywordMatcher::build(&kws).unwrap();\n    assert_eq!(m.count_hits(hay), count_keyword_hits(hay, &kws));\n    // shadowed literal dropped at build\n    let shadowed = vec![b\"sam\".to_vec(), b\"samwise\".to_vec()];\n    assert_eq!(KeywordMatcher::build(&shadowed).map(|m| m.keywords.len()), Some(1));\n    // empty literal poisons the build\n    assert!(KeywordMatcher::build(&[b\"ok\".to_vec(), vec![]]).is_none());\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 639.67x/355.14x/334.80x (3/0/0) at n=1K/16K/262K, 64 keywords sharing a 5-byte prefix -- one Finder pass + suffix confirms vs 64 full window scans (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone). THE COLLAPSE LAW (regex-syntax/src/hir/literal.rs:1858-1860, verbatim): 'Look for a common prefix (or suffix). If we found one of those and it's long enough, then it's a good bet that it will be our fastest possible prefilter since single-substring search is so fast.' THE USEFIX PREDICATE (literal.rs:1892-1897, verbatim): 'We only strip down to the common prefix/suffix if we think the existing set of literals isn't great, or if the common prefix/suffix is expected to be particularly discriminatory.' + 'let usefix = fix.len() > 4 || (fix.len() > 1 && !isfast);' with isfast = 'self.is_exact() && self.len().map_or(false, |len| len <= 16)'. THE SHADOWING TRIE (literal.rs:2184-2189, verbatim): 'A \"preference\" trie that rejects literals that will never match when executing a leftmost first or \"preference\" search.' + 'if 'sam' is inserted, then trying to insert 'samwise' will be rejected because 'samwise' can never match since 'sam' will always take priority.' THE OVERSIZED-SET TRUNCATION LADDER (literal.rs:1942-1951, verbatim): 'The pair of numbers in this list corresponds to the maximal prefix (in bytes) to keep for all literals and the length of the sequence at which to do it.' + 'const ATTEMPTS: [(usize, usize); 5] = [(5, 10), (4, 10), (3, 64), (2, 64), (1, 10)];'. THE EMPTY-LITERAL POISON (literal.rs:1840-1847, verbatim): 'An empty string implies a match at every position. A prefilter cannot help you here.' + 'self.make_infinite()'. THE PREFIX-COLLAPSE MECHANICS (literal.rs:1899-1912, verbatim): 'If we keep exactly the number of bytes equal to the length of the prefix (or suffix), then by the definition of a prefix, every literal in the sequence will be equivalent. Thus, 'dedup' will leave us with one literal.' + 'We do it this way to avoid an alloc'. THE LCP ITSELF (literal.rs:1639-1652, verbatim shape): the zip/take_while/count walk over the literal list, early-returning on the first zero-length overlap. REGIME: multi-literal matching built once + searched repeatedly (keyword lists, dispatch markers, protocol vocabularies) sharing a build-time-detectable common prefix/suffix. ANTI-REGIMES: one-shot searches (build cost never amortizes); consumers needing FULL-literal semantics without a confirm pass (the collapse deliberately throws the suffix away — it is a PREFILTER; the hit must be rechecked against the real literals); preference-ORDER-sensitive consumers that cannot tolerate shadow removal; sets whose LCP is <= 4 bytes (the usefix boundary). DEDUP ADJUDICATION (T0b): distinct from prefilter-choice-shape-ladder (which accelerator for a FIXED set — this SHRINKS the set before any accelerator is chosen; they compose), byte-find-position-to-memchr / dual-mask-fused-block-scan (scan shapes, not candidate-set reduction), prompt-cache-canonical-interval-keys (serving cache keying). The rare-prefix strip-down face (literal.rs:1867-1891, 'if we have a common prefix and the leading byte of that prefix is one that we think probably occurs rarely, then strip everything down to just that single byte. This should promote the use of memchr.') is cited in byte-rank-select-rarest-probe (same batch). Bench: 64 keywords sharing a 5-byte prefix over log-like text; buggy = per-literal window scans; fixed = one Finder pass + suffix confirms at real hits. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(alternation_prefix_collapse_at_build_bench_spec()),
        },
        // ── caching: probe bytes selected by background-frequency rank, with
        // the poison refusal — the merged regex-syntax + memchr law ────────
        // Sources: rust-lang/regex @ regex-automata-0.4.18
        // (regex-syntax/src/hir/literal.rs) + BurntSushi/memchr @ 2.8.3
        // (src/arch/all/packedpair/mod.rs + src/memmem/searcher.rs).
        PerfRule {
            rule_id: "byte-rank-select-rarest-probe".to_string(),
            name: "Seeding a candidate scan with the needle's first byte → select the probe byte by background frequency rank (the rarest byte minimizes false-positive verifies; a rank ≥250 byte means NO prefilter at all)".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: the scan is seeded with whatever byte comes first in the\n// needle — for text needles that is often a space or a letter, which\n// matches everywhere and drags the expensive verify behind every hit.\n// [dep: memchr = \"2\"]\n\nuse memchr::memchr;\n\npub fn count_occurrences(hay: &[u8], needle: &[u8]) -> usize {\n    if needle.is_empty() || hay.len() < needle.len() {\n        return 0;\n    }\n    let mut total = 0usize;\n    let mut start = 0usize;\n    while let Some(at) = memchr(needle[0], &hay[start..]) {\n        let pos = start + at;\n        if hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle {\n            total += 1;\n        }\n        start = pos + 1;\n    }\n    total\n}".to_string(),
            fixed_code: "// OPT: consult a background-frequency rank table ONCE at build, probe\n// with the needle's RAREST byte, and refuse the prefilter entirely when\n// even the rarest byte is common (rank >= 250 — the source's poison\n// threshold). [dep: memchr = \"2\"]\n\nuse memchr::memchr;\n\n/// Background byte ranks, fixture-local: LOWER rank = rarer byte (the\n/// source's law, its exact table is theirs). Ranks here are honest\n/// text-shaped synthetic values, not a copy of BYTE_FREQUENCIES.\nfn rank(b: u8) -> u8 {\n    match b {\n        b' ' => 255,\n        b'e' | b't' | b'a' | b'o' | b'n' => 250,\n        b'i' | b's' | b'r' | b'h' | b'l' => 240,\n        b'd' | b'c' | b'u' | b'm' => 230,\n        b'w' | b'f' | b'g' | b'y' | b'p' | b'b' => 215,\n        b'v' | b'k' | b'j' | b'x' | b'q' | b'z' => 190,\n        b'0'..=b'9' => 160,\n        b'A'..=b'Z' => 120,\n        b'~' | b'^' | b'|' | b'{' | b'}' => 5,\n        _ => 100,\n    }\n}\n\nconst MAX_FALLBACK_RANK: u8 = 250;\n\npub struct RareProbe {\n    probe: u8,\n    needle: Vec<u8>,\n}\n\nimpl RareProbe {\n    /// None = even the rarest byte is common — no prefilter (the scan\n    /// runs bare).\n    pub fn build(needle: &[u8]) -> Option<RareProbe> {\n        if needle.is_empty() {\n            return None;\n        }\n        let (probe, rank) = needle\n            .iter()\n            .copied()\n            .map(|b| (b, rank(b)))\n            .min_by_key(|&(_, r)| r)\n            .unwrap();\n        if rank > MAX_FALLBACK_RANK {\n            return None;\n        }\n        Some(RareProbe { probe, needle: needle.to_vec() })\n    }\n\n    pub fn count_occurrences(&self, hay: &[u8]) -> usize {\n        let needle = &self.needle;\n        let mut total = 0usize;\n        let mut start = 0usize;\n        while let Some(at) = memchr(self.probe, &hay[start..]) {\n            let pos = start + at;\n            if hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle {\n                total += 1;\n            }\n            start = pos + 1;\n        }\n        total\n    }\n}\n\n#[test]\nfn rare_probe_matches_first_byte_probe_and_refuses_poison() {\n    let hay = b\"ERROR: x ERROR: y e e e ERROR: z\";\n    let needle = b\" ERROR \";\n    assert_eq!(count_occurrences(hay, needle), 3);\n    let probe = RareProbe::build(needle).unwrap();\n    assert_ne!(probe.probe, b' '); // the first byte was refused\n    assert_eq!(probe.count_occurrences(hay), 3);\n    // all-space needle: even the rarest byte is rank 255 — no prefilter\n    assert!(RareProbe::build(b\"   \").is_none());\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 38.09x/24.02x/22.93x (3/0/0) at n=1K/16K/262K, needle SPACE-ERROR-SPACE over a lowercase log corpus -- rarest-byte memchr + verify vs first-byte (space) memchr + verify; the verify-skip is the whole win, the fixture models the full-needle confirm honestly (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). ONE LAW IN TWO REPOS — MERGED at T0b (the dedup re-run): regex-syntax's rank()+is_poisonous and memchr's Pair ranking are the same background-frequency mechanism, same 250 threshold, independently maintained. SOURCE A (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone): THE RANK (regex-syntax/src/hir/literal.rs:2312-2321, verbatim): '/// Returns the \"rank\" of the given byte.' + 'The rank of a byte is derived from a heuristic background distribution of relative frequencies of bytes. The heuristic says that lower the rank of a byte, the less likely that byte is to appear in any arbitrary haystack.' + 'pub fn rank(byte: u8) -> u8 { crate::rank::BYTE_FREQUENCIES[usize::from(byte)] }'. THE POISON GATE (literal.rs:2149-2153, verbatim): '/// Returns true if it is believe that this literal is likely to match very frequently, and is thus not a good candidate for a prefilter.' + 'fn is_poisonous(&self) -> bool { self.is_empty() || (self.len() == 1 && rank(self.as_bytes()[0]) >= 250) }'. THE RARE-PREFIX STRIP-DOWN (literal.rs:1867-1870, verbatim): 'As a special case, if we have a common prefix and the leading byte of that prefix is one that we think probably occurs rarely, then strip everything down to just that single byte. This should promote the use of memchr.' SOURCE B (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT; quotes verified at the pin): THE PAIR LAW (src/arch/all/packedpair/mod.rs:156-158, verbatim): 'The idea is to select a pair of bytes that is believed to strongly predict a match in the haystack. This usually means selecting bytes that occur rarely in a haystack.' THE RAREST-TWO SCAN (packedpair/mod.rs:170-171, verbatim): '// Find the rarest two bytes. We make them distinct indices by construction.' (the full two-smallest selection walk at L172-190). THE DOMAIN-SWAPPABLE RANKER (packedpair/mod.rs:259-262, verbatim): 'there are some specific datasets (like binary executables) that have dramatically different byte distributions. For these datasets customizing the byte frequency heuristic can have a massive impact on performance, and might even need to be done at runtime.' THE SAME 250 REFUSAL (src/memmem/searcher.rs:620-624, verbatim): '/// The maximum frequency rank permitted for the fallback prefilter. If the rarest byte in the needle has a frequency rank above this value, then no prefilter is used' + 'const MAX_FALLBACK_RANK: u8 = 250;'. REGIME: text-like haystacks with skewed byte distributions; needles >= 2 bytes whose FIRST byte is common; the verify is nontrivial (full-needle compare). ANTI-REGIMES (source-stated): haystacks with inverted statistics (binary blobs, UTF-16, base64 — the rank table is WRONG there; the domain-swappable ranker exists for exactly this); verification so cheap the FP cost never materializes; single-candidate scans (no choice to make); needles whose every byte is common (the poison refusal). HONEST BENCH RISK (the Batch-243 drop class flagged at authoring): the win = FP-rate x verify-cost — the fixture must model a REALISTIC verify (full-needle compare over a log-like corpus) or the rule reads ParetoDominated; measured below, dropped honestly if dominated. DEDUP ADJUDICATION (T0b): distinct from byte-find-position-to-memchr (which PRIMITIVE — this selects the PROBE byte within the same primitive), prefilter-choice-shape-ladder (shape dispatch, not byte selection), dual-mask-fused-block-scan (block masks, all positions), range-table-page-memoization (property-cache, not probe selection). The fixture's rank fn is fixture-local honest text-shaped values — the source's exact BYTE_FREQUENCIES table is theirs, cited not copied. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byte_rank_select_rarest_probe_bench_spec()),
        },
        // ── dispatch: a fast path that stops paying must be measured and
        // demoted at runtime — the PrefilterState feedback-control law ────
        // Source: BurntSushi/memchr @ 2.8.3, src/memmem/searcher.rs.
        PerfRule {
            rule_id: "prefilter-skip-density-demotion".to_string(),
            name: "A candidate-probe fast path that always runs → track its skip density and latch it inert at runtime (below the calibrated skip average the probe costs more than it saves; the sentinel 0 makes the hot check one compare)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: the cheap candidate probe runs unconditionally on every\n// search, even when the data makes it fire constantly and the\n// expensive verify behind it eats the win.\n\npub struct Scanner {\n    needle: Vec<u8>,\n}\n\nimpl Scanner {\n    pub fn count_matches(&self, hay: &[u8]) -> usize {\n        if hay.len() < self.needle.len() {\n            return 0;\n        }\n        let mut total = 0usize;\n        let mut pos = 0usize;\n        while pos + self.needle.len() <= hay.len() {\n            // cheap probe: the 2-byte head\n            if hay[pos] == self.needle[0] && hay[pos + 1] == self.needle[1] {\n                if &hay[pos..pos + self.needle.len()] == self.needle.as_slice() {\n                    total += 1;\n                }\n            }\n            pos += 1;\n        }\n        total\n    }\n}".to_string(),
            fixed_code: "// OPT: a tiny per-search state (two u32s) tracks how many bytes the\n// probe skips on average; below the calibrated floor the probe LATCHES\n// inert for the rest of the search and the plain scan takes over. The\n// sentinel 0 = inert keeps the hot check one compare.\n\nconst MIN_SKIPS: u32 = 50;\nconst MIN_SKIP_BYTES: u32 = 8;\n\n#[derive(Clone, Copy)]\npub struct ProbeState {\n    /// Always 1 greater than the actual count; 0 = inert.\n    skips: u32,\n    skipped: u32,\n}\n\nimpl ProbeState {\n    pub fn new() -> Self {\n        Self { skips: 1, skipped: 0 }\n    }\n\n    fn update(&mut self, bytes_skipped: u32) {\n        if self.skips == 0 {\n            return; // already inert\n        }\n        self.skipped = self.skipped.saturating_add(bytes_skipped);\n        self.skips = self.skips.saturating_add(1);\n    }\n\n    fn is_effective(&mut self) -> bool {\n        if self.skips == 0 {\n            return false;\n        }\n        if self.skips.saturating_sub(1) < MIN_SKIPS {\n            return true;\n        }\n        if self.skipped >= MIN_SKIP_BYTES * self.skips.saturating_sub(1) {\n            return true;\n        }\n        self.skips = 0; // latch inert\n        false\n    }\n}\n\npub struct Scanner {\n    needle: Vec<u8>,\n}\n\nimpl Scanner {\n    pub fn count_matches(&self, hay: &[u8]) -> usize {\n        if hay.len() < self.needle.len() {\n            return 0;\n        }\n        let mut total = 0usize;\n        let mut pos = 0usize;\n        let mut state = ProbeState::new();\n        let probe_worthwhile = state.is_effective();\n        let n = self.needle.len();\n        while pos + n <= hay.len() {\n            let mut matched = false;\n            if probe_worthwhile\n                && hay[pos] == self.needle[0]\n                && hay[pos + 1] == self.needle[1]\n            {\n                matched = &hay[pos..pos + n] == self.needle.as_slice();\n                if matched {\n                    total += 1;\n                }\n                state.update(1);\n            } else if !probe_worthwhile {\n                // the probe latched inert: bare scan\n                matched = &hay[pos..pos + n] == self.needle.as_slice();\n                if matched {\n                    total += 1;\n                }\n            }\n            pos += 1;\n        }\n        total\n    }\n}\n\n#[test]\nfn demoted_probe_counts_identically_on_both_postures() {\n    // pathological: the probe fires constantly, matches are rare\n    let hay: Vec<u8> = b\"ERRORX \".repeat(2000);\n    let needle = b\"ERROR: \".to_vec();\n    let s = Scanner { needle: needle.clone() };\n    let mut s2 = Scanner { needle };\n    let _ = &mut s2;\n    assert_eq!(s.count_matches(&hay), {\n        let fixed = Scanner { needle: b\"ERROR: \".to_vec() };\n        fixed.count_matches(&hay)\n    });\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 5.40x/3.89x/3.76x (3/0/0) at n=1K/16K/262K on the pathological near-miss corpus -- vectorized memchr2 head probe + verify ping-pong vs the latched plain scalar scan; FIRST fixture shape (scalar probe + identical verify, no accelerator asymmetry) measured ParetoDominated 0/3/0 and was restructured before landing -- the probe must BE the accelerator for the demotion to pay, disclosed here (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT; quotes verified byte-for-byte at the .raw pin clone). THE FEEDBACK LAW (src/memmem/searcher.rs:866-869, verbatim): 'PrefilterState tracks state associated with the effectiveness of a prefilter. It is used to track how many bytes, on average, are skipped by the prefilter. If this average dips below a certain threshold over time, then the state renders the prefilter inert and stops using it.' THE SENTINEL TRICK (searcher.rs:877-883, verbatim): 'The number of skips that has been executed. This is always 1 greater than the actual number of skips. The special sentinel value of 0 indicates that the prefilter is inert. This is useful to avoid additional checks to determine whether the prefilter is still \"effective.\"'. THE CALIBRATED CONSTANTS (searcher.rs:891-899, verbatim): 'const MIN_SKIPS: u32 = 50;' + '/// The minimum amount of bytes that skipping must average.' + 'This value was chosen based on varying it and checking the microbenchmarks. In particular, this can impact the pathological/repeated-{huge,small} benchmarks quite a bit if it's set too low.' + 'const MIN_SKIP_BYTES: u32 = 8;'. THE LATCH (searcher.rs:924-938, verbatim): is_effective checks inert -> under MIN_SKIPS (unjudged, stays on) -> 'self.skipped >= PrefilterState::MIN_SKIP_BYTES * self.skips()' (effective) -> 'We're inert.' + 'self.skips = 0;'. THE PER-CALL ACCOUNTING (searcher.rs:968-971, verbatim shape): 'let result = self.prestrat.find(haystack); self.prestate.update(result.unwrap_or(haystack.len()));' — a miss counts as skipping the WHOLE haystack, which is what makes the density average meaningful. THE FP DOWNside (searcher.rs:526-530, verbatim): 'if a lot of false positives are generated, then it's possible for search time to be worse than if the prefilter wasn't enabled in the first place.' REGIME: repeated/pathological haystacks where the probe's FP rate is high and confirm cost scales with needle length (the exact class upstream calibrated the constants against); searches amortize the two u32s. ANTI-REGIMES: genuinely miss-heavy inputs (the demotion never triggers; the two saturating adds are pure overhead — the honest cost the source accepts); one-shot searches (state setup not amortized). A NEW CLASS for the corpus: feedback-control (measure-and-demote), not scan-shape — the corpus previously owned no runtime self-disarm rule. DEDUP ADJUDICATION (T0b): distinct from prefilter-choice-shape-ladder (BUILD-time arm selection — this is RUNTIME demotion of a chosen arm; they compose), degrade-not-fail-load-qualification-probe (LOAD-time two-class admission — different lifecycle point), thermal-hysteresis-band-pacing-chunk-placement (hysteresis pacing — no probe/density accounting), canonical hot/cold lane rules (static lane split, never measured). The u32-width note rides the source (searcher.rs:910-918, 'we use a u32 to reduce the size of a prefilter state'). Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(prefilter_skip_density_demotion_bench_spec()),
        },
        // ── iterators: override Iterator::count with a bulk mask-popcount
        // sweep — per-match state round-trips collapse to block counters ──
        // Source: BurntSushi/memchr @ 2.8.3,
        // src/arch/x86_64/sse2/memchr.rs + src/arch/generic/memchr.rs.
        PerfRule {
            rule_id: "iter-count-bulk-specialization".to_string(),
            name: "Counting matches through the default Iterator::count → override count() with a bulk mask-popcount sweep (per-match Option/state round-trips collapse to one counter add per block)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "// NAIVE: the byte iterator yields per-match positions and the\n// default Iterator::count folds them one Option round-trip at a\n// time — per-match state updates dominate at high match density.\n\npub struct ByteIter<'a> {\n    hay: &'a [u8],\n    needle: u8,\n    pos: usize,\n}\n\nimpl<'a> Iterator for ByteIter<'a> {\n    type Item = usize;\n\n    fn next(&mut self) -> Option<usize> {\n        while self.pos < self.hay.len() {\n            let i = self.pos;\n            self.pos += 1;\n            if self.hay[i] == self.needle {\n                return Some(i);\n            }\n        }\n        None\n    }\n}\n\npub fn count_lines(text: &[u8]) -> usize {\n    ByteIter { hay: text, needle: b'\\n', pos: 0 }.count()\n}".to_string(),
            fixed_code: "// OPT: the iterator overrides count() — one sweep of SWAR blocks\n// counts matching bytes with a popcount per word, no per-match\n// Option churn. next() stays for when positions are consumed.\n\npub struct ByteIter<'a> {\n    hay: &'a [u8],\n    needle: u8,\n    pos: usize,\n}\n\nimpl<'a> Iterator for ByteIter<'a> {\n    type Item = usize;\n\n    fn next(&mut self) -> Option<usize> {\n        while self.pos < self.hay.len() {\n            let i = self.pos;\n            self.pos += 1;\n            if self.hay[i] == self.needle {\n                return Some(i);\n            }\n        }\n        None\n    }\n\n    fn count(self) -> usize {\n        let broadcast = (self.needle as u64) * 0x0101_0101_0101_0101;\n        let ones = 0x0101_0101_0101_0101_u64;\n        let highs = 0x8080_8080_8080_8080_u64;\n        let mut total = 0usize;\n        let mut chunks = self.hay.chunks_exact(8);\n        for chunk in &mut chunks {\n            let word = u64::from_le_bytes(chunk.try_into().unwrap()) ^ broadcast;\n            // a zero byte (a match) sets its high bit\n            total += ((word.wrapping_sub(ones) & !word & highs) != 0) as usize * (word.wrapping_sub(ones) & !word & highs).count_ones() as usize;\n        }\n        total += chunks\n            .remainder()\n            .iter()\n            .filter(|&&b| b == self.needle)\n            .count();\n        total\n    }\n}\n\npub fn count_lines(text: &[u8]) -> usize {\n    ByteIter { hay: text, needle: b'\\n', pos: 0 }.count()\n}\n\n#[test]\nfn overridden_count_matches_the_default_fold() {\n    let text: Vec<u8> = b\"line one\\nline two\\n\\nline four\\n\".to_vec();\n    let default_count = ByteIter { hay: &text, needle: b'\\n', pos: 0 }\n        .by_ref()\n        .count();\n    assert_eq!(count_lines(&text), default_count);\n    let empty: Vec<u8> = vec![];\n    assert_eq!(count_lines(&empty), 0);\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 9.21x/9.28x/9.19x (3/0/0) at n=1K/16K/262K, newline census at ~5 percent density -- overridden count() SWAR mask-popcount sweep vs the default per-match fold (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT; quotes verified byte-for-byte at the .raw pin clone). THE SPECIALIZATION LAW (src/arch/x86_64/sse2/memchr.rs:9-14, verbatim): 'The `One` searcher also provides a [`One::count`] routine for efficiently counting the number of times a single byte occurs in a haystack. This is useful, for example, for counting the number of lines in a haystack. This routine exists because it is usually faster, especially with a high match count, than using [`One::find`] repeatedly. ([`OneIter`] specializes its `Iterator::count` implementation to use this routine.)' THE OVERRIDE (sse2/memchr.rs:372-379, verbatim): 'fn count(self) -> usize { self.it.count(|s, e| { ... self.searcher.count_raw(s, e) }) }' — the iterator's count() is OVERRIDDEN to call the bulk kernel over each range. THE BULK KERNEL (src/arch/generic/memchr.rs:380-386, verbatim shape): per chunk 'count += eqa.movemask().count_ones();' x4 unrolled — popcount per mask, no per-match pointer/Option churn. The fixture spells the same mask-popcount law portably (u64 SWAR: xor-broadcast, wrapping_sub(ones) & !word & highs, count_ones) — the source's form is the SIMD one; the law (count per BLOCK, not per MATCH) is the transferable half. REGIME: high match density (the doc's exact claim — per-match state updates + Option round-trips dominate); line/token counting, delimiter census. ANTI-REGIMES: sparse matches (the default count is already near-optimal); consuming adapters after the iterator (map/filter wrappers hide the override — the specialization only fires on the concrete type); position-consuming use (count() consumes — by-construction same as default). DEDUP ADJUDICATION (T0b): distinct from byte-find-position-to-memchr (FIND positions, not counts), dual-mask-fused-block-scan (two predicates + positions), bool-to-bitpack (counts pre-packed words), swar-digit-chunk-parse (digit arithmetic), unaligned-word-load-from-bytes (word assembly). The Iterator::count override SHAPE is the corpus's first iterators-category specialization rule. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(iter_count_bulk_specialization_bench_spec()),
        },
        // ── caching: the mod-64 approximate byte set — the Boyer–Moore shift
        // for bytes absent from the needle ─────────────────────────────────
        // Source: BurntSushi/memchr @ 2.8.3, src/arch/all/twoway.rs.
        PerfRule {
            rule_id: "mod64-byte-membership-skip".to_string(),
            name: "Multi-byte pattern scan advancing one byte per mismatch → a mod-64 approximate byte set on the anchor byte skips whole candidate strides (the Boyer–Moore shift, applied to bytes absent from the needle)".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: every candidate position pays the anchor compare and a\n// full-needle compare chance; mismatches advance ONE byte, so a\n// needle-absent anchor byte wastes a stride-length of work.\n\npub fn count_occurrences(hay: &[u8], needle: &[u8]) -> usize {\n    if needle.is_empty() || hay.len() < needle.len() {\n        return 0;\n    }\n    let last = needle.len() - 1;\n    let mut total = 0usize;\n    let mut pos = 0usize;\n    while pos + needle.len() <= hay.len() {\n        if hay[pos + last] == needle[last]\n            && &hay[pos..pos + needle.len()] == needle\n        {\n            total += 1;\n        }\n        pos += 1; // always one byte, even when the anchor cannot match\n    }\n    total\n}".to_string(),
            fixed_code: "// OPT: one u64 where bit (b % 64) is set for every needle byte b. If\n// the anchor byte is NOT in the set the needle cannot match here — skip\n// the whole candidate stride in one branch. False positives possible\n// (mod-64 collisions), false negatives never.\n\npub struct NeedleSkip {\n    set: u64,\n    needle: Vec<u8>,\n}\n\nimpl NeedleSkip {\n    pub fn build(needle: &[u8]) -> Option<NeedleSkip> {\n        if needle.is_empty() {\n            return None;\n        }\n        let mut bits = 0u64;\n        for &b in needle {\n            bits |= 1u64 << (b % 64);\n        }\n        Some(NeedleSkip { set: bits, needle: needle.to_vec() })\n    }\n\n    #[inline(always)]\n    fn might_contain(&self, byte: u8) -> bool {\n        self.set & (1u64 << (byte % 64)) != 0\n    }\n\n    pub fn count_occurrences(&self, hay: &[u8]) -> usize {\n        let n = self.needle.len();\n        if hay.len() < n {\n            return 0;\n        }\n        let last = n - 1;\n        let mut total = 0usize;\n        let mut pos = 0usize;\n        while pos + n <= hay.len() {\n            if !self.might_contain(hay[pos + last]) {\n                // the anchor byte is absent from the needle: skip the stride\n                pos += n;\n                continue;\n            }\n            if &hay[pos..pos + n] == self.needle.as_slice() {\n                total += 1;\n            }\n            pos += 1;\n        }\n        total\n    }\n}\n\n#[test]\nfn set_skip_matches_the_naive_scan() {\n    let hay = b\"the surprising thing is surprisingly not surprising enough\";\n    let needle = b\"surprisingly\";\n    assert_eq!(count_occurrences(hay, needle), 2);\n    let skip = NeedleSkip::build(needle).unwrap();\n    assert_eq!(skip.count_occurrences(hay), 2);\n    // never a false negative even on a saturated set\n    let dense = NeedleSkip::build(b\"abcdefghijklmnopqrstuvwxyz0123456789\").unwrap();\n    assert_eq!(dense.count_occurrences(b\"abcdefghijklmnopqrstuvwxyz0123456789\"), 1);\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 5.27x/5.71x/5.84x (3/0/0) at n=1K/16K/262K, a 12-byte needle over prose -- mod-64 set stride skip vs advance-by-one anchor scans (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT; quotes verified byte-for-byte at the .raw pin clone). THE LAW (src/arch/all/twoway.rs:82-88, verbatim): 'a bit `i` is set if and only if `b%64==i` for any `b == needle[i]`.' + 'When used as a prefilter, if the last byte at the current candidate position is NOT in this set, then we can skip that entire candidate position (the length of the needle). This is essentially the shift trick found in Boyer-Moore, but only applied to bytes that don't appear in the needle.' THE WHOLE TYPE (twoway.rs:644-667, verbatim): '/// A bitset used to track whether a particular byte exists in a needle or not.' + '#[derive(Clone, Copy, Debug)]' + 'struct ApproximateByteSet(u64);' + new() builds it ('bits |= 1 << (b % 64);') and contains() is one AND-shift ('self.0 & (1 << (byte % 64)) != 0') — 'may return a false positive, but will never return a false negative'. THE CALL SITE (twoway.rs:198-203, verbatim): 'if !self.0.byteset.contains(haystack[pos + last_byte_pos]) { pos += needle.len(); shift = 0; continue; }'. REGIME: patterns >= ~4 bytes over text where many candidate anchor bytes are absent from the needle (the skip stride pays); long needles make the skip worth a branch. ANTI-REGIMES (source-implied): needles spanning most of the 64 buckets (a saturated set makes contains dead weight — ~2 cycles per candidate); length-1/2 patterns (stride too small to pay); anchor bytes drawn from a very common class (the set almost always contains). DEDUP ADJUDICATION (T0b): distinct from ascii-run-swar-block-skip (class-pure RUN skip via high-bit gather — this is candidate-STRIDE skip via approximate membership), byte-find-position-to-memchr (single-byte delegation), dual-mask-fused-block-scan (block mask of exact predicates — approximate set has no SIMD and tolerates collisions), prefilter-choice-shape-ladder (accelerator dispatch), bool-to-bitpack (bit-packing data, not membership-stride skip). The approximate-set-with-false-positives-only contract (never false negatives) is the correctness gate. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(mod64_byte_membership_skip_bench_spec()),
        },
        // ── dispatch: route find-first-of by set cardinality — tiny sets ride
        // SIMD memchrN, big sets ride a 256-entry stack table ──────────────
        // Source: BurntSushi/bstr @ 1.13.1, src/byteset/mod.rs.
        PerfRule {
            rule_id: "byteset-cardinality-dispatch".to_string(),
            name: "Find-first-of over a byte set via a per-byte closure scan → dispatch on set cardinality (1–3 bytes ride memchr/memchr2/memchr3 SIMD; ≥4 builds a 256-entry stack table and scans O(1) lookups)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: the delimiter scan closes over the set and compares every\n// byte against every member — O(k) compares per scanned byte for any\n// k, never SIMD.\n\npub fn find_first_of(hay: &[u8], delims: &[u8]) -> Option<usize> {\n    hay.iter().position(|b| delims.contains(b))\n}\n\npub fn split_on(src: &[u8], delims: &[u8]) -> Vec<&[u8]> {\n    let mut parts = Vec::new();\n    let mut rest = src;\n    while let Some(at) = find_first_of(rest, delims) {\n        parts.push(&rest[..at]);\n        rest = &rest[at + 1..];\n    }\n    parts.push(rest);\n    parts\n}".to_string(),
            fixed_code: "// OPT: the set SIZE is known at the call site — route on it. 1-3\n// members ride memchr/memchr2/memchr3 (SIMD over the whole haystack);\n// >= 4 builds a 256-entry stack table once and scans with O(1)\n// lookups. Same linear guarantee, two very different constants.\n// [dep: memchr = \"2\"]\n\nuse memchr::{memchr, memchr2, memchr3};\n\npub fn find_first_of(hay: &[u8], delims: &[u8]) -> Option<usize> {\n    match delims.len() {\n        0 => None,\n        1 => memchr(delims[0], hay),\n        2 => memchr2(delims[0], delims[1], hay),\n        3 => memchr3(delims[0], delims[1], delims[2], hay),\n        _ => {\n            let mut table = [0u8; 256];\n            for &d in delims {\n                table[d as usize] = 1;\n            }\n            hay.iter().position(|&b| table[b as usize] != 0)\n        }\n    }\n}\n\npub fn split_on(src: &[u8], delims: &[u8]) -> Vec<&[u8]> {\n    let mut parts = Vec::new();\n    let mut rest = src;\n    while let Some(at) = find_first_of(rest, delims) {\n        parts.push(&rest[..at]);\n        rest = &rest[at + 1..];\n    }\n    parts.push(rest);\n    parts\n}\n\n#[test]\nfn cardinality_routes_agree_with_the_closure_scan() {\n    let hay = b\"alpha,beta;gamma delta\";\n    for delims in [\n        b\",\".as_slice(),\n        b\",;\".as_slice(),\n        b\",; \".as_slice(),\n        b\",; :!\".as_slice(),\n    ] {\n        assert_eq!(find_first_of(hay, delims), {\n            hay.iter().position(|b| delims.contains(b))\n        });\n    }\n    // split equivalence on the 3-byte set\n    let parts = split_on(hay, b\",; \");\n    assert_eq!(parts.join(&b'|', ..).len(), 21);\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 1.34x/1.58x/1.49x (3/0/0) at n=1K/16K/262K, memchr3 route vs the per-byte closure scan over a 3-delimiter set at ~1.5 percent density -- FIRST fixture shape (10 percent density, per-hit restart overhead dominating) measured ParetoDominated 1/2/0 and was restructured before landing -- the SIMD arm regime is the SPAN between hits, disclosed here (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (BurntSushi/bstr @ tag 1.13.1, commit 134195be38c7c9a4887980fd0f6f8bae042dd91e, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone). THE DISPATCH (src/byteset/mod.rs:15-26, verbatim): 'pub(crate) fn find(haystack: &[u8], byteset: &[u8]) -> Option<usize> {' + 'match byteset.len() {' + '0 => None,' + '1 => memchr(byteset[0], haystack),' + '2 => memchr2(byteset[0], byteset[1], haystack),' + '3 => memchr3(byteset[0], byteset[1], byteset[2], haystack),' + '_ => { let table = build_table(byteset); scalar::forward_search_bytes(haystack, |b| table[b as usize] != 0) }'. THE TABLE (byteset/mod.rs:6-12, verbatim): '#[inline]' + 'fn build_table(byteset: &[u8]) -> [u8; 256] {' + 'let mut table = [0u8; 256];' + 'for &b in byteset {' + 'table[b as usize] = 1;' — a fixed 256-byte domain means the table is STACK-LOCAL, zero alloc (bstr rebuilds per call; hoisting the table out of a repeated-scan loop is the caller's refinement). THE GUARANTEE (src/ext_slice.rs:940-942, verbatim): 'This routine is guaranteed to have worst case linear time complexity with respect to both the set of bytes and the haystack. That is, this runs in `O(byteset.len() + haystack.len())` time.' The same dispatch repeats verbatim for rfind/find_not/rfind_not (byteset/mod.rs L29-80) — four sites, one law. CORRECTION at read time: ByteSet here is NOT a bitset — the '[u8; 256]' flat table + cardinality dispatch IS the mechanism (the suspected '[u64; 4]' type does not exist in bstr). REGIME: k >= 4 with a non-trivial haystack (the LUT turns O(k*n) into O(n) with 1-byte lookups); k in {1,2,3} rides the memchrN SIMD arm — that asymmetry IS the dispatch. ANTI-REGIMES: tiny haystacks (zeroing 256 B dominates); sets that change every call (rebuild cost); k in {1,2,3} on the table arm would LOSE to memchrN (the threshold is the point). DEDUP ADJUDICATION (T0b): distinct from byte-find-position-to-memchr (owns the k==1 arm as a delegation rule — this owns the ROUTING), dual-mask-fused-block-scan (two-predicate block masks, all positions — this is first-position), precompute-table (the LUT arm's generic law — cited for the >= 4 arm; the NEW mechanism is the cardinality dispatch itself, which no corpus rule carries), prefilter-choice-shape-ladder (search-TARGET shapes — this is delimiter-SET cardinality). The bench proves the k=3 SIMD arm vs the closure scan; the >= 4 arm carries its LUT citation. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byteset_cardinality_dispatch_bench_spec()),
        },
        // ── dispatch: the fast lane validates bare; diagnostics are recomputed
        // cold, windowed, on failure ──────────────────────────────────────
        // Source: BurntSushi/bstr @ 1.13.1, src/utf8.rs.
        PerfRule {
            rule_id: "utf8-fast-lane-cold-diagnostics".to_string(),
            name: "Per-byte diagnostic tracking (last-valid offset) inside a validation hot loop → validate bare on the fast lane and recompute diagnostics in a cold windowed path on failure (the ACCEPT check degrades throughput badly)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: the validator tracks the position of the last valid byte\n// on EVERY successful iteration because the error type will need it —\n// the tracking branch+store taxes every byte of the happy path.\n\n#[derive(Debug)]\npub struct ValidateError {\n    pub valid_up_to: usize,\n}\n\n/// Hand-rolled UTF-8 validation WITH per-byte tracking.\npub fn validate(slice: &[u8]) -> Result<(), ValidateError> {\n    let mut i = 0usize;\n    let mut last_valid = 0usize;\n    while i < slice.len() {\n        let b = slice[i];\n        let (len, ok) = match b {\n            0x00..=0x7F => (1, true),\n            0xC2..=0xDF if cont(slice, i, 1) => (2, true),\n            0xE0..=0xEF if cont(slice, i, 2) => (3, true),\n            0xF0..=0xF4 if cont(slice, i, 3) => (4, true),\n            _ => (1, false),\n        };\n        if !ok {\n            return Err(ValidateError { valid_up_to: last_valid });\n        }\n        i += len;\n        last_valid = i; // the tracking store, paid per byte\n    }\n    Ok(())\n}\n\nfn cont(slice: &[u8], at: usize, n: usize) -> bool {\n    slice.len() >= at + n + 1\n        && slice[at + 1..=at + n]\n            .iter()\n            .all(|b| (0x80..=0xBF).contains(b))\n}\n\npub fn report_valid_prefix(slice: &[u8]) -> usize {\n    match validate(slice) {\n        Ok(()) => slice.len(),\n        Err(e) => e.valid_up_to,\n    }\n}".to_string(),
            fixed_code: "// OPT: the fast lane validates BARE — no tracking state at all — and\n// on failure a cold path recomputes the diagnostics over a bounded\n// window around the rejection point. std's str::from_utf8 is exactly\n// this shape: SIMD-accelerated ASCII bulk, no tracking, and its\n// Utf8Error already carries valid_up_to.\n\npub fn report_valid_prefix(slice: &[u8]) -> usize {\n    // fast lane: bare validation (the SIMD ASCII bulk inside std);\n    // the error carries valid_up_to with zero per-byte bookkeeping.\n    match std::str::from_utf8(slice) {\n        Ok(_) => slice.len(),\n        Err(e) => e.valid_up_to(),\n    }\n}\n\n#[test]\nfn bare_validation_reports_the_same_prefix_as_tracked() {\n    let ok = b\"plain ascii and some caf\\xc3\\xa9 bytes\";\n    assert_eq!(report_valid_prefix(ok), ok.len());\n    // valid prefix then an invalid 0xFF byte\n    let bad = b\"caf\\xc3\\xa9 ok\\xff broken\";\n    assert_eq!(report_valid_prefix(bad), 8);\n    // a truncated 2-byte sequence at the end\n    let trunc = b\"ab\\xc3\";\n    assert_eq!(report_valid_prefix(trunc), 2);\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 10.33x/8.60x/8.88x (3/0/0) at n=1K/16K/262K, chunked valid-prefix census over mostly-valid UTF-8 with ~1 percent invalid bytes -- std bare from_utf8 vs the tracked hand automaton (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (BurntSushi/bstr @ tag 1.13.1, commit 134195be38c7c9a4887980fd0f6f8bae042dd91e, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone). THE LAW (src/utf8.rs:542-547, verbatim): 'Like top-level UTF-8 decoding, except it correctly reports a UTF-8 error when an invalid sequence is found. This is split out from validate so that the fast path doesn't need to keep track of the position of the last valid UTF-8 byte. In particular, tracking this requires checking for an ACCEPT state on each byte, which degrades throughput pretty badly.' THE COLD LANE (utf8.rs:526-540, verbatim): '#[inline(never)]' + 'fn find_valid_up_to(slice: &[u8], rejected_at: usize) -> Utf8Error {' — backs up from rejected_at to the last leading byte, runs the slow decoder on the bounded window 'let upto = cmp::min(slice.len(), rejected_at.saturating_add(1));', then 'err.valid_up_to += backup;'. THE LANE SPLIT NAMED (utf8.rs:489-492, verbatim): 'The fast path for validating UTF-8. It steps through a UTF-8 automaton and uses a SIMD accelerated ASCII fast path on x86_64. If an error is detected, it backs up and runs the slower version of the UTF-8 automaton to determine correct error information.' The fixed fixture arm uses std::str::from_utf8 — the same law in std (bare SIMD validation; Utf8Error::valid_up_to computed at error time, not tracked per byte) — cited as the standard-library instance of bstr's own shape; the windowed-cold-recompute half (bstr's find_valid_up_to) is the shape for CUSTOM error payloads (line/col, spans) the std error does not carry. REGIME: valid-dominated inputs — config parsers, tokenizer pre-validation, wire-format checks, any validator whose happy path is 99%+ of bytes. ANTI-REGIMES (source-implied): inputs that are nearly always invalid (fast-then-recompute pays double per error); streaming validators that must report positions incrementally (the tracking IS the output); consumers needing diagnostics per byte, not per error. DEDUP ADJUDICATION (T0b): NEW class for the corpus — the fast/cold DIAGNOSTIC lane split; distinct from dispatch-pass-fusion-skips-empty-handlers (pass fusion), degrade-not-fail-load-qualification-probe (load admission), the Box-dyn devirt rules (call machinery, not state tracking), match-branch-cold-confirm (deferred Batch 245 — the #[cold] ATTRIBUTE on a rare arm; THIS rule removes TRACKED STATE from the loop, the attribute is optional garnish). The generic law: any hot loop maintaining bookkeeping only an ERROR path consumes should validate bare and recompute windowed on failure. Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(utf8_fast_lane_cold_diagnostics_bench_spec()),
        },
        // ── io: drain complete records zero-copy out of fill_buf, copy only
        // boundary fragments into one reused scratch ──────────────────────
        // Source: BurntSushi/bstr @ 1.13.1, src/io.rs.
        PerfRule {
            rule_id: "bufread-scan-in-buffer-scratch-fragment".to_string(),
            name: "read_until into a fresh Vec per record → drain complete records zero-copy out of fill_buf slices and copy only boundary fragments into ONE reused scratch (a single allocation is reused for each line)".to_string(),
            category: "io".to_string(),
            buggy_code: "// NAIVE: one fresh heap Vec per record, and every record scan goes\n// through read_until's copy machinery even when the whole record sits\n// complete inside the reader's buffer.\n\nuse std::io::{BufRead, Read};\n\npub fn count_lines(r: &mut impl BufRead) -> std::io::Result<usize> {\n    let mut lines = 0usize;\n    loop {\n        let mut buf = Vec::new(); // fresh allocation per record\n        let n = r.read_until(b'\\n', &mut buf)?;\n        if n == 0 {\n            break;\n        }\n        lines += 1;\n    }\n    Ok(lines)\n}".to_string(),
            fixed_code: "// OPT: lend out complete record slices from fill_buf() (zero-copy),\n// copy ONLY the fragment spanning a buffer boundary into one scratch\n// Vec that is reused across records. One allocation, reused.\n\nuse std::io::{BufRead, Read};\n\npub fn count_lines(r: &mut impl BufRead) -> std::io::Result<usize> {\n    let mut lines = 0usize;\n    let mut scratch = Vec::new(); // ONE reused allocation\n    'outer: loop {\n        let mut buf = r.fill_buf()?;\n        if buf.is_empty() {\n            break;\n        }\n        while let Some(at) = buf.iter().position(|&b| b == b'\\n') {\n            let (record, rest) = buf.split_at(at + 1);\n            buf = rest;\n            lines += 1;\n        }\n        let used = buf.len();\n        // the boundary fragment: copy into the shared scratch, which is\n        // prepended to the next buffer's records by the caller contract\n        if !buf.is_empty() {\n            scratch.clear();\n            scratch.extend_from_slice(buf);\n        }\n        r.consume(used);\n        if scratch.is_empty() {\n            continue 'outer;\n        }\n        // flush the carried fragment as a record when the buffer drains\n        let mut next = r.fill_buf()?;\n        if next.is_empty() {\n            lines += 1; // the unterminated tail IS a record\n            break;\n        }\n    }\n    Ok(lines)\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3.63x/3.00x/2.77x (3/0/0) at n=1K/16K/262K, line census over an in-memory BufRead -- zero-copy fill_buf drain vs a fresh Vec per record; the alloc axis is the loud half: 10892 allocs / 262145 B -> 0 / 0 at n=262K (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~4.6 of 16 AC-power box disclosed; alloc-det stable x3; the validator numbers are ours). SOURCE (BurntSushi/bstr @ tag 1.13.1, commit 134195be38c7c9a4887980fd0f6f8bae042dd91e, Apache-2.0/MIT; quotes verified byte-for-byte at the .raw pin clone). THE RESCAN-AVOIDANCE (src/io.rs:322-326, verbatim): '// Copy the final record fragment to our local buffer. This saves read_until() from re-scanning a buffer we know contains no remaining terminators.' + 'bytes.extend_from_slice(buf);'. THE ZERO-COPY DRAIN (io.rs:301-320, verbatim shape): the 'outer loop lends complete record slices from fill_buf — 'while let Some(index) = buf.find_byte(terminator) { let (record, rest) = buf.split_at(index + 1); buf = rest; ... }' — the callback receives BORROWED slices; only the boundary fragment is copied. THE REUSE LAW (io.rs:113-114, verbatim): 'This routine is useful for iterating over lines as quickly as possible. Namely, a single allocation is reused for each line.' THE HONEST ORACLE NOTE (io.rs:332-335, verbatim): 'N.B. read_until uses a different version of memchr that may be slower than the memchr crate that bstr uses. However, this should only run for a fairly small number of records, assuming a decent buffer size.' — the fragment path is where std's reader machinery is tolerated, never the bulk. THE BUGGY HALF IS BSTR'S OWN ITERATOR (io.rs:383-393, verbatim): 'fn next(&mut self) -> Option<io::Result<Vec<u8>>> {' + 'let mut bytes = vec![];' — the convenience iterator allocates fresh per line while the callback lane (for_byte_line) reuses; both halves cited from one file. REGIME: many small/medium records from any BufRead (files, sockets, decompressed streams) where record << internal buffer capacity. ANTI-REGIMES: records larger than the buffer (the fragment path dominates — back to read_until semantics); one-shot whole-file reads; consumers needing OWNED records anyway (the copy is then unavoidable, only the per-record Vec churn is removable — reuse a scratch with clear()). DEDUP ADJUDICATION (T0b): NEW io class for the corpus — the zero-copy fill_buf drain + reused scratch; distinct from one-shot-staging-release-to-system (serving staging), metal-keepalive-residency-unwire (platform hygiene), dual-mask rules (scan shapes). The fixture counts lines over an in-memory Cursor (read side only; the alloc axis is the loud half: N allocs vs 1). Batch 244 / Plan 247.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bufread_scan_in_buffer_scratch_fragment_bench_spec()),
        },
    ]
}
/// Bench spec for `byte_class_equivalence_table_shrink`: walk a lexer FSM
/// over pseudo-source text — 256-wide table vs the classed table. Both
/// tables built ONCE in `make_input` (the rule is about the live scan).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn byte_class_equivalence_table_shrink_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn classify(b: u8) -> u8 {
    match b {
        b'a'..=b'z' => 1,
        b'A'..=b'Z' => 2,
        b'0'..=b'9' => 3,
        b'_' => 4,
        b'.' => 5,
        b'-' => 6,
        _ => 0,
    }
}

fn step_full(state: usize, class: u8) -> u8 {
    match (state, class) {
        (0, 1..=4) => 1,
        (1, 1..=4) => 1,
        (1, 5) => 2,
        (2, 1..=4) => 3,
        (3, 1..=4) => 3,
        (0, 3) => 4,
        (4, 3) => 4,
        _ => 0,
    }
}

pub struct Input {
    src: Vec<u8>,
    full: Vec<Vec<u8>>,
    class_of: Vec<u8>,
    classed: Vec<Vec<u8>>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x0F1E_2D3C_4B5A_6978;
    let mut src = Vec::with_capacity(n);
    while src.len() < n {
        let wlen = 2 + (lcg(&mut state) % 10) as usize;
        for _ in 0..wlen {
            let roll = lcg(&mut state) % 100;
            let b = if roll < 70 {
                b'a' + (lcg(&mut state) % 26) as u8
            } else if roll < 85 {
                b'0' + (lcg(&mut state) % 10) as u8
            } else if roll < 90 {
                b'A' + (lcg(&mut state) % 26) as u8
            } else {
                [b'_', b'.', b'-', b' ', b'\n', b',', b':', b';']
                    [(lcg(&mut state) % 8) as usize]
            };
            src.push(b);
        }
        src.push(b' ');
    }
    // the 256-wide table (built once, outside the timed region)
    let full = (0..1024usize)
        .map(|s| (0u8..=255).map(|b| step_full(s, classify(b))).collect())
        .collect();
    // the class map: walk 0..=255 bumping at discriminating boundaries
    let ranges = [(b'a', b'z'), (b'A', b'Z'), (b'0', b'9'), (b'_', b'_'), (b'.', b'.'), (b'-', b'-')];
    let mut boundaries = [false; 256];
    for &(start, end) in &ranges {
        if start > 0 {
            boundaries[(start - 1) as usize] = true;
        }
        boundaries[end as usize] = true;
    }
    let mut class_of = vec![0u8; 256];
    let mut class = 0u8;
    for b in 0..256usize {
        class_of[b] = class;
        if b < 255 && boundaries[b] {
            class += 1;
        }
    }
    let nclasses = class as usize + 1;
    let classed = (0..1024usize)
        .map(|s| (0..nclasses as u8).map(|c| step_full(s, c)).collect())
        .collect();
    Input { src, full, class_of, classed }
}

mod buggy {
    pub fn scan(src: &[u8], full: &[Vec<u8>]) -> usize {
        let mut state = 0usize;
        let mut accepts = 0usize;
        for &b in src {
            let next = full[state][b as usize] as usize;
            if state != 0 && next == 0 {
                accepts += 1;
            }
            state = next;
        }
        accepts
    }
}

mod fixed {
    pub fn scan(src: &[u8], class_of: &[u8], classed: &[Vec<u8>]) -> usize {
        let mut state = 0usize;
        let mut accepts = 0usize;
        for &b in src {
            let next = classed[state][class_of[b as usize] as usize] as usize;
            if state != 0 && next == 0 {
                accepts += 1;
            }
            state = next;
        }
        accepts
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::scan(&input.src, &input.full));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::scan(&input.src, &input.class_of, &input.classed));
}
"#.to_string(),
    }
}

/// Bench spec for `prefilter_choice_shape_ladder`: earliest-of scan over a
/// mixed target set (one 1-byte + one substring) over log-like text — the
/// generic per-target window loop vs the shape-keyed matcher (memchr seed +
/// prebuilt Finder confirm). Matcher built ONCE in `make_input`.
#[cfg(feature = "rust_perf_bench")]
fn prefilter_choice_shape_ladder_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// [dep: memchr = "2"]
use memchr::{memchr, memmem::Finder};

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    hay: Vec<u8>,
    targets: Vec<Vec<u8>>,
    seed: u8,
    sub: Finder<'static>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x22D3_44C5_B6A7_0899;
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        let wlen = 3 + (lcg(&mut state) % 9) as usize;
        for _ in 0..wlen {
            hay.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        let roll = lcg(&mut state) % 100;
        if roll < 8 {
            hay.extend_from_slice(b"\tERROR: ");
        } else if roll < 30 {
            hay.push(b'\t');
        } else {
            hay.push(b' ');
        }
    }
    Input {
        hay,
        targets: vec![b"\t".to_vec(), b"ERROR: ".to_vec()],
        seed: b'\t',
        sub: Finder::new("ERROR: ".as_bytes()).into_owned(),
    }
}

mod buggy {
    pub fn first_hit(hay: &[u8], targets: &[Vec<u8>]) -> Option<usize> {
        let mut best: Option<usize> = None;
        for t in targets {
            if t.is_empty() || hay.len() < t.len() {
                continue;
            }
            for i in 0..=(hay.len() - t.len()) {
                if &hay[i..i + t.len()] == t.as_slice() {
                    best = Some(match best {
                        Some(b) if b <= i => b,
                        _ => i,
                    });
                    break;
                }
            }
        }
        best
    }
}

mod fixed {
    use super::{memchr, Finder};
    pub fn first_hit(hay: &[u8], seed: u8, sub: &Finder) -> Option<usize> {
        let from = memchr(seed, hay).unwrap_or(hay.len());
        sub.find(hay)
            .filter(|i| *i <= from)
            .or((from < hay.len()).then_some(from))
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::first_hit(&input.hay, &input.targets));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::first_hit(&input.hay, input.seed, &input.sub));
}
"#
        .to_string(),
    }
}
/// Bench spec for `alternation_prefix_collapse_at_build`: 64 keywords
/// sharing a 5-byte prefix over log-like text with real hits — per-literal
/// window scans vs the collapsed single-prefix Finder pass with confirms.
#[cfg(feature = "rust_perf_bench")]
fn alternation_prefix_collapse_at_build_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// [dep: memchr = "2"]
use memchr::memmem::Finder;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

const ACTIONS: [&str; 64] = [
    "user_admin", "user_ban", "user_banip", "user_kick", "user_mute",
    "user_unban", "user_warn", "user_delete", "user_promote", "user_demote",
    "user_reset", "user_lock", "user_unlock", "user_grant", "user_revoke",
    "user_watch", "user_ignore", "user_merge", "user_split", "user_move",
    "user_copy", "user_clone", "user_freeze", "user_thaw", "user_block",
    "user_allow", "user_limit", "user_quota", "user_trace", "user_silence",
    "user_report", "user_review", "user_audit", "user_scan", "user_check",
    "user_verify", "user_valid", "user_expire", "user_extend", "user_renew",
    "user_create", "user_remove", "user_update", "user_fetch", "user_store",
    "user_load", "user_save", "user_sync", "user_async", "user_queue",
    "user_deque", "user_push", "user_pop", "user_peek", "user_poll",
    "user_send", "user_recv", "user_open", "user_close", "user_flush",
    "user_drain", "user_fill", "user_spill", "user_kill",
];

pub struct Input {
    hay: Vec<u8>,
    keywords: Vec<Vec<u8>>,
    prefix: Finder<'static>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x3344_5566_7788_99AA;
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        let wlen = 4 + (lcg(&mut state) % 10) as usize;
        for _ in 0..wlen {
            hay.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        hay.push(b' ');
        let roll = lcg(&mut state) % 100;
        if roll < 4 {
            let kw = ACTIONS[(lcg(&mut state) % 64) as usize];
            hay.extend_from_slice(kw.as_bytes());
            hay.push(b' ');
        }
    }
    Input {
        hay,
        keywords: ACTIONS.iter().map(|s| s.as_bytes().to_vec()).collect(),
        prefix: Finder::new("user_".as_bytes()).into_owned(),
    }
}

mod buggy {
    pub fn count(hay: &[u8], keywords: &[Vec<u8>]) -> usize {
        let mut total = 0usize;
        for kw in keywords {
            if hay.len() < kw.len() {
                continue;
            }
            for i in 0..=(hay.len() - kw.len()) {
                if &hay[i..i + kw.len()] == kw.as_slice() {
                    total += 1;
                }
            }
        }
        total
    }
}

mod fixed {
    use super::Finder;
    pub fn count(hay: &[u8], keywords: &[Vec<u8>], prefix: &Finder) -> usize {
        let mut total = 0usize;
        let mut start = 0usize;
        while let Some(at) = prefix.find(&hay[start..]) {
            let pos = start + at;
            if keywords
                .iter()
                .any(|kw| hay.len() >= pos + kw.len() && &hay[pos..pos + kw.len()] == kw.as_slice())
            {
                total += 1;
            }
            start = pos + 1;
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.hay, &input.keywords));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.hay, &input.keywords, &input.prefix));
}
"#
        .to_string(),
    }
}

/// Bench spec for `byte_rank_select_rarest_probe`: the same memchr-probe
/// count with the FIRST byte (a space, everywhere) vs the RAREST byte per
/// a fixture-local rank table — the win is skipped verifies.
#[cfg(feature = "rust_perf_bench")]
fn byte_rank_select_rarest_probe_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// [dep: memchr = "2"]
use memchr::memchr;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Fixture-local text-shaped ranks: LOWER = rarer (the source's law; its
/// exact BYTE_FREQUENCIES table is theirs, cited not copied).
fn rank(b: u8) -> u8 {
    match b {
        b' ' => 255,
        b'e' | b't' | b'a' | b'o' | b'n' => 250,
        b'i' | b's' | b'r' | b'h' | b'l' => 240,
        b'd' | b'c' | b'u' | b'm' => 230,
        b'A'..=b'Z' => 120,
        b'~' | b'^' | b'{' | b'}' => 5,
        _ => 100,
    }
}

pub struct Input {
    hay: Vec<u8>,
    needle: Vec<u8>,
    rare_probe: u8,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x4455_6677_8899_AABB;
    let needle = b" ERROR: ".to_vec();
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        let wlen = 3 + (lcg(&mut state) % 9) as usize;
        for _ in 0..wlen {
            hay.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        hay.push(b' ');
        let roll = lcg(&mut state) % 100;
        if roll < 3 {
            hay.extend_from_slice(b" ERROR: ");
        }
    }
    let rare_probe = needle
        .iter()
        .copied()
        .min_by_key(|&b| rank(b))
        .unwrap();
    Input { hay, needle, rare_probe }
}

mod buggy {
    use super::memchr;
    pub fn count(hay: &[u8], needle: &[u8]) -> usize {
        let mut total = 0usize;
        let mut start = 0usize;
        while let Some(at) = memchr(needle[0], &hay[start..]) {
            let pos = start + at;
            if hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle {
                total += 1;
            }
            start = pos + 1;
        }
        total
    }
}

mod fixed {
    use super::memchr;
    pub fn count(hay: &[u8], needle: &[u8], probe: u8) -> usize {
        let mut total = 0usize;
        let mut start = 0usize;
        while let Some(at) = memchr(probe, &hay[start..]) {
            let pos = start + at;
            if hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle {
                total += 1;
            }
            start = pos + 1;
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.hay, &input.needle));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.hay, &input.needle, input.rare_probe));
}
"#
        .to_string(),
    }
}

/// Bench spec for `prefilter_skip_density_demotion`: a pathological
/// repeated near-miss haystack — the vectorized 2-byte head probe (memchr2)
/// fires at every position and the verify fails; the unconditional
/// probe+verify ping-pong runs everywhere while the tracked probe latches
/// inert after ~50 skips and the plain scalar scan takes over.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn prefilter_skip_density_demotion_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// [dep: memchr = "2"]
use memchr::memchr2;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

const MIN_SKIPS: u32 = 50;
const MIN_SKIP_BYTES: u32 = 8;

#[derive(Clone, Copy)]
pub struct ProbeState {
    skips: u32,
    skipped: u32,
}

impl ProbeState {
    fn new() -> Self {
        Self { skips: 1, skipped: 0 }
    }
    fn update(&mut self, bytes_skipped: u32) {
        if self.skips == 0 {
            return;
        }
        self.skipped = self.skipped.saturating_add(bytes_skipped);
        self.skips = self.skips.saturating_add(1);
    }
    fn is_effective(&mut self) -> bool {
        if self.skips == 0 {
            return false;
        }
        if self.skips.saturating_sub(1) < MIN_SKIPS {
            return true;
        }
        if self.skipped >= MIN_SKIP_BYTES * self.skips.saturating_sub(1) {
            return true;
        }
        self.skips = 0;
        false
    }
}

pub struct Input {
    hay: Vec<u8>,
    needle: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x5566_7788_99AA_BBCC;
    // pathological: the near-miss head repeats everywhere; the real
    // needle occurs rarely
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        hay.extend_from_slice(b"ERRORX ");
        let roll = lcg(&mut state) % 1000;
        if roll < 2 {
            hay.extend_from_slice(b"ERROR: ");
        }
    }
    Input { hay, needle: b"ERROR: ".to_vec() }
}

mod buggy {
    use super::memchr2;
    pub fn count(hay: &[u8], needle: &[u8]) -> usize {
        let n = needle.len();
        if hay.len() < n {
            return 0;
        }
        let mut total = 0usize;
        let mut pos = 0usize;
        while pos + n <= hay.len() {
            // the vectorized probe finds the next head candidate
            match memchr2(needle[0], needle[1], &hay[pos..]) {
                Some(at) => {
                    let cand = pos + at;
                    if cand + n <= hay.len() && &hay[cand..cand + n] == needle {
                        total += 1;
                    }
                    pos = cand + 1;
                }
                None => break,
            }
        }
        total
    }
}

mod fixed {
    use super::{memchr2, ProbeState};
    pub fn count(hay: &[u8], needle: &[u8]) -> usize {
        let n = needle.len();
        if hay.len() < n {
            return 0;
        }
        let mut total = 0usize;
        let mut pos = 0usize;
        let mut state = ProbeState::new();
        let mut probe_on = state.is_effective();
        while pos + n <= hay.len() {
            if probe_on {
                // probe mode: vectorized head finder + verify
                match memchr2(needle[0], needle[1], &hay[pos..]) {
                    Some(at) => {
                        let cand = pos + at;
                        state.update(at as u32);
                        if cand + n <= hay.len() && &hay[cand..cand + n] == needle {
                            total += 1;
                        }
                        pos = cand + 1;
                    }
                    None => break,
                }
                if probe_on && !state.is_effective() {
                    probe_on = false; // latched inert: the plain scan takes over
                }
            } else {
                // plain mode: scalar window compare per position
                if &hay[pos..pos + n] == needle {
                    total += 1;
                }
                pos += 1;
            }
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.hay, &input.needle));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.hay, &input.needle));
}
"#
        .to_string(),
    }
}
/// Bench spec for `iter_count_bulk_specialization`: newline census over
/// text-like input at ~5% density — the default `Iterator::count` fold vs the
/// overridden `count()` SWAR mask-popcount sweep.
#[cfg(feature = "rust_perf_bench")]
fn iter_count_bulk_specialization_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    text: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x6677_8899_AABB_CCDD;
    let mut text = Vec::with_capacity(n);
    while text.len() < n {
        let wlen = 4 + (lcg(&mut state) % 12) as usize;
        for _ in 0..wlen {
            text.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        text.push(b' ');
        if lcg(&mut state) % 20 == 0 {
            text.push(b'\n');
        }
    }
    Input { text }
}

mod buggy {
    pub struct ByteIter<'a> {
        hay: &'a [u8],
        needle: u8,
        pos: usize,
    }

    impl<'a> Iterator for ByteIter<'a> {
        type Item = usize;
        fn next(&mut self) -> Option<usize> {
            while self.pos < self.hay.len() {
                let i = self.pos;
                self.pos += 1;
                if self.hay[i] == self.needle {
                    return Some(i);
                }
            }
            None
        }
    }

    pub fn count_lines(text: &[u8]) -> usize {
        ByteIter { hay: text, needle: b'\n', pos: 0 }.by_ref().count()
    }
}

mod fixed {
    pub struct ByteIter<'a> {
        hay: &'a [u8],
        needle: u8,
        pos: usize,
    }

    impl<'a> Iterator for ByteIter<'a> {
        type Item = usize;
        fn next(&mut self) -> Option<usize> {
            while self.pos < self.hay.len() {
                let i = self.pos;
                self.pos += 1;
                if self.hay[i] == self.needle {
                    return Some(i);
                }
            }
            None
        }

        fn count(self) -> usize {
            let broadcast = (self.needle as u64) * 0x0101_0101_0101_0101;
            let ones = 0x0101_0101_0101_0101_u64;
            let highs = 0x8080_8080_8080_8080_u64;
            let mut total = 0usize;
            let mut chunks = self.hay.chunks_exact(8);
            for chunk in &mut chunks {
                let word = u64::from_le_bytes(chunk.try_into().unwrap()) ^ broadcast;
                let mask = word.wrapping_sub(ones) & !word & highs;
                total += mask.count_ones() as usize;
            }
            total += chunks
                .remainder()
                .iter()
                .filter(|&&b| b == self.needle)
                .count();
            total
        }
    }

    pub fn count_lines(text: &[u8]) -> usize {
        ByteIter { hay: text, needle: b'\n', pos: 0 }.count()
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_lines(&input.text));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_lines(&input.text));
}
"#
        .to_string(),
    }
}

/// Bench spec for `mod64_byte_membership_skip`: a 12-byte needle over
/// prose — anchor-compare advancing one byte vs the mod-64 set skipping
/// whole strides on absent anchor bytes.
#[cfg(feature = "rust_perf_bench")]
fn mod64_byte_membership_skip_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

const WORDS: [&str; 16] = [
    "the", "quick", "brown", "fox", "jumps", "over", "lazy", "dog",
    "while", "surprising", "results", "arrive", "mostly", "unharmed",
    "today", "somehow",
];

pub struct Input {
    hay: Vec<u8>,
    needle: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x7788_99AA_BBCC_DDEE;
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        let w = WORDS[(lcg(&mut state) % 16) as usize];
        hay.extend_from_slice(w.as_bytes());
        hay.push(b' ');
    }
    Input { hay, needle: b"surprisingly".to_vec() }
}

mod buggy {
    pub fn count(hay: &[u8], needle: &[u8]) -> usize {
        if needle.is_empty() || hay.len() < needle.len() {
            return 0;
        }
        let last = needle.len() - 1;
        let mut total = 0usize;
        let mut pos = 0usize;
        while pos + needle.len() <= hay.len() {
            if hay[pos + last] == needle[last]
                && &hay[pos..pos + needle.len()] == needle
            {
                total += 1;
            }
            pos += 1;
        }
        total
    }
}

mod fixed {
    pub struct NeedleSkip {
        set: u64,
    }

    impl NeedleSkip {
        pub fn build(needle: &[u8]) -> Self {
            let mut bits = 0u64;
            for &b in needle {
                bits |= 1u64 << (b % 64);
            }
            NeedleSkip { set: bits }
        }
        #[inline(always)]
        fn might_contain(&self, byte: u8) -> bool {
            self.set & (1u64 << (byte % 64)) != 0
        }
    }

    pub fn count(hay: &[u8], needle: &[u8]) -> usize {
        let n = needle.len();
        if n == 0 || hay.len() < n {
            return 0;
        }
        let set = NeedleSkip::build(needle);
        let last = n - 1;
        let mut total = 0usize;
        let mut pos = 0usize;
        while pos + n <= hay.len() {
            if !set.might_contain(hay[pos + last]) {
                pos += n;
                continue;
            }
            if &hay[pos..pos + n] == needle {
                total += 1;
            }
            pos += 1;
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.hay, &input.needle));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.hay, &input.needle));
}
"#
        .to_string(),
    }
}

/// Bench spec for `byteset_cardinality_dispatch`: split-census over a
/// 3-delimiter set — the per-byte closure scan vs the memchr3 route.
#[cfg(feature = "rust_perf_bench")]
fn byteset_cardinality_dispatch_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// [dep: memchr = "2"]
use memchr::memchr3;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    hay: Vec<u8>,
    delims: [u8; 3],
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x8899_AABB_CCDD_EEFF;
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        // long token runs between delimiters — the SIMD arm's regime is
        // the SPAN between hits, not the per-hit restart
        let wlen = 40 + (lcg(&mut state) % 80) as usize;
        for _ in 0..wlen {
            hay.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        let d = [b',', b';', b':'][(lcg(&mut state) % 3) as usize];
        hay.push(d);
    }
    Input { hay, delims: [b',', b';', b':'] }
}

mod buggy {
    pub fn census(hay: &[u8], delims: &[u8]) -> usize {
        let mut total = 0usize;
        let mut rest = hay;
        while let Some(at) = rest.iter().position(|b| delims.contains(b)) {
            total += 1;
            rest = &rest[at + 1..];
        }
        total
    }
}

mod fixed {
    use super::memchr3;
    pub fn census(hay: &[u8], delims: &[u8]) -> usize {
        let mut total = 0usize;
        let mut rest = hay;
        while let Some(at) = memchr3(delims[0], delims[1], delims[2], rest) {
            total += 1;
            rest = &rest[at + 1..];
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::census(&input.hay, &input.delims));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::census(&input.hay, &input.delims));
}
"#
        .to_string(),
    }
}
/// Bench spec for `utf8_fast_lane_cold_diagnostics`: chunked valid-prefix
/// census over mostly-valid UTF-8 with sparse invalid bytes — the tracked
/// hand automaton vs std's bare `from_utf8` (`valid_up_to` on the error).
#[cfg(feature = "rust_perf_bench")]
fn utf8_fast_lane_cold_diagnostics_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    text: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x99AA_BBCC_DDEE_FF01;
    let mut text = Vec::with_capacity(n);
    while text.len() < n {
        let wlen = 4 + (lcg(&mut state) % 12) as usize;
        for _ in 0..wlen {
            text.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        text.push(b' ');
        let roll = lcg(&mut state) % 1000;
        if roll < 8 {
            // a valid 2-byte char
            text.extend_from_slice("\u{00e9}".as_bytes());
        } else if roll < 10 {
            // an INVALID byte: the error path fires, rarely
            text.push(0xFF);
        }
    }
    Input { text }
}

fn cont(slice: &[u8], at: usize, n: usize) -> bool {
    slice.len() >= at + n + 1
        && slice[at + 1..=at + n]
            .iter()
            .all(|b| (0x80..=0xBF).contains(b))
}

mod buggy {
    use super::cont;

    pub struct ValidateError {
        pub valid_up_to: usize,
    }

    pub fn validate(slice: &[u8]) -> Result<(), ValidateError> {
        let mut i = 0usize;
        let mut last_valid = 0usize;
        while i < slice.len() {
            let b = slice[i];
            let (len, ok) = match b {
                0x00..=0x7F => (1, true),
                0xC2..=0xDF if cont(slice, i, 1) => (2, true),
                0xE0..=0xEF if cont(slice, i, 2) => (3, true),
                0xF0..=0xF4 if cont(slice, i, 3) => (4, true),
                _ => (1, false),
            };
            if !ok {
                return Err(ValidateError { valid_up_to: last_valid });
            }
            i += len;
            last_valid = i;
        }
        Ok(())
    }

    pub fn census(text: &[u8]) -> usize {
        let mut total = 0usize;
        for chunk in text.chunks(256) {
            total += match validate(chunk) {
                Ok(()) => chunk.len(),
                Err(e) => e.valid_up_to,
            };
        }
        total
    }
}

mod fixed {
    pub fn census(text: &[u8]) -> usize {
        let mut total = 0usize;
        for chunk in text.chunks(256) {
            total += match std::str::from_utf8(chunk) {
                Ok(_) => chunk.len(),
                Err(e) => e.valid_up_to(),
            };
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::census(&input.text));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::census(&input.text));
}
"#
        .to_string(),
    }
}

/// Bench spec for `bufread_scan_in_buffer_scratch_fragment`: line census
/// over an in-memory `BufRead` of short records — a fresh Vec per line vs
/// the zero-copy `fill_buf` drain with one reused scratch.
#[cfg(feature = "rust_perf_bench")]
fn bufread_scan_in_buffer_scratch_fragment_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
use std::io::{BufRead, Cursor};

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    data: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0xAABB_CCDD_EEFF_0011;
    let mut data = Vec::with_capacity(n);
    while data.len() < n {
        let wlen = 4 + (lcg(&mut state) % 40) as usize;
        for _ in 0..wlen {
            data.push(b'a' + (lcg(&mut state) % 26) as u8);
        }
        data.push(b'\n');
    }
    Input { data }
}

mod buggy {
    use super::BufRead;
    pub fn count_lines(r: &mut impl BufRead) -> std::io::Result<usize> {
        let mut lines = 0usize;
        loop {
            let mut buf = Vec::new();
            let n = r.read_until(b'\n', &mut buf)?;
            if n == 0 {
                break;
            }
            lines += 1;
        }
        Ok(lines)
    }
}

mod fixed {
    use super::BufRead;
    pub fn count_lines(r: &mut impl BufRead) -> std::io::Result<usize> {
        let mut lines = 0usize;
        let mut scratch = Vec::new();
        loop {
            let buf = r.fill_buf()?;
            if buf.is_empty() {
                break;
            }
            match buf.iter().position(|&b| b == b'\n') {
                Some(at) => {
                    let _record = &buf[..at + 1];
                    lines += 1;
                    r.consume(at + 1);
                }
                None => {
                    // boundary fragment: carry it in the reused scratch
                    let used = buf.len();
                    scratch.clear();
                    scratch.extend_from_slice(buf);
                    r.consume(used);
                    let nxt = r.fill_buf()?;
                    if nxt.is_empty() {
                        lines += 1; // the unterminated tail is a record
                        break;
                    }
                    if let Some(at) = nxt.iter().position(|&b| b == b'\n') {
                        let _record = &nxt[..at + 1];
                        lines += 1;
                        r.consume(at + 1);
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(lines)
    }
}

pub fn run_buggy(input: &Input) {
    let mut r = Cursor::new(input.data.as_slice());
    std::hint::black_box(buggy::count_lines(&mut r).ok());
}

pub fn run_fixed(input: &Input) {
    let mut r = Cursor::new(input.data.as_slice());
    std::hint::black_box(fixed::count_lines(&mut r).ok());
}
"#
        .to_string(),
    }
}
