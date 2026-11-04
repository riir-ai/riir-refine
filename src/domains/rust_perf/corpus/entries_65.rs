//! Entries slice — Batch 332 (2026-11-05): two `rust_perf` dispatch rules
//! from the blazing-smtlib distill (Plan 340 / Research 370).
//!
//! Source: onderjan/blazing-smtlib @ `42b45c20b0045495a40b9eaf0a717e0f1c976bed`
//! (MIT OR Apache-2.0, v0.1.0, HEAD 2026-10-29; paper provenance arXiv
//! 2611.00185 "Blazing-Fast SMT-LIB Parsing for Rust and Lean" — a NEW
//! library, no upstream port), quotes re-verified at the pin at plan-340
//! authoring time; the verbatim law quotes live in the rules'
//! `context_notes` (the doc-lazy-continuation class: quoted prose in `//!`
//! breaks the paragraph parse). Fixtures are ORIGINAL plain-Rust stand-ins
//! (a byte-class scanner pair + a token-batch handover pair), standalone
//! rustc-compiled AND executed before landing (rule 1: 3/3 — per-byte
//! class census 0..=255 + block-edge sweep 3/4/5/7/8/9/31/32/33 +
//! generated-corpus equivalence, release direction check 2.87x at n=262K;
//! rule 2: 1/1 — identical token streams + the ceil(tokens/256) dispatch
//! census). Batch 332 / Plan 340 / Research 370.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── dispatch: a 256-entry u8 class-flag table + a masked-prefix
        // run scan replaces the per-byte predicate chain ─────────────────
        // Source: blazing-smtlib rust/src/lexer/char_class.rs (run_flags,
        // RUN_FLAGS, run_end) + lexer/helpers.rs (the packed FAST
        // dispatch byte) + lexer.rs (lex_normal's fast path).
        PerfRule {
            rule_id: "byte-flag-table-masked-run-scan".to_string(),
            name: "A hand-rolled scanner that classifies EVERY byte through a predicate chain and walks its class runs one byte at a time → classify once per BYTE VALUE into a const-eval 256-entry u8 flag table and complete each class run with ONE masked-prefix scan (one table load + AND + branch per byte, a 4-byte-unrolled scalar loop; the token kind's run mask picks the class set, and a packed dispatch byte can fold the fast-path decision into the same table)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: a hand-rolled scanner classifies EVERY byte through a\n// predicate chain — several unpredictable branches per byte — and\n// re-walks each class run one byte at a time even inside a long\n// symbol or whitespace run (the shape behind hand-rolled lexers,\n// pre-tokenizers, and per-char predicate chains in serving\n// tokenizers).\n\nconst IS_SYMBOL: u8 = 1;\nconst IS_SPACE: u8 = 2;\n\nfn class_of(c: u8) -> u8 {\n    // the predicate chain: alnum + the symbol extras, then whitespace,\n    // then the fall-through — paid PER BYTE, forever\n    let mut cls = 0u8;\n    if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_' || c == b'+' {\n        cls |= IS_SYMBOL;\n    }\n    if c == b' ' || c == b'\\t' || c == b'\\n' || c == b'\\r' {\n        cls |= IS_SPACE;\n    }\n    cls\n}\n\n/// Token count: whitespace separates, symbol runs are one token,\n/// every other byte is a single-byte token.\npub fn count_tokens(bytes: &[u8]) -> usize {\n    let mut n = 0usize;\n    let mut i = 0usize;\n    while i < bytes.len() {\n        let cls = class_of(bytes[i]);\n        if cls & IS_SPACE != 0 {\n            i += 1; // the whitespace run is re-classified byte by byte\n        } else if cls & IS_SYMBOL != 0 {\n            let mut j = i + 1;\n            while j < bytes.len() && class_of(bytes[j]) & IS_SYMBOL != 0 {\n                j += 1; // the symbol run re-classifies every byte too\n            }\n            n += 1;\n            i = j;\n        } else {\n            n += 1;\n            i += 1;\n        }\n    }\n    n\n}".to_string(),
            fixed_code: "// OPT: classify once per BYTE VALUE into a const-eval 256-entry\n// u8 flag table (bit set = the byte may continue a run of that\n// class), and complete each class run with ONE masked-prefix scan —\n// position of the first byte carrying no flags from the mask, i.e.\n// the longest prefix where all bytes carry at least one flag. The\n// scan is a 4-byte-unrolled SCALAR loop: at lexer run lengths it\n// measured faster than an iterator position() AND than SWAR block\n// tricks (the source measured all three; the block-gather sibling\n// rule wins only LONG pure blocks — the two are regime-complementary,\n// never a merge).\n\nconst RUN_SYMBOL: u8 = 1 << 0;\nconst RUN_SPACE: u8 = 1 << 1;\n\n/// One byte per byte value; built at COMPILE time — zero runtime\n/// init cost. The flags partition the byte space: every byte carries\n/// at least one bit or none, and the mask picks the class set.\nstatic RUN_FLAGS: [u8; 256] = {\n    let mut t = [0u8; 256];\n    let mut i = 0usize;\n    while i < 256 {\n        let c = i as u8;\n        let mut f = 0u8;\n        if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_' || c == b'+' {\n            f |= RUN_SYMBOL;\n        }\n        if c == b' ' || c == b'\\t' || c == b'\\n' || c == b'\\r' {\n            f |= RUN_SPACE;\n        }\n        t[i] = f;\n        i += 1;\n    }\n    t\n};\n\n/// Length of the longest prefix of `bytes` where EVERY byte carries\n/// at least one flag from `mask` — the run's end. 4-byte unrolled;\n/// the tail (at most 3 bytes) is evaluated byte-wise.\nfn run_end(bytes: &[u8], mask: u8) -> usize {\n    let mut i = 0usize;\n    while let Some(&[a, b, c, d]) = bytes.get(i..i + 4) {\n        if RUN_FLAGS[a as usize] & mask == 0 { return i; }\n        if RUN_FLAGS[b as usize] & mask == 0 { return i + 1; }\n        if RUN_FLAGS[c as usize] & mask == 0 { return i + 2; }\n        if RUN_FLAGS[d as usize] & mask == 0 { return i + 3; }\n        i += 4;\n    }\n    let tail = &bytes[i..];\n    i + tail\n        .iter()\n        .position(|&c| RUN_FLAGS[c as usize] & mask == 0)\n        .unwrap_or(tail.len())\n}\n\npub fn count_tokens(bytes: &[u8]) -> usize {\n    let mut n = 0usize;\n    let mut i = 0usize;\n    while i < bytes.len() {\n        let f = RUN_FLAGS[bytes[i] as usize];\n        if f & RUN_SPACE != 0 {\n            i += run_end(&bytes[i..], RUN_SPACE); // the run in ONE scan\n        } else if f & RUN_SYMBOL != 0 {\n            i += run_end(&bytes[i..], RUN_SYMBOL);\n            n += 1;\n        } else {\n            n += 1;\n            i += 1;\n        }\n    }\n    n\n}\n\n#[test]\nfn masked_run_scan_matches_the_predicate_chain_reference() {\n    // the reference: the per-byte predicate chain, spelled independently\n    fn class_of(c: u8) -> u8 {\n        let mut cls = 0u8;\n        if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_' || c == b'+' {\n            cls |= 1;\n        }\n        if c == b' ' || c == b'\\t' || c == b'\\n' || c == b'\\r' {\n            cls |= 2;\n        }\n        cls\n    }\n    fn reference(bytes: &[u8]) -> usize {\n        let mut n = 0usize;\n        let mut i = 0usize;\n        while i < bytes.len() {\n            let cls = class_of(bytes[i]);\n            if cls & 2 != 0 { i += 1; }\n            else if cls & 1 != 0 {\n                let mut j = i + 1;\n                while j < bytes.len() && class_of(bytes[j]) & 1 != 0 { j += 1; }\n                n += 1;\n                i = j;\n            } else { n += 1; i += 1; }\n        }\n        n\n    }\n    // class-boundary census: the table must agree per single byte\n    for c in 0u8..=255 {\n        let bytes = [c];\n        assert_eq!(count_tokens(&bytes), reference(&bytes), \"byte {c}\");\n    }\n    // block-edge straddles over a pure symbol run\n    for len in [3usize, 4, 5, 7, 8, 9, 31, 32, 33] {\n        let bytes = vec![b'a'; len];\n        assert_eq!(count_tokens(&bytes), 1, \"len {len}\");\n    }\n    // mixed corpus + whitespace-run edges\n    let mixed = b\"(set-logic QF_BV)\\n(declare-fun x0 () (_ BitVec 8))\\n(add 123 -45 _+x.y)\\n\\t(check-sat)\";\n    assert_eq!(count_tokens(mixed), reference(mixed));\n    let ws = b\"abc   def\\n\\nghi\\r\\t j\";\n    assert_eq!(count_tokens(ws), reference(ws));\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-11-05, m3 QUIET, AC-power box disclosed per the box-state law; Batch 332 GOAT: 2.62x/2.94x/2.10x at n=1K/16K/262K, alloc 0/0 at every size, alloc-free by construction in both arms — the validator numbers are ours; the standalone rustc direction check agreed at 2.87x at n=262K rounds=200): the per-byte predicate chain re-classifies every byte of every run (~4-9 branches per byte whose short-circuit shape varies per byte: alnum is 2 range checks, the symbol extras add 4 equality tests, whitespace 4 more) and re-enters the walk per byte; the table+scan arm does one load + AND + branch per byte and completes a whole run per scan. SOURCE (onderjan/blazing-smtlib @ 42b45c20b0045495a40b9eaf0a717e0f1c976bed, MIT OR Apache-2.0, v0.1.0, HEAD 2026-10-29; paper provenance arXiv 2611.00185 Onderka/Biere/Boeving 2026-10-30 — a NEW library, no upstream port, the upstream is the SMT-LIB 2 standard itself; the >6x geomean vs previous parsing libraries is source-reported on their 9950X3D + 2025 SMT-LIB corpus sample — provenance, never bars; every quote re-verified at the .raw pin this batch). THE SHIPPED FORM (rust/src/lexer/char_class.rs, verbatim): the 8 run flags 'partition the byte into single bits' (RUN_SIMPLE_SYMBOL / RUN_DIGIT / RUN_HEX_DIGIT / RUN_BINARY_DIGIT / RUN_NON_LF_WHITESPACE / RUN_QUOTED_PLAIN / RUN_STRING_PLAIN / RUN_COMMENT_PLAIN), the table is const-eval ('static RUN_FLAGS: [u8; 256] = { ... table[i] = CharClass::run_flags(i as u8) ... }') with the compile-time coverage assert 'run flags must cover every bit of the byte'; the scan doc (verbatim): 'Position of the first byte carrying no flags from the mask, slice length if none. In other words, the longest prefix where all bytes carry at least one flag from the mask.'; the perf note (verbatim, char_class.rs:219): 'PERFORMANCE: Measured as superior to iter().position(), chunks, and SWAR.'; the hot-loop discipline (lexer.rs lex_normal, verbatim): 'This loop is VERY HOT and even small changes can drastically affect performance. This especially applies to unintentional panic paths introduced by maybe-panicking bounds checks.' — the scan uses slice.get(i..i+4) (no panicking index) and the 4-byte unroll is the measured shape. THE PACKED-DISPATCH FOLDED HALF (lexer/helpers.rs, verbatim): 'Fast dispatch bitmasks for every input byte.' — ONE u8 per byte value packs FAST_DELIM ('Set if the byte delimits a token: whitespace, a parenthesis, or the start of a comment. Every token must be followed by such a byte or by the end of the input.') + FAST_ON + FAST_EMIT + FAST_PIPE + the 3-bit FAST_RUN_MASK, const-eval with the masks-partition-u8 + coverage + agreement-with-START asserts; the hot loop 'is determined by one table lookup' ('PERFORMANCE: A fast path handles the most common token types. It is determined by one table lookup. If the started token is not on the fast path, a slower path is taken, again looked up. Arms eagerly try to complete the token with the whole content borrowed from the slice'); the fallback law: 'Lexing only returns early if eager dispatch is not possible, typically due to running out of bytes in the slice' — the state machine owns only slice overrun + unfinished/quoted tokens. REGIME: byte-oriented scanners, lexers, tokenizers, protocol delimiters — class-pure RUNS (identifiers, numbers, whitespace, comments) where the class is per-BYTE decidable; the win scales with run length (a run completes per scan, not per byte). ANTI-REGIMES: delimiter-dense input (every byte its own token — the source special-cases parens ahead of the table: 'The closing parenthesis is first as it forms more clusters'); classes NOT per-byte decidable (multi-byte UTF-8 class membership — classify per CHAR then, and a per-char class fn folded through a small table stays the bridge; the B243 sibling owns the Unicode side); one-shot short scans where a match chain is cheaper than any table indirection (the table is const-eval so there is no init cost, but a 5-byte input never amortizes the machinery's intent). TWO-REGIME CROSS-CITE (never merge): ascii-run-swar-block-skip (entries_29, B243) owns LONG PURE-CLASS blocks skipped by a high-bit gather — ONE class axis (ASCII vs not), SIMD-shaped; THIS owns SHORT MIXED-CLASS runs resolved by a masked table scan — MANY class axes (symbol/digit/hex/whitespace/quoted-plain/comment-plain), scalar-shaped, the mask picked per token kind; the source's own measurement is the bridge ('superior to ... SWAR' at lexer run lengths) — SWAR loses at SHORT runs because the block setup dominates. DEDUP ADJUDICATION (T0 re-run at the batch slot, 298-rule corpus): grep run_end|run flag|run mask|flag table|char class over src/domains/rust_perf/corpus/ — owners: ascii-run-swar-block-skip (ONE class, block skip), swar-digit-chunk-parse (SWAR ARITHMETIC on digit runs, the <8-digit tie regime), minimal-perfect-hash-static-lookup (hash lookup, not a dense byte index), dense-subdomain-arithmetic-before-table-lookup (arithmetic replacing a table, the inverse direction), loop-invariant-dispatch-resolution-hoist (hoisting resolution out of a loop) — ZERO owner of multi-class masked runs over a dense byte-indexed flag table with a scalar unrolled scan. LANE-INTEL (the verdict's mapping, Plan 340 T5 files the sibling issue via the owning lane): riir-infer src/tokenizer.rs:1514-1595 — the GPT-2-style split rules (Rules 1-7) walk per-CHAR Unicode predicate chains (chars[j].1.is_numeric() / .is_whitespace() / .is_alphabetic(), the bounded digit-run and whitespace-run walks) on the laya encode path; letters/digits/whitespace/punct are SEPARATE run classes with distinct predicates — exactly this rule's masked-run shape; B243 is the pure-ASCII sibling already on the books. Price via laya_fixture_timing before any change; league cells LOW (encode-side; decode WON per Bench 1155, prefill rides the Issue-1146 tensor stack). P113 upstream-state check: the mechanism is the repo's OWN landed decision (the shipped v0.1.0 form) — mineable decision content, never an outbound-PR candidate. Fixture: ORIGINAL plain-Rust scanner pair (never paste source), standalone rustc-compiled AND executed this batch (arms-agree-on-every-single-byte 0..=255 census + block-edge straddles 3/4/5/7/8/9/31/32/33 + generated SMT-script-shaped corpus equivalence at 1K/16K/262K; release direction check 2.87x at n=262K). Test pins: batch332 needles + the in-rule census/block-edge/mixed asserts. Source: onderjan/blazing-smtlib @ 42b45c20, read + quoted in Research 370. Batch 332 / Plan 340 / Research 370.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(byte_flag_table_masked_run_scan_bench_spec()),
        },
        // ── dispatch: a fixed-capacity staging batch + cursor bulk handover
        // amortizes the per-token consumer dispatch ─────────────────────────
        // Source: blazing-smtlib rust/src/lexer.rs (Batch, flush_batch,
        // flush_cursor) — BATCH_CAPACITY 256, zero-copy token views, the
        // Relex/Discard actions at the batch boundary.
        PerfRule {
            rule_id: "token-batch-cursor-handover".to_string(),
            name: "A producer that hands each item to its consumer one call at a time — re-entering the consumer and re-checking the stop action per item — → fill a FIXED-CAPACITY staging array of zero-copy views inside one producer call, then hand the whole batch over through a cursor the consumer pulls from, with the abort actions (continue / stop-after-consumed / discard) applying at the BATCH boundary only".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: the scanner hands each token to the driver ONE AT A\n// TIME — a dispatch call boundary per token, each re-entering the\n// consumer and re-checking the stop action, paid even when the\n// consumer takes the whole stream anyway.\n\n#[derive(Clone, Copy, PartialEq, Debug)]\npub enum Cat { Symbol, Digit, Space, Paren }\n\npub struct Counters {\n    pub dispatch_calls: usize,\n    pub stop_checks: usize,\n    pub tokens: usize,\n}\n\npub fn scan(bytes: &[u8], counters: &mut Counters) -> Vec<(Cat, usize)> {\n    let mut out = Vec::new();\n    let mut i = 0usize;\n    while i < bytes.len() {\n        let c = bytes[i];\n        let (cat, end) = if c == b'(' || c == b')' {\n            (Cat::Paren, i + 1)\n        } else if c.is_ascii_digit() {\n            let mut j = i + 1;\n            while j < bytes.len() && bytes[j].is_ascii_digit() { j += 1; }\n            (Cat::Digit, j)\n        } else if c == b' ' || c == b'\\t' || c == b'\\n' {\n            let mut j = i + 1;\n            while j < bytes.len()\n                && (bytes[j] == b' ' || bytes[j] == b'\\t' || bytes[j] == b'\\n')\n            {\n                j += 1;\n            }\n            (Cat::Space, j)\n        } else {\n            let mut j = i + 1;\n            while j < bytes.len() && bytes[j].is_ascii_alphanumeric() { j += 1; }\n            (Cat::Symbol, j)\n        };\n        // the per-token dispatch: the consumer is re-entered for every\n        // token and the stop action is re-checked for every token\n        counters.stop_checks += 1;\n        counters.dispatch_calls += 1;\n        out.push((cat, i));\n        counters.tokens += 1;\n        i = end;\n    }\n    out\n}".to_string(),
            fixed_code: "// OPT: fill a FIXED-CAPACITY staging array with zero-copy token\n// views inside one producer call (the scanner never returns to the\n// driver mid-batch), then hand the WHOLE batch to the consumer\n// through a bounded cursor it pulls from — one stop-check + one\n// dispatch per batch, and the abort action applies at the boundary,\n// never mid-batch.\n\npub const BATCH_CAPACITY: usize = 256; // maximal tokens per batch\n\n#[derive(Clone, Copy, PartialEq, Debug)]\npub enum Cat { Symbol, Digit, Space, Paren }\n\npub struct Counters {\n    pub dispatch_calls: usize,\n    pub stop_checks: usize,\n    pub tokens: usize,\n}\n\n/// Zero-copy token views staged in a fixed-capacity array.\ntype Slot = Option<(Cat, usize, usize)>; // (cat, start, end)\n\n/// What the consumer's action may ask at a BATCH boundary.\n#[derive(Clone, Copy, PartialEq)]\npub enum Action {\n    Continue,\n    Stop, // stop after the consumed prefix\n}\n\n/// A bounded cursor over one staged batch; the consumer pulls from it.\npub struct Cursor<'a> {\n    batch: &'a [Slot],\n    len: usize,\n    consumed: usize,\n}\n\nimpl<'a> Cursor<'a> {\n    pub fn peek(&self) -> Option<(Cat, usize, usize)> {\n        if self.consumed < self.len { self.batch[self.consumed] } else { None }\n    }\n    pub fn advance(&mut self) {\n        self.consumed += 1;\n    }\n}\n\nfn classify(bytes: &[u8], i: usize) -> (Cat, usize) {\n    let c = bytes[i];\n    if c == b'(' || c == b')' {\n        (Cat::Paren, i + 1)\n    } else if c.is_ascii_digit() {\n        let mut j = i + 1;\n        while j < bytes.len() && bytes[j].is_ascii_digit() { j += 1; }\n        (Cat::Digit, j)\n    } else if c == b' ' || c == b'\\t' || c == b'\\n' {\n        let mut j = i + 1;\n        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\\t' || bytes[j] == b'\\n') {\n            j += 1;\n        }\n        (Cat::Space, j)\n    } else {\n        let mut j = i + 1;\n        while j < bytes.len() && bytes[j].is_ascii_alphanumeric() { j += 1; }\n        (Cat::Symbol, j)\n    }\n}\n\npub fn scan(bytes: &[u8], counters: &mut Counters) -> Vec<(Cat, usize)> {\n    let mut out = Vec::new();\n    let mut i = 0usize;\n    while i < bytes.len() {\n        // fill the staging array — one producer call, zero-copy views\n        let mut entries: [Slot; BATCH_CAPACITY] = [None; BATCH_CAPACITY];\n        let mut len = 0usize;\n        while i < bytes.len() && len < BATCH_CAPACITY {\n            let (cat, end) = classify(bytes, i);\n            entries[len] = Some((cat, i, end));\n            len += 1;\n            i = end;\n        }\n        // ONE stop-check + one dispatch per BATCH: the abort action\n        // takes effect at the boundary, never mid-batch\n        counters.stop_checks += 1;\n        counters.dispatch_calls += 1;\n        let mut cursor = Cursor { batch: &entries, len, consumed: 0 };\n        let action = if counters.tokens < usize::MAX { Action::Continue } else { Action::Stop };\n        if action == Action::Stop {\n            break;\n        }\n        while let Some((cat, start, _end)) = cursor.peek() {\n            out.push((cat, start));\n            counters.tokens += 1;\n            cursor.advance();\n        }\n    }\n    out\n}\n\n#[test]\nfn batch_handover_matches_per_token_and_amortizes_the_boundaries() {\n    fn lcg(state: &mut u64) -> u64 {\n        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);\n        *state\n    }\n    fn make_bytes(n: usize) -> Vec<u8> {\n        let mut state: u64 = 0x5EED_B332_CAFE_F00D;\n        let mut bytes = Vec::with_capacity(n + 64);\n        while bytes.len() < n {\n            bytes.push(b'(');\n            bytes.extend_from_slice(b\"declare\");\n            bytes.push(b' ');\n            let ilen = 4 + (lcg(&mut state) % 12) as usize;\n            for _ in 0..ilen {\n                let r = lcg(&mut state) % 36;\n                let b = if r < 26 { b'a' + r as u8 } else { b'0' + (r - 26) as u8 };\n                bytes.push(b);\n            }\n            bytes.push(b' ');\n            let dlen = 1 + (lcg(&mut state) % 8) as usize;\n            for _ in 0..dlen {\n                bytes.push(b'0' + (lcg(&mut state) % 10) as u8);\n            }\n            bytes.extend_from_slice(b\")\\n\");\n        }\n        bytes\n    }\n    for n in [0usize, 1, 100, 255, 256, 257, 1_000, 16_384] {\n        let bytes = make_bytes(n);\n        let mut c1 = Counters { dispatch_calls: 0, stop_checks: 0, tokens: 0 };\n        let mut c2 = Counters { dispatch_calls: 0, stop_checks: 0, tokens: 0 };\n        let t1 = scan_per_token_reference(&bytes, &mut c1);\n        let t2 = scan(&bytes, &mut c2);\n        assert_eq!(t1, t2, \"token stream must be identical at n={n}\");\n        // the amortization law: per-token dispatch once per token;\n        // batched dispatch once per batch\n        let expected = c1.tokens.div_ceil(256).max(if c1.tokens == 0 { 0 } else { 1 });\n        assert_eq!(c2.dispatch_calls, expected, \"dispatch calls at n={n}\");\n        assert_eq!(c1.dispatch_calls, c1.tokens);\n    }\n}\n\n/// The per-token reference, spelled independently of the buggy arm.\nfn scan_per_token_reference(bytes: &[u8], counters: &mut Counters) -> Vec<(Cat, usize)> {\n    let mut out = Vec::new();\n    let mut i = 0usize;\n    while i < bytes.len() {\n        let (cat, end) = classify(bytes, i);\n        counters.dispatch_calls += 1;\n        out.push((cat, i));\n        counters.tokens += 1;\n        i = end;\n    }\n    out\n}".to_string(),
            context_notes: Some("Grade B — decision-content, suggestion-only, NotBenchable (the amortization magnitude depends on the CONSUMER's per-call cost — a census rule, not a clock rule; no Pareto spec: the fixture asserts the boundary COUNT law, deterministic and exact, and a timing bench of the boundary would measure the validator's own consumer stub). SOURCE (onderjan/blazing-smtlib @ 42b45c20b0045495a40b9eaf0a717e0f1c976bed, MIT OR Apache-2.0, v0.1.0, quotes re-verified at the .raw pin this batch; README verbatim): the lexer is 'optimised for speed, with a custom-written fast-path lexer, token batching, zero-copy tokens, and resource arenas'. THE SHIPPED FORM (rust/src/lexer.rs, verbatim): 'Maximal number of tokens that can be batched.' — const BATCH_CAPACITY: usize = 256; the batch is a fixed array of zero-copy views (struct Batch with entries: [Option<(InnerCategory, &slice)>; BATCH_CAPACITY] — tokens borrow the input slice, no per-token allocation); the handover is flush_batch to flush_cursor, and the consumer contract is PULL-based: consumer.consume(&mut cursor) drains through cursor methods with the CONSUMER's action (None / Relex / Discard) deciding the pipeline's next move; the actions apply at the BATCH boundary — 'the stop takes effect after the consumed tokens' and 'the batch is spent once handed over, no matter what the consumer decides' (the lexer writes its state back only at fallback boundaries, then returns the overrun span). THE WIN: the hot fill loop stays INSIDE one function (the batch push is a local array write), so the caller-level call boundary + the state save/restore happen once per BATCH or at a genuine fallback ('typically due to running out of bytes in the slice'), never per token; the consumer sees one cursor per batch instead of one re-entry per token, and its abort decision costs one check per batch. REGIME: pull-consumers over item streams (parse events, scanner tokens, telemetry records) where the consumer takes or skips the whole run anyway and the per-item dispatch (re-entry, action check, borrow re-establishment) is a measurable share; the fixed capacity bounds the staging memory AND bounds the waste past a mid-stream stop. ANTI-REGIMES: consumers that stop mid-stream OFTEN (every early stop strands unfilled staging — the source's answer is bounded waste + boundary-applied Relex/Discard, but a consumer that stops per item negates the amortization); observers needing PARTIAL mid-batch state (the B174 deferred-command caveat verbatim: hoist only where no observer needs partial mid-batch state); unbounded per-item consumer state (the staging array feeds a cursor, not a collection — do not accumulate the batch in a growing Vec, that re-introduces the per-item cost as per-item push). DISTINCT-FROM (the batching family, all four owners re-grepped at the batch slot): bufwriter-batched (entries_02 — SYSCALL batching: the barrier is a write to the OS), channel-send-batching-unpark (per-message WAKEUP amortization via a futex), deferred-command-flush-hoist-out-of-batch-loop (entries_08, B174 — the APPLICATION barrier: deferred-command application hoisted once-after-loop, and the no-partial-mid-batch-observer contract), batch-blocking-pool-segment (entries_06/Research 179 — BLOCKING-POOL coordination batching, the segment-size law); THIS amortizes the CONSUMER-DISPATCH boundary in a parse/producer loop with a PULL cursor + boundary-applied abort actions — no syscall, no wakeup, no barrier, no pool. fanout-transform-emit-callback-not-vec (entries_33) is the OPPOSITE pole (PUSH-callback emit to avoid collecting a Vec on the ALLOCATION axis; batching here stages a bounded array to amortize the CALL axis — different mechanism, different fix-space, and the staged array is fixed-capacity so it neither collects unbounded nor allocates per item). The source's own compare bench discipline (benches/compare.rs: build both revisions, run them ALTERNATELY file-by-file and round-by-round, median of per-round ratios) is an independent implementation of the workspace's sequential-AB law (katgpt-rs scripts/sequential_ab_timing_audit.py + tests/common/ab_timing.rs) — recorded in Research 370 as external CORROBORATION, never a corpus rule. P113 upstream-state: the batch is the repo's OWN landed design — decision content, never an outbound-PR candidate. Fixture: ORIGINAL plain-Rust pair (never paste source), standalone rustc-compiled AND executed this batch (identical token streams at n = 0/1/100/255/256/257/1000/16384 + the dispatch census: per-token arm = tokens calls, batched arm = ceil(tokens/256) calls + one stop-check per batch). Batch 332 / Plan 340 / Research 370.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// Bench spec for `byte-flag-table-masked-run-scan`: token-count over an
/// SMT-script-shaped byte stream (long identifier runs, numeric runs,
/// whitespace, parens, semicolon comments) — the per-byte predicate chain
/// vs the 256-entry flag table + masked run scan. Neither arm allocates:
/// the win is pure timing (the branch-count asymmetry + one scan per run).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)]
fn byte_flag_table_masked_run_scan_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub struct Input {
    bytes: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0xC0FF_EE12_3456_789A;
    let mut bytes = Vec::with_capacity(n + 64);
    while bytes.len() < n {
        bytes.push(b'(');
        bytes.extend_from_slice(b"declare-fun");
        bytes.push(b' ');
        let ilen = 8 + (lcg(&mut state) % 24) as usize;
        for _ in 0..ilen {
            let r = lcg(&mut state) % 62;
            let b = match r {
                0..=25 => b'a' + r as u8,
                26..=51 => b'A' + (r - 26) as u8,
                _ => b'0' + (r - 52) as u8,
            };
            bytes.push(b);
        }
        bytes.push(b' ');
        let dlen = 1 + (lcg(&mut state) % 12) as usize;
        for _ in 0..dlen {
            bytes.push(b'0' + (lcg(&mut state) % 10) as u8);
        }
        bytes.extend_from_slice(b") ()\n");
        if lcg(&mut state) % 8 == 0 {
            bytes.extend_from_slice(b"; a comment to the end of line\n");
        }
    }
    Input { bytes }
}

mod buggy {
    const IS_SYMBOL: u8 = 1;
    const IS_SPACE: u8 = 2;

    fn class_of(c: u8) -> u8 {
        let mut cls = 0u8;
        if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_' || c == b'+' {
            cls |= IS_SYMBOL;
        }
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
            cls |= IS_SPACE;
        }
        cls
    }

    pub fn count_tokens(bytes: &[u8]) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < bytes.len() {
            let cls = class_of(bytes[i]);
            if cls & IS_SPACE != 0 {
                i += 1;
            } else if cls & IS_SYMBOL != 0 {
                let mut j = i + 1;
                while j < bytes.len() && class_of(bytes[j]) & IS_SYMBOL != 0 {
                    j += 1;
                }
                n += 1;
                i = j;
            } else {
                n += 1;
                i += 1;
            }
        }
        n
    }
}

mod fixed {
    const RUN_SYMBOL: u8 = 1 << 0;
    const RUN_SPACE: u8 = 1 << 1;

    static RUN_FLAGS: [u8; 256] = {
        let mut t = [0u8; 256];
        let mut i = 0usize;
        while i < 256 {
            let c = i as u8;
            let mut f = 0u8;
            if c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_' || c == b'+' {
                f |= RUN_SYMBOL;
            }
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                f |= RUN_SPACE;
            }
            t[i] = f;
            i += 1;
        }
        t
    };

    fn run_end(bytes: &[u8], mask: u8) -> usize {
        let mut i = 0usize;
        while let Some(&[a, b, c, d]) = bytes.get(i..i + 4) {
            if RUN_FLAGS[a as usize] & mask == 0 { return i; }
            if RUN_FLAGS[b as usize] & mask == 0 { return i + 1; }
            if RUN_FLAGS[c as usize] & mask == 0 { return i + 2; }
            if RUN_FLAGS[d as usize] & mask == 0 { return i + 3; }
            i += 4;
        }
        let tail = &bytes[i..];
        i + tail
            .iter()
            .position(|&c| RUN_FLAGS[c as usize] & mask == 0)
            .unwrap_or(tail.len())
    }

    pub fn count_tokens(bytes: &[u8]) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < bytes.len() {
            let f = RUN_FLAGS[bytes[i] as usize];
            if f & RUN_SPACE != 0 {
                i += run_end(&bytes[i..], RUN_SPACE);
            } else if f & RUN_SYMBOL != 0 {
                i += run_end(&bytes[i..], RUN_SYMBOL);
                n += 1;
            } else {
                n += 1;
                i += 1;
            }
        }
        n
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_tokens(&input.bytes));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_tokens(&input.bytes));
}
"#
        .to_string(),
    }
}
