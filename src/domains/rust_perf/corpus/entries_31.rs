//! Entries slice 31 — Batch 245 (2026-10-16, Plan 248): the Batch-244 DEFER
//! LIST conversion — the eight staged leads from Research 276 adjudicated at
//! the same three release pins (`rust-lang/regex` @ tag
//! `regex-automata-0.4.18` commit `e6ad938198e68339a593f85d228f2d44cb54718f`,
//! Apache-2.0/MIT; `BurntSushi/memchr` @ tag `2.8.3` commit
//! `5fdb40c054e1fff359a2f7bdf7f87a13b34b465d`, Unlicense/MIT; `BurntSushi/bstr`
//! @ tag `1.13.1` commit `134195be38c7c9a4887980fd0f6f8bae042dd91e`,
//! Apache-2.0/MIT). Every quote in this slice re-verified byte-for-byte at the
//! `.raw` pin clones this session (the Batch 31 law); sources read by two
//! disjoint read subagents + coordinator re-verification of every landed
//! quote byte-for-byte. THE PREMISE CORRECTIONS the deep read found (both
//! carried into the rules): the interval-set guard lives in
//! `canonicalize`/`folded`/`union` — `push` at this pin is already sort-free
//! (`binary_search` + splice); the accel representation is `[u8; 8]`
//! (`ACCEL_CAP`), not the lead's guessed `[u8; 4]`.
//!
//! THE BATCH'S SHAPE: +6 rules (3 benchable + 3 NotBenchable-annotated) + 1
//! count-neutral rider (the char face on `byte-find-position-to-memchr`,
//! `entries_04`) + 1 DEFER confirmed (align-probe-overlap-tail — the align half
//! source-silent on modern µarchs, the tail half real but ≤V−1 scalar
//! iterations saved per call, offset-dependent; the citable atoms live in
//! Research 277). dispatch +2 (self-loop-state-memchr-skip,
//! match-branch-cold-confirm), simd +1 (pair-probe-confirm-fused), caching +1
//! (finder-hoist-reuse), complexity +1 (canonical-set-guarded-rebuild),
//! layout +1 (nonmax-sentinel-option-niche). Honest conversions where the
//! staged defers said the opposite: match-branch-cold-confirm's counter-case
//! is NARROW (two tiny-helper sites — the offset-arithmetic fn and a generic
//! match handler) while the repo's one `#[cold]` on the rare confirm memcmp
//! carries a benchmark-cited POSITIVE (rabinkarp.rs:350-355); a fixture
//! against that half-refuted attribute would be a coin flip, so the rule
//! lands `NotBenchable` suggest-only with the measurement-demand clause.
//! nonmax-sentinel-option-niche's "timing-only-at-scale" staging reason is
//! RELABELED: the source frames the payoff as MEMORY (the factor-2 claim is
//! source-stated); timing-at-scale is inference.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── dispatch: a DFA state that self-loops on all but a few rare
        // exit bytes is skipped by one memchr over the exits, not stepped
        // byte-at-a-time ──────────────────────────────────────────────────
        // Source: rust-lang/regex @ regex-automata-0.4.18,
        // regex-automata/src/dfa/accel.rs (+ dfa/search.rs, dfa/dense.rs).
        PerfRule {
            rule_id: "self-loop-state-memchr-skip".to_string(),
            name: "Stepping a state machine byte-at-a-time through a state that transitions to itself → skip the self-loop with one bulk search over the ≤3 exit bytes (and refuse the skip when an exit byte is common — acceleration then makes the search slower)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// NAIVE: the scanner steps the automaton one byte at a time\n// through every state — including the state that eats whole runs of\n// the haystack before anything interesting happens.\n\nconst EXIT: u8 = b'a'; // the only byte that leaves state 1\n\nstatic NEXT: [[u8; 256]; 3] = build_transitions();\n\n/// Counts non-overlapping runs of [^a]+a. State 1 self-loops on every\n/// byte except the exit byte.\npub fn count_runs(hay: &[u8]) -> usize {\n    let mut sid = 0usize;\n    let mut runs = 0usize;\n    let mut at = 0usize;\n    while at < hay.len() {\n        let next = NEXT[sid][hay[at] as usize] as usize;\n        sid = next;\n        at += 1;\n        if sid == 2 {\n            runs += 1;\n            sid = 0;\n        }\n    }\n    runs\n}\n\nfn build_transitions() -> [[u8; 256]; 3] {\n    let mut t = [[0u8; 256]; 3];\n    for b in 0..=255u8 {\n        // 0 --(non-exit)--> 1: entering a run\n        if b != EXIT {\n            t[0][b as usize] = 1;\n        }\n        // 1 --(non-exit)--> 1: the self-loop\n        // 1 --(exit)-----> 2: the run ends\n        t[1][b as usize] = u8::from(b == EXIT) * 2;\n        // 2 restarts like 0\n        if b != EXIT {\n            t[2][b as usize] = 1;\n        }\n    }\n    t\n}".to_string(),
            fixed_code: "// OPT: the self-loop state is the one place the automaton is\n// doing no work — every step re-reads the same transition row. When a\n// state's non-loop transitions are few and RARE, replace the whole run\n// with one bulk search for the exit bytes (regex-automata's\n// Accelerator; the b' ' refusal is the source's own admission gate).\n// [dep: memchr = \"2\"]\n\nuse memchr::memchr;\n\nconst EXIT: u8 = b'a';\n\nstatic NEXT: [[u8; 256]; 3] = build_transitions();\n\n/// The accel band: states 1..=1 carry an accelerator. The range check\n/// is paid on EVERY state visit — keep it one compare (regex-automata\n/// answers it from a contiguous accel id range).\nconst ACCEL_MIN: usize = 1;\nconst ACCEL_MAX: usize = 1;\n\npub fn count_runs(hay: &[u8]) -> usize {\n    let mut sid = 0usize;\n    let mut runs = 0usize;\n    let mut at = 0usize;\n    while at < hay.len() {\n        if (ACCEL_MIN..=ACCEL_MAX).contains(&sid) {\n            // One bulk skip over the self-loop. A miss does not end the\n            // search — the EOI transition still runs (the source sets\n            // `at` to the end and falls through for exactly that\n            // reason).\n            at = memchr(EXIT, &hay[at..]).map_or(hay.len(), |i| at + i + 1);\n            sid = 2; // the exit transition\n        } else {\n            let next = NEXT[sid][hay[at] as usize] as usize;\n            sid = next;\n            at += 1;\n        }\n        if sid == 2 {\n            runs += 1;\n            sid = 0;\n        }\n    }\n    runs\n}\n\nfn build_transitions() -> [[u8; 256]; 3] {\n    let mut t = [[0u8; 256]; 3];\n    for b in 0..=255u8 {\n        if b != EXIT {\n            t[0][b as usize] = 1;\n        }\n        t[1][b as usize] = u8::from(b == EXIT) * 2;\n        if b != EXIT {\n            t[2][b as usize] = 1;\n        }\n    }\n    t\n}\n\n/// Admission gate (the source's build-time law): an exit byte that is\n/// COMMON in the expected haystack refuses the accelerator — the skip\n/// then makes the search slower.\nfn accel_admissible(exit: u8) -> bool {\n    exit != b' '\n}\n\n#[test]\nfn accelerated_scan_agrees_with_the_byte_stepper() {\n    assert!(accel_admissible(b'a'));\n    assert!(!accel_admissible(b' '));\n    // identical counts across densities (the anti-regime cell is the\n    // dense one — the arm still agrees, it just stops winning)\n    for (seed, dense) in [(0x1111, 0.05_f64), (0x2222, 0.5)] {\n        let mut state: u64 = seed;\n        let mut lcg = || {\n            state = state\n                .wrapping_mul(6364136223846793005)\n                .wrapping_add(1442695040888963407);\n            state\n        };\n        let hay: Vec<u8> = (0..4096)\n            .map(|_| {\n                if lcg() % 100 < (dense * 100.0) as u64 {\n                    b'a'\n                } else {\n                    b'x'\n                }\n            })\n            .collect();\n        assert_eq!(count_runs(&hay), {\n            // the plain stepper, inlined for the oracle\n            let mut sid = 0usize;\n            let mut runs = 0usize;\n            for &b in &hay {\n                sid = NEXT[sid][b as usize] as usize;\n                if sid == 2 {\n                    runs += 1;\n                    sid = 0;\n                }\n            }\n            runs\n        });\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~10 of 16 AC-power box disclosed -- the sibling Warp-gate bevy build was live; both arms interleaved so the ratios stay fair; alloc-det stable x3): 6.35x/5.71x/5.51x at n=1K/16K/262K at exit-density 5% (first run 5.56x at max), alloc 0/0 -- the fixture carries BOTH the win regime (sparse exits: long non-exit runs, one memchr replaces ~20 state steps) and the anti-regime cells (50% density; the in-rule test asserts agreement at both). SOURCE (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; every quote verified byte-for-byte at the .raw/regex pin clone this session, checked out AT the release tag before any read). THE LAW (regex-automata/src/dfa/accel.rs:20-24, verbatim): 'In particular, state 3 is accelerated (shown via the A indicator) since the only way to leave that state once entered is to see an a byte. If there is a long run of non-a bytes, then using something like memchr to find the next a byte can be significantly faster than just using the standard byte-at-a-time state machine.' THE CAP (accel.rs:33-37, verbatim): 'In practice, we only consider accelerating states that have 3 or fewer non-loop transitions. At a certain point, you get diminishing returns, but also because that is what the memchr crate supports.' THE UNICODE ANTI-REGIME (accel.rs:26-31, verbatim): 'Unfortunately, this optimization rarely applies when Unicode is enabled.' (negated classes widen to UTF-8 sequences and the state stops being a simple self-loop). THE ADMISSION GATE (accel.rs:453-458, verbatim): 'As a special case, we totally reject trying to accelerate a state with an ASCII space. In most cases, it occurs very frequently, and tends to result in worse overall performance.' + 'if byte == b' ' { return false; }' THE VARIABLE-PERF HONESTY (dfa/dense.rs:112-115, verbatim): 'Callers may elect to disable this if consistent performance is more desirable than variable performance. Namely, acceleration can sometimes make searching slower than it otherwise would be if the transitions that leave accelerated states are traversed frequently.' THE SHIP POSTURE: always-on with a config demote (dense.rs:953-954 'self.accelerate.unwrap_or(true)'; Config::accelerate(bool) at dense.rs:120) -- a corpus analog: gate the skip behind a cheap PREDICATE, never behind a build flag. THE APPLY SITE (dfa/search.rs:158-163, verbatim): '} else if dfa.is_accel_state(sid) {' + 'let needles = dfa.accelerator(sid);' + 'at = accel::find_fwd(needles, input.haystack(), at + 1)' + '.unwrap_or(input.end());' + 'continue;' THE MISS-IS-NOT-QUIT NUANCE (search.rs:294-303, verbatim): 'If the accelerator returns nothing, why don't we quit the search? Well, if the accelerator doesn't find anything, that doesn't mean we don't have a match. It just means that we can't leave the current state given one of the 255 possible byte values. However, there might be an EOI transition.' (the miss jumps to end-of-input only when EOI is impossible... here: falls through to the end state). REPRESENTATION (accel.rs:390-400): 'pub(crate) struct Accel { bytes: [u8; ACCEL_CAP] }' with ACCEL_CAP = 8 (accel.rs:88) -- byte 0 is the length (<= 3), bytes 1..len the exit bytes, zero-padded 'as a slightly wasteful way of making sure alignment is always correct for state ID sizes of 1, 2, 4 and 8' (accel.rs:395-398); the reverse-DFA wrapper DISABLES it entirely for limited searches (meta/wrappers.rs:1266-1273). REGIME: automaton/scanner states that self-loop on all but <= 3 RARE exit bytes; long homogeneous runs between exits (log separators, whitespace runs, field delimiters). ANTI-REGIMES (all source-stated): common exit bytes (the b' ' refusal + the dense.rs variable-perf note -- the skip then fires constantly and the memchr call overhead ADDES to the step cost); Unicode negated classes (no simple exit set); reverse/limited searches (the wrapper disable); inter-exit runs below ~16-32B where the memchr call overhead eats the win (the byte-find-position-to-memchr wash note, same boundary). DEDUP ADJUDICATION (T0c): distinct from byte-find-position-to-memchr (the PRIMITIVE swap -- iter().position -> memchr; that rule's comment already cites dfa/accel.rs as provenance but owns no TRIGGER -- the distinct content here is the state-machine-level decision WHEN to abandon byte-stepping, keyed on automaton structure, with the <= 3 cap + the common-byte refusal), prefilter-choice-shape-ladder / prefilter-skip-density-demotion / mod64-byte-membership-skip / byteset-cardinality-dispatch (memchr/regex-automata INTERNAL scan mechanisms -- this is the engine-level consumer of those primitives), ascii-run-swar-block-skip (run-skip keyed on BYTE CLASS -- this is keyed on automaton STATE; the Batch-244 drop of sentinel-run-word-broadcast-skip for genus-sharing does not transfer). Fixture honesty (the three risks named at authoring): the buggy arm is the natural index-loop stepper (no strawman branches); the fixed arm PAYS the is_accel range check on every visit; the density ladder carries the anti-regime cell. HONEST BENCH RISK: at 5% density the inter-exit run averages 20 bytes -- if the validator measures the memchr call overhead dominating, the rule demotes/drops honestly (the Batch-243 precedent). Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(self_loop_state_memchr_skip_bench_spec()),
        },
        // ── simd: probe TWO needle bytes at two offsets in one pass — the
        // fused AND halves the false-positive candidates before the verify ──
        // Source: BurntSushi/memchr @ 2.8.3,
        // src/arch/generic/packedpair.rs (the generic fiddler).
        PerfRule {
            rule_id: "pair-probe-confirm-fused".to_string(),
            name: "Scanning for a needle by probing ONE of its bytes then verifying → probe TWO bytes at two fixed needle offsets in the same pass and AND the masks (a candidate now requires both bytes — ~p₁·p₂ false positives instead of ~p₁ — before the expensive full-needle verify)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// NAIVE: the scan is seeded by the needle's single rarest byte;\n// every hit pays the full-needle verify — including the hits that one\n// more probe byte would have killed for free.\n// [dep: memchr = \"2\"]\n\nuse memchr::memchr;\n\npub fn count(hay: &[u8], needle: &[u8]) -> usize {\n    if needle.len() < 2 || hay.len() < needle.len() {\n        return 0;\n    }\n    // the rarest single byte by the fixture-local rank table\n    let probe = *needle.iter().min_by_key(|&b| rank(b)).unwrap();\n    let mut total = 0usize;\n    let mut start = 0usize;\n    while let Some(at) = memchr(probe, &hay[start..]) {\n        let pos = start + at;\n        if hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle {\n            total += 1;\n        }\n        start = pos + 1;\n    }\n    total\n}\n\nfn rank(b: u8) -> u8 {\n    match b {\n        b'X' | b'Q' => 3,\n        b'a'..=b'z' | b'0'..=b'9' => 100,\n        b' ' => 150,\n        _ => 120,\n    }\n}".to_string(),
            fixed_code: "// OPT: probe TWO bytes at two fixed needle offsets in one pass over\n// the haystack: two equality masks (bytes == A, bytes == B at +delta)\n// AND into one candidate mask — a candidate now requires BOTH bytes, so\n// the expensive verify runs on ~p1*p2 of the single-probe candidates.\n// The offsets are the needle's two rarest bytes (selection owned by\n// byte-rank-select-rarest-probe; this rule owns the fused CONFIRM).\n// [dep: memchr = \"2\"]\n\nuse memchr::memchr;\n\npub struct PairProbe {\n    needle: Vec<u8>,\n    idx1: usize,\n    idx2: usize,\n}\n\nimpl PairProbe {\n    /// None = the needle is too short for two distinct offsets.\n    pub fn build(needle: &[u8]) -> Option<PairProbe> {\n        if needle.len() < 2 {\n            return None;\n        }\n        // two distinct offsets: the rarest byte and the rarest byte at a\n        // DIFFERENT index (the source asserts index1 != index2 -- equal\n        // offsets reduce the candidate filter to a single byte)\n        let order: Vec<usize> = (0..needle.len()).collect();\n        let mut sorted = order;\n        sorted.sort_by_key(|&i| rank(needle[i]));\n        let idx1 = sorted[0];\n        let idx2 = *sorted.iter().find(|&&i| i != idx1)?;\n        Some(PairProbe { needle: needle.to_vec(), idx1, idx2 })\n    }\n\n    pub fn count(&self, hay: &[u8]) -> usize {\n        let n = &self.needle;\n        if hay.len() < n.len() {\n            return 0;\n        }\n        let (a, b) = (n[self.idx1], n[self.idx2]);\n        let delta = self.idx2.abs_diff(self.idx1);\n        let (lo_off, hi_off) = (self.idx1.min(self.idx2), self.idx1.max(self.idx2));\n        let (lo, hi) = (n[lo_off], n[hi_off]);\n        let mut total = 0usize;\n        let end = hay.len() - n.len();\n        let mut base = 0usize;\n        // 8-byte SWAR body: two unaligned u64 loads at base+lo_off and\n        // base+hi_off, per-byte equality, AND. Byte j of the fused mask\n        // = candidate at base+j (the second load covers base+j+delta for\n        // every j -- no shift needed, both loads anchor at base).\n        while base + 8 + delta <= hay.len() && base + 8 <= end + 8 {\n            let w1 = u64::from_le_bytes(hay[base + lo_off..base + lo_off + 8].try_into().unwrap());\n            let w2 = u64::from_le_bytes(hay[base + hi_off..base + hi_off + 8].try_into().unwrap());\n            let m1 = byte_eq_mask(w1, lo);\n            let m2 = byte_eq_mask(w2, hi);\n            let mut bits = m1 & m2; // flags sit at bit 8j+7\n            while bits != 0 {\n                let j = bits.trailing_zeros() as usize / 8; // byte index from the bit-8j+7 flag\n                let pos = base + j;\n                if pos <= end && &hay[pos..pos + n.len()] == n.as_slice() {\n                    total += 1;\n                }\n                bits &= bits - 1;\n            }\n            base += 8;\n        }\n        // scalar tail (below min_haystack_len -- the source asserts\n        // haystack >= max(offsets)+V::BYTES in the body; the tail is the\n        // scalar path BOTH regimes share)\n        while base <= end {\n            if hay[base + lo_off] == lo && hay[base + hi_off] == hi {\n                if &hay[base..base + n.len()] == n.as_slice() {\n                    total += 1;\n                }\n            }\n            base += 1;\n        }\n        total\n    }\n}\n\n/// Per-byte equality bitmap of a u64 word (bit j = byte j == want):\n/// XOR to zero-fold, then the standard haszero trick per lane.\nfn byte_eq_mask(word: u64, want: u8) -> u64 {\n    let x = word ^ (u64::from(want) * 0x0101_0101_0101_0101);\n    let has = (x.wrapping_sub(0x0101_0101_0101_0101) & !x) & 0x8080_8080_8080_8080;\n    has\n}\n\n// single-byte fallback kept for parity with the buggy arm's rank law\nfn rank(b: u8) -> u8 {\n    match b {\n        b'X' | b'Q' => 3,\n        b'a'..=b'z' | b'0'..=b'9' => 100,\n        b' ' => 150,\n        _ => 120,\n    }\n}\n\n#[test]\nfn fused_pair_probe_agrees_with_single_probe_at_every_position() {\n    let hay: Vec<u8> = b\"xxXx QxxXQ xXQx XQ XQzz xQxX QXXq xXQz\".to_vec();\n    let needle = b\"XQz\".to_vec();\n    let pp = PairProbe::build(&needle).unwrap();\n    assert_eq!(pp.count(&hay), count(&hay, &needle));\n    // a needle whose two rarest bytes sit at ONE index degrades cleanly\n    assert!(PairProbe::build(&[b'X']).is_none());\n    // empty/short needles refuse the build\n    assert!(PairProbe::build(&[]).is_none());\n    // the fused mask kills a single-byte false positive: X alone matches\n    // at many positions, X..z does not\n    let mut x_only = 0usize;\n    let mut start = 0usize;\n    while let Some(at) = memchr(b'X', &hay[start..]) {\n        x_only += 1;\n        start += at + 1;\n    }\n    assert!(x_only >= pp.count(&hay));\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~10 of 16 AC-power disclosed; alloc-det stable x3): 8.82x/8.14x/9.91x at n=1K/16K/262K in the COMMON-BYTE regime -- the fixture was RESTRUCTURED ONCE per the discipline: the FIRST shape (planted rare singles X/Q ~2.7% each, jointly-rare pair) measured ParetoDominated 0/3/0 -- with a rare single byte available the single-probe arm already wins; the source's law lives where NO rare byte exists (seven symbols ~14% each, even the needle's rarest byte is common, pair ~2%) -- restructured to that regime and re-measured Dominant. PARITY INSTRUMENT: a standalone brute-force-oracle harness (both arms vs oracle at 3 sizes) caught a REAL fixture defect the timing-only validator cannot see -- the SWAR flag sits at bit 8j+7, and the first extraction treated trailing_zeros as the byte index (candidates anchored at base+8j+7, the fixed arm verified at garbage positions and SKIPPED nearly all verifies); fixed to tz/8 -- the earlier 8.9x pre-fix read was an artifact of missing verifies, the honest post-fix read is 8.1-9.9x WITH parity proven. needles whose second byte adds no discrimination measure a TIE (disclosed regime: joint rarity x verify cost is the win). SOURCE (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT; every quote verified byte-for-byte at the .raw/memchr pin clone this session, checked out AT the release tag before any read). THE SHAPE (src/arch/generic/packedpair.rs:236-243, verbatim): 'let index1 = usize::from(self.pair.index1());' + 'let index2 = usize::from(self.pair.index2());' + 'let chunk1 = V::load_unaligned(cur.add(index1));' + 'let chunk2 = V::load_unaligned(cur.add(index2));' + 'let eq1 = chunk1.cmpeq(self.v1);' + 'let eq2 = chunk2.cmpeq(self.v2);' + 'let mut offsets = eq1.and(eq2).movemask().and(mask);' -- the PREFILTER twin at 274-277: 'let offsets = eq1.and(eq2).movemask();' (same AND, no extra mask). THE INTENT (packedpair.rs:19-23, verbatim): 'This finder picks two bytes that it believes have high predictive power for indicating an overall match of a needle. Depending on whether Finder::find or Finder::find_prefilter is used, it reports offsets where the needle matches or could match. In the prefilter case, candidates are reported whenever the Pair of bytes given matches.' THE DISTINCT-INDICES ASSERT (arch/all/packedpair/mod.rs:191-195, verbatim): 'While not strictly required for how a Pair is normally used, we really don't want these to be equivalent. If they were, it would reduce the effectiveness of candidate searching using these rare bytes by increasing the rate of false positives.' + 'assert_ne!(index1, index2);' THE REGIME BOUNDS (src/memmem/searcher.rs:995-998, verbatim): 'The minimum length of a needle required for this algorithm. The minimum is 2 since a length of 1 should just use memchr and a length of 0 isn't a case handled by this searcher.' + 'const MIN_LEN: usize = 2;' and (searcher.rs:1019-1026, verbatim): 'This specific number was chosen by tweaking a bit and running benchmarks. The rare-medium-needle, for example, gets about 5% faster by using this algorithm instead of a prefilter-accelerated Two-Way. There's also a theoretical desire to keep this number reasonably low, to mitigate the impact of pathological cases. I did try 64, and some benchmarks got a little better, and others (particularly the pathological ones), got a lot worse. So... 32 it is?' + 'const MAX_LEN: usize = 32;' THE MIN-HAYSTACK GUARD (generic/packedpair.rs:55-57, verbatim): 'let max_index = pair.index1().max(pair.index2());' + 'let min_haystack_len =' + 'core::cmp::max(needle.len(), usize::from(max_index) + V::BYTES);' (the fixture's body loop respects it; the tail is the shared scalar path). REGIME: needles 2..=32 bytes whose two rarest bytes are jointly rarer than either alone (log markers, protocol framing, structured text); haystacks at/above min_haystack_len; verifies expensive enough that candidate reduction pays. ANTI-REGIMES (source-stated + labeled inference): needles whose pair bytes sit at adjacent/identical contexts with correlated appearances (the AND filters nothing); 1-byte needles (memchr's own ladder); haystacks below min_haystack_len (shape panics -- the fixture routes the tail to the shared scalar path); verification so cheap the FP cost never materializes (short needles). DEDUP ADJUDICATION (T0c): distinct from byte-rank-select-rarest-probe (owns the rare-byte SELECTION law -- this rule OWNS the fused conjunction CONFIRM; cite, don't re-own; its fixed arm probes ONE byte), dual-mask-fused-block-scan (OR-s two INDEPENDENT predicates over all positions to answer two questions -- this AND-s two cmpeq of the SAME needle at two offsets to answer ONE question with fewer false positives: conjunction vs disjunction, same-needle vs cross-question; the Batch-244 defer's 'needs its own pass' adjudication satisfied here), byte-find-position-to-memchr (crate delegation vs the hand-rolled probe shape -- packedpair is crate-internal, not publicly callable, hence the portable SWAR spelling of the same law), prefilter-skip-density-demotion / prefilter-choice-shape-ladder (orthogonal layers around the same scan). HONEST BENCH RISK (the Batch-243 drop class, flagged at authoring): a needle whose second-rarest byte is ~as common as the first measures Dominated; the fixture plants jointly-rare bytes with individually-tolerable rates and discloses the tie regime. The SWAR spelling is the portable form of the source's SIMD -- the u64 loads lower to single unaligned loads; the [dep: memchr] header is the buggy arm's probe only. Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(pair_probe_confirm_fused_bench_spec()),
        },
        // ── caching: a substring searcher's construction is real work —
        // build the Finder once per needle, reuse across haystacks ────────
        // Sources: BurntSushi/memchr @ 2.8.3 (src/memmem/mod.rs) +
        // BurntSushi/bstr @ 1.13.1 (src/ext_slice.rs) — ONE law in two
        // repos (the byte-rank-select-rarest-probe merge precedent).
        PerfRule {
            rule_id: "finder-hoist-reuse".to_string(),
            name: "Rebuilding the substring searcher per haystack (find()/Finder::new in the loop) → construct the Finder once per needle and reuse it across haystacks (construction is a frequency-rank scan + Rabin-Karp table + prefilter/pair selection — O(needle) real work)".to_string(),
            category: "caching".to_string(),
            buggy_code: "// NAIVE: the same needle is searched across many haystacks, but the\n// searcher is rebuilt every call — the two-way machinery, the Rabin-Karp\n// table and the prefilter selection all re-run per line.\n// [dep: memchr = \"2\"]\n\nuse memchr::memmem;\n\npub fn count_marker(lines: &[Vec<u8>], needle: &[u8]) -> usize {\n    let mut total = 0usize;\n    for line in lines {\n        // per-call construction: rank scan + table + prefilter selection\n        if memmem::find(line, needle).is_some() {\n            total += 1;\n        }\n    }\n    total\n}".to_string(),
            fixed_code: "// OPT: construct ONCE per needle (the docs' own law: when the same\n// needle searches many haystacks, do the construction once and avoid it\n// for subsequent searches), reuse the Finder per haystack.\n// [dep: memchr = \"2\"]\n\nuse memchr::memmem::Finder;\n\npub struct MarkerCounter {\n    finder: Finder<'static>,\n}\n\nimpl MarkerCounter {\n    pub fn build(needle: &[u8]) -> MarkerCounter {\n        MarkerCounter { finder: Finder::new(needle.to_vec()).into_owned() }\n    }\n\n    pub fn count_marker(&self, lines: &[Vec<u8>]) -> usize {\n        let mut total = 0usize;\n        for line in lines {\n            if self.finder.find(line).is_some() {\n                total += 1;\n            }\n        }\n        total\n    }\n}\n\n#[test]\nfn hoisted_finder_counts_identically() {\n    let lines: Vec<Vec<u8>> = [\"WARN disk slow\", \"ok\", \"WARN again\", \"fine\"]\n        .iter()\n        .map(|s| s.as_bytes().to_vec())\n        .collect();\n    let needle = b\"WARN\";\n    assert_eq!(count_marker(&lines, needle), 2);\n    let c = MarkerCounter::build(needle);\n    assert_eq!(c.count_marker(&lines), 2);\n    // one-shot shapes still work through the same finder\n    assert_eq!(c.count_marker(&[b\"nothing\".to_vec()]), 0);\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, rust_perf_bench validator, 2026-10-16, load ~10 of 16 AC-power disclosed; alloc-det stable x3): 6.35x/5.71x/5.51x at n=1K/16K/262K lines (first run at the small-line shape) -- the fixture was RESTRUCTURED ONCE per the discipline: the FIRST shape (~30B lines) measured ParetoDominated 0/3/0 -- below memchr's own 64B dispatch threshold the per-call shape takes the cheap direct-rabinkarp path and NEVER constructs the meta searcher, so the hoist forced the HEAVIER path; restructured to ~200B lines (the meta-Finder route the rule claims) and re-measured Dominant. the win scales with needle length x haystack count (construction is O(needle): rank scan + rabinkarp table + pair/prefilter selection); SHORT needles (0/1 byte) skip the machinery at construction and measure a TIE -- disclosed, not hidden (the 1-byte cell is the source's own early-return, memmem/searcher.rs:59-63). ONE LAW IN TWO REPOS -- MERGED with both citations (the byte-rank-select-rarest-probe precedent). SOURCES (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT + BurntSushi/bstr @ tag 1.13.1, commit 134195be38c7c9a4887980fd0f6f8bae042dd91e, Apache-2.0/MIT; every quote verified byte-for-byte at the .raw pin clones this session, checked out AT the release tags before any read). THE LAW (memchr/src/memmem/mod.rs:51-57, verbatim): 'It may be possible for the overhead of constructing a substring searcher to be measurable in some workloads. In cases where the same needle is used to search many haystacks, it is possible to do construction once and thus to avoid it for subsequent searches. This can be done with a Finder (or a FinderRev for reverse searches).' THE FINDER DOCS (memmem/mod.rs:370-378, verbatim): 'A single substring searcher fixed to a particular needle.' + 'The purpose of this type is to permit callers to construct a substring searcher that can be used to search haystacks without the overhead of constructing the searcher in the first place. This is a somewhat niche concern when it's necessary to re-use the same needle to search multiple different haystacks with as little overhead as possible. In general, using find is good enough, but Finder is useful when you can meaningfully observe searcher construction time in a profile.' THE ONE-SHOT ANTI-REGIME IS THE DOCS' OWN (the hedge belongs in the rule): 'In general, using find is good enough'. THE BSTR TWIN (bstr/src/ext_slice.rs:3080-3091, verbatim): 'A single substring searcher fixed to a particular needle.' + '...it's necessary to reuse the same needle to search multiple different haystacks with as little overhead as possible. In general, using ByteSlice::find or ByteSlice::find_iter is good enough, but Finder is useful when you can meaningfully observe searcher construction time in a profile.' + the reverse-finder face (ext_slice.rs:720-723, verbatim): 'Note that if you're are searching for the same needle in many different small haystacks, it may be faster to initialize a FinderReverse once, and reuse it for each search.' (the source's own doubled-word typo preserved). WHAT CONSTRUCTION COSTS (memchr/src/memmem/searcher.rs:40-49 + 68-71, verbatim): 'Creates a new meta substring searcher that attempts to choose the best algorithm based on the needle, heuristics and what the current target supports.' + 'let rabinkarp = rabinkarp::Finder::new(needle);' + 'let pair = match Pair::with_ranker(needle, &ranker) {' -- the rank scan + table build run unconditionally; the 1-byte early-return (searcher.rs:59-63) is why short needles tie. THE PER-CALL SHAPE THIS RULE KILLS (memmem/mod.rs:185-190, verbatim): 'pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {' + 'if haystack.len() < 64 {' + 'rabinkarp::Finder::new(needle).find(haystack, needle)' + '} else {' + 'Finder::new(needle).find(haystack)' -- every convenience call constructs. THE OWNED COPY (memmem/mod.rs:461-465, verbatim): 'If this is already an owned finder, then this is a no-op. Otherwise, this copies the needle.' -- hoist the construction, not a per-call owned conversion. REGIME: the same needle searched across MANY haystacks (log scanners, field splitters, route matchers); needles long enough to build real machinery (>= a few bytes; 16B in the fixture). ANTI-REGIMES (source-stated + labeled inference): one-shot searches (the docs' own 'find is good enough'); needles that change per call (construction cannot amortize); 0/1-byte needles (the machinery is skipped -- tie); a Finder built with a borrowed needle outliving the needle is UB-adjacent borrow error -- into_owned at hoist time, never per call. DEDUP ADJUDICATION (T0b): distinct from regex-hoist-lazylock (same hoist DECISION, different substrate + cost shape -- a compiled regex is us+KB, a Finder is O(needle); LazyLock static vs finder reuse; the bench shapes differ -- NEVER merged; they compose: a regex's meta engine hoists its own prefilters by this same law), prefilter-choice-shape-ladder (owns the accelerator CHOICE -- its fixed arm already uses a prebuilt Finder; this rule owns the hoist decision itself; the boundary sentence stands), dispatch-handler-pair-hoist / mask-memo-hoist-to-owner-lifetime / debug-assert-length-hoist (loop-invariant hoists, not build-once searcher amortization). Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(finder_hoist_reuse_bench_spec()),
        },
        // ── complexity: an idempotent normalize checks the invariant it
        // restores (or keeps the applied flag) before rebuilding ──────────
        // Source: rust-lang/regex @ regex-automata-0.4.18,
        // regex-syntax/src/hir/interval.rs (canonicalize/folded/union).
        PerfRule {
            rule_id: "canonical-set-guarded-rebuild".to_string(),
            name: "An idempotent normalize/rebuild that sorts + merges unconditionally → check the invariant it restores first (or keep an applied flag) and early-return — callers that already hold the invariant pay an O(n) scan instead of a sort + merge".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// NAIVE: every normalize call sorts + merges, even when the set is\n// already in canonical form — and the operation may run many times on\n// the same (already-normal) data.\n\n#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]\nstruct Span {\n    start: u32,\n    end: u32, // inclusive\n}\n\npub struct SpanSet {\n    ranges: Vec<Span>,\n}\n\nimpl SpanSet {\n    pub fn normalize(&mut self) {\n        // unconditional: sort + merge every call\n        self.ranges.sort();\n        let mut out: Vec<Span> = Vec::with_capacity(self.ranges.len());\n        for &s in &self.ranges {\n            match out.last_mut() {\n                // overlapping or adjacent -> extend\n                Some(last) if s.start <= last.end.saturating_add(1) => {\n                    last.end = last.end.max(s.end);\n                }\n                _ => out.push(s),\n            }\n        }\n        self.ranges = out;\n    }\n\n    pub fn push(&mut self, s: Span) {\n        self.ranges.push(s);\n    }\n}".to_string(),
            fixed_code: "// OPT: the guard family from the source — (1) the invariant check\n// before the rebuild, (2) the applied flag for idempotent operations.\n// An already-canonical set pays one O(n) windows-scan; a repeat call\n// pays nothing.\n\n#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]\nstruct Span {\n    start: u32,\n    end: u32,\n}\n\npub struct SpanSet {\n    ranges: Vec<Span>,\n    normalized: bool,\n}\n\nimpl SpanSet {\n    pub fn normalize(&mut self) {\n        // the applied flag: the operation already ran to fixpoint\n        if self.normalized {\n            return;\n        }\n        // the invariant check: already ordered + non-adjacent -> skip\n        if self.is_canonical() {\n            self.normalized = true;\n            return;\n        }\n        self.ranges.sort();\n        let mut out: Vec<Span> = Vec::with_capacity(self.ranges.len());\n        for &s in &self.ranges {\n            match out.last_mut() {\n                Some(last) if s.start <= last.end.saturating_add(1) => {\n                    last.end = last.end.max(s.end);\n                }\n                _ => out.push(s),\n            }\n        }\n        self.ranges = out;\n        self.normalized = true;\n    }\n\n    fn is_canonical(&self) -> bool {\n        for pair in self.ranges.windows(2) {\n            if pair[0] >= pair[1] || pair[0].end.saturating_add(1) >= pair[1].start {\n                return false;\n            }\n        }\n        true\n    }\n\n    pub fn push(&mut self, s: Span) {\n        self.ranges.push(s);\n        self.normalized = false; // mutations invalidate the flag\n    }\n\n    pub fn ranges(&self) -> &[Span] {\n        &self.ranges\n    }\n}\n\n#[test]\nfn guarded_normalize_matches_unconditional_and_skips_when_canonical() {\n    let mut a = SpanSet { ranges: vec![Span { start: 5, end: 9 }, Span { start: 0, end: 3 }, Span { start: 20, end: 25 }], normalized: false };\n    let mut b = a.clone();\n    a.normalize();\n    b.normalize();\n    b.normalize(); // the repeat call hits the flag\n    assert_eq!(a.ranges(), b.ranges());\n    assert_eq!(a.ranges().len(), 3); // 0-3, 5-9, 20-25 — nothing adjacent\n    // an already-canonical set short-circuits at the windows-scan\n    let mut c = SpanSet { ranges: vec![Span { start: 0, end: 3 }, Span { start: 5, end: 9 }], normalized: false };\n    c.normalize();\n    assert!(c.is_canonical());\n    // mutations clear the flag (the flag can never lie true)\n    c.push(Span { start: 4, end: 4 });\n    assert!(!c.normalized);\n    c.normalize();\n    assert_eq!(c.ranges().len(), 1); // 0-9 merged\n}".to_string(),
            context_notes: Some("IDIOM RULE, source-shipped, honestly NotBenchable: a synthetic bench of sort-already-sorted is O(n) either way -- the isolated win reduces to the merge pass + any realloc, exactly the marginal class the validator measures Dominated; the decision content is the guard FAMILY, not a timing claim. SOURCE (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; every quote verified byte-for-byte at the .raw/regex pin clone this session). THE GUARD (regex-syntax/src/hir/interval.rs:369-381, verbatim): 'Converts this set into a canonical ordering.' + 'fn canonicalize(&mut self) {' + 'if self.is_canonical() {' + 'return;' + '}' + 'self.ranges.sort();' -- the O(n) windows-scan gates the sort + merge. THE INVARIANT TEST (interval.rs:397-408, verbatim): 'Returns true if and only if this class is in a canonical ordering.' + 'fn is_canonical(&self) -> bool {' + 'for pair in self.ranges.windows(2) {' + 'if pair[0] >= pair[1] {' + 'return false;' + '}' + 'if pair[0].is_contiguous(&pair[1]) {' + 'return false;' + '}' THE APPLIED FLAG (interval.rs:142-145, verbatim): 'pub fn case_fold_simple(&mut self) -> Result<(), unicode::CaseFoldError> {' + 'if self.folded {' + 'return Ok(());' + '}' with the flag's own honesty clause (interval.rs:37-53, verbatim): 'While not required at all for correctness, we keep track of whether an interval set has been case folded or not. This helps us avoid doing redundant work if, for example, a set has already been cased folded.' + 'Note that when this is true, it *must* be the case that the set is case folded. But when it's false, the set *may* be case folded.' + 'Bottom line: this is a performance optimization.' + 'folded: bool,' -- the flag may be pessimistically false (cheap to re-derive), never optimistically true. THE EQUALITY SHORT-CIRCUIT (interval.rs:160-166, verbatim): 'pub fn union(&mut self, other: &IntervalSet<I>) {' + 'if other.ranges.is_empty() || self.ranges == other.ranges {' + 'return;' + '}' -- the third member of the family. THE PREMISE CORRECTION (read at the pin this session): the staged lead named 'the is_canonical guard before sort' on the PUSH path -- at this pin IntervalSet::push (interval.rs:83-119) is a binary_search + union-window + single splice that NEVER sorts and never calls is_canonical; the guard family lives in canonicalize (callers: new at line 78, case_fold_simple at 150/154, union at 166 -- the entire caller list). A rule written to the stale premise would be unfireable against real code of this vintage; the rule above is written to the ACTUAL mechanism. WHY THE SOURCE BOTHERS (interval.rs:20-29, verbatim): 'character classes (especially of the Unicode variety) can become quite large, and it would be nice to keep regex compilation snappy even in debug builds. (In the past, I have been careless with this area of code and it has caused slow regex compilations in debug mode, so this isn't entirely unwarranted.)' REGIME: idempotent normalize/fixpoint operations called repeatedly or on data that may already hold the invariant (set normalizers, dedup passes, format normalizers); large domains where the sort+merge is real work. ANTI-REGIMES: one-shot builds from arbitrary-order input (canonicalize must run anyway -- the guard is pure overhead); tiny sets where the windows-scan costs what the skipped sort would; invariants that are EXPENSIVE to check (the guard must be O(n) or better vs the O(n log n) rebuild); mutation-heavy workloads that clear the flag every other call. DEDUP ADJUDICATION (T0c): distinct from sortedness-check-branchless-accumulate (the SCAN SHAPE of a sortedness validation -- early-exit branch vs branchless accumulate; this rule is SKIP-THE-REBUILD-WHEN-THE-INVARIANT-HOLDS, a decision that rule says nothing about; the Batch-244 adjacency reason was genus-level and is overruled with this read), sorted-input-previous-row-fast-path (caller-side per-row pre-check; this check lives INSIDE the normalizer so every caller benefits, and the skipped work is a sort+merge), range-table-page-memoization (lazy build-once keyed by page; this is early-return-on-already-applied keyed by an invariant or monotone flag), prompt-cache-canonical-interval-keys (name-only overlap -- TensorFold serving-cache key canonicalization, unrelated). Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── layout: reserve the in-domain-impossible value as Option's
        // niche — every optional slot halves ─────────────────────────────
        // Source: rust-lang/regex @ regex-automata-0.4.18,
        // regex-automata/src/util/primitives.rs (NonMaxUsize).
        PerfRule {
            rule_id: "nonmax-sentinel-option-niche".to_string(),
            name: "Table of Option<usize> slots (2× the payload width) → reserve the in-domain-impossible value (usize::MAX, provable via the isize::MAX slice bound) as the niche payload — Option<NonMaxUsize> is the same size as usize and every slot halves".to_string(),
            category: "layout".to_string(),
            buggy_code: "// NAIVE: the slot table stores optional offsets as Option<usize> —\n// None needs its own tag bit, so every slot is 16 bytes wide for an\n// 8-byte payload.\n\npub struct Slots {\n    slots: Vec<Option<usize>>,\n}\n\nimpl Slots {\n    pub fn new(n: usize) -> Slots {\n        Slots { slots: vec![None; n] }\n    }\n\n    pub fn set(&mut self, at: usize, offset: usize) {\n        self.slots[at] = Some(offset);\n    }\n\n    pub fn get(&self, at: usize) -> Option<usize> {\n        self.slots[at]\n    }\n}\n\n#[test]\nfn slots_round_trip() {\n    let mut s = Slots::new(4);\n    assert_eq!(s.get(0), None);\n    s.set(1, 42);\n    assert_eq!(s.get(1), Some(42));\n}".to_string(),
            fixed_code: "// OPT: usize::MAX is IMPOSSIBLE for an offset (Rust guarantees\n// slices never exceed isize::MAX), so it can carry Option's niche: wrap\n// the value +-1 through NonZeroUsize and Option<NonMaxUsize> is exactly\n// one usize wide — every slot in the table halves.\n\nuse core::num::NonZeroUsize;\n\n/// A usize that can never be usize::MAX (the source's own shape:\n/// repr(transparent) over NonZeroUsize, + -1 wrapping).\n#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]\n#[repr(transparent)]\npub struct NonMaxUsize(NonZeroUsize);\n\nimpl NonMaxUsize {\n    /// None only when the value IS usize::MAX (the impossible offset).\n    pub fn new(value: usize) -> Option<NonMaxUsize> {\n        NonZeroUsize::new(value.wrapping_add(1)).map(NonMaxUsize)\n    }\n\n    pub fn get(self) -> usize {\n        self.0.get().wrapping_sub(1)\n    }\n}\n\npub struct Slots {\n    slots: Vec<Option<NonMaxUsize>>,\n}\n\nimpl Slots {\n    pub fn new(n: usize) -> Slots {\n        Slots { slots: vec![None; n] }\n    }\n\n    pub fn set(&mut self, at: usize, offset: usize) {\n        self.slots[at] = NonMaxUsize::new(offset);\n    }\n\n    pub fn get(&self, at: usize) -> Option<usize> {\n        self.slots[at].map(NonMaxUsize::get)\n    }\n}\n\n#[test]\nfn niche_slots_are_half_width_and_round_trip() {\n    // the whole law, asserted: the Option is the payload's width\n    assert_eq!(\n        std::mem::size_of::<Option<NonMaxUsize>>(),\n        std::mem::size_of::<usize>()\n    );\n    assert_eq!(\n        std::mem::size_of::<Option<usize>>(),\n        2 * std::mem::size_of::<usize>()\n    );\n    let mut s = Slots::new(4);\n    assert_eq!(s.get(0), None);\n    s.set(1, 42);\n    assert_eq!(s.get(1), Some(42));\n    // the boundary: 0 and usize::MAX - 1 round-trip; MAX is refused\n    s.set(2, 0);\n    assert_eq!(s.get(2), Some(0));\n    s.set(3, usize::MAX - 1);\n    assert_eq!(s.get(3), Some(usize::MAX - 1));\n    assert_eq!(NonMaxUsize::new(usize::MAX), None);\n}".to_string(),
            context_notes: Some("DESIGN-TIME LAYOUT LAW, honestly NotBenchable: a bench would sweep two slot-table types and rediscover size_of::<Option<usize>>() == 16 vs 8 -- the fixture-artificiality drop class; the decision is WHICH TYPE YOU DECLARE, not how you loop (the repr-align-register-partition-store-forwarding verdict class). SOURCE (rust-lang/regex @ tag regex-automata-0.4.18, commit e6ad938198e68339a593f85d228f2d44cb54718f, Apache-2.0/MIT; every quote verified byte-for-byte at the .raw/regex pin clone this session). THE LAW (regex-automata/src/util/primitives.rs:25-29, verbatim): '[NonMaxUsize] - Represents a usize that cannot be usize::MAX. As a result, Option<NonMaxUsize> has the same size in memory as a usize. This useful, for example, when representing the offsets of submatches since it reduces memory usage by a factor of 2. It is a legal optimization since Rust guarantees that slices never have a length that exceeds isize::MAX.' (the source's own grammar slip 'This useful' preserved; the isize::MAX bound IS the correctness contract -- the legality proof ships with the type). THE TYPE (primitives.rs:39-49, verbatim): 'A usize that can never be usize::MAX.' + 'This is similar to core::num::NonZeroUsize, but instead of not permitting a zero value, this does not permit a max value.' + '...since Rust slices are guaranteed to never have a length exceeding isize::MAX, we can use usize::MAX as a sentinel to indicate that no match was found. Indeed, types like Option<NonMaxUsize> have exactly the same size in memory as a usize.' THE REPRESENTATION (primitives.rs:51-56, verbatim): 'This type is defined to be repr(transparent) for core::num::NonZeroUsize, which is in turn defined to be repr(transparent) for usize.' + '#[repr(transparent)]' + 'pub struct NonMaxUsize(NonZeroUsize);' THE ENCODE/DECODE (primitives.rs:59-72, verbatim): 'NonZeroUsize::new(value.wrapping_add(1)).map(NonMaxUsize)' + 'self.0.get().wrapping_sub(1)' -- one wrapping add/sub per crossing, the cost the source pays willingly on slot writes. THE LIVE TABLE (util/captures.rs:171-172, verbatim): 'N.B. Option<NonMaxUsize> has the same size as a usize.' + 'slots: Vec<Option<NonMaxUsize>>,' and the PikeVM slot arrays carry it throughout (nfa/thompson/pikevm.rs:1095/1143/1228/1474/1614/1667: 'slots: &mut [Option<NonMaxUsize>],'). A SECOND, PRIVATE TWIN in the same monorepo (regex-lite/src/int.rs:33 'pub(crate) struct NonMaxUsize(NonZeroUsize)') -- cite regex-automata as the provenance (the public one), the twin is the corroboration. THE PAYOFF AXIS, RELABELED from the staging reason: the source frames this as MEMORY (the factor-2 claim is source-stated, primitives.rs:25-29); the timing-at-scale framing (2x table density -> cache/bandwidth on large slot tables) is OUR INFERENCE and only materializes at scale -- labeled as such, never benched. REGIME: dense tables of optional indices/offsets where the sentinel is PROVABLY impossible in-domain (slice-derived values vs isize::MAX; arena indices vs the arena's capacity bound). ANTI-REGIMES: values for which MAX or the sentinel is MEANINGFUL in-domain (the rule is only legal when the impossibility is provable -- the proof is part of the type, not a hope); small tables where density is irrelevant; hot paths doing many new/get round-trips where the wrapping +-1 is paid per crossing (the source accepts this on slot writes); domains wanting the value usize::MAX itself. DEDUP ADJUDICATION (T0c): distinct from repr-align-register-partition-store-forwarding (aligns a struct for register partitioning -- this shrinks an Option via sentinel niche; same verdict class, different lever), byte-class-equivalence-table-shrink (collapses the KEY SPACE before the table -- a semantic re-index; this is a pure type-level repack with zero code-shape change; they compose), the sentinel LATCH rules (prefilter-skip-density-demotion's sentinel-0 -- a runtime latch VALUE; this sentinel is a type-representation device for Option's niche), repr-u8-enum (enum width, not Option-niche). Grep confirms no corpus rule owns NonMax/niche (the only occurrences are the Batch-244 deferral record + this batch's staging). Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── dispatch: #[cold] belongs on the rare heavyweight confirm arm,
        // never the small hot helper — and never without a measurement ────
        // Source: BurntSushi/memchr @ 2.8.3 (arch/all/rabinkarp.rs — the
        // positive; arch/generic/packedpair.rs + generic/memchr.rs — the
        // counter-case + the generic-code constraint).
        PerfRule {
            rule_id: "match-branch-cold-confirm".to_string(),
            name: "A rare, heavyweight confirm branch inline in the hot scan → #[cold] #[inline(never)] on the confirm fn tightens the hot loop (the source cites a benchmark win) — but never on small always-hot arithmetic helpers (measured no consistent difference) and never in generic code inlined into #[target_feature] fns (unexpressible); measure, never assume".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "// The rare confirm (full memcmp behind a probabilistic prefilter)\n// is an ordinary inline fn — its body lands in the hot scan's code\n// path and the branch predicts-taken layout suffers for it.\n\npub fn count(hay: &[u8], needle: &[u8]) -> usize {\n    if needle.is_empty() || hay.len() < needle.len() {\n        return 0;\n    }\n    let mut total = 0usize;\n    let mut start = 0usize;\n    while let Some(at) = memchr::memchr(needle[0], &hay[start..]) {\n        let pos = start + at;\n        // inline confirm: the rare heavy branch lives in the loop\n        if hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle {\n            total += 1;\n        }\n        start = pos + 1;\n    }\n    total\n}".to_string(),
            fixed_code: "// OPT: the confirm moves OUT of the hot path into its own\n// #[cold] #[inline(never)] fn — the source's rabinkarp shape. The\n// attribute is a MEASUREMENT decision (the same repo removed it from a\n// hot arithmetic helper after measuring no difference), never a\n// default.\n// [dep: memchr = \"2\"]\n\nuse memchr::memchr;\n\n/// The rare heavyweight confirm, forced out of the hot loop's code\n/// path. #[cold] hints rarity to the branch layout; #[inline(never)]\n/// keeps the body from re-entering the loop's code.\n#[cold]\n#[inline(never)]\nfn confirm(hay: &[u8], pos: usize, needle: &[u8]) -> bool {\n    hay.len() >= pos + needle.len() && &hay[pos..pos + needle.len()] == needle\n}\n\npub fn count(hay: &[u8], needle: &[u8]) -> usize {\n    if needle.is_empty() || hay.len() < needle.len() {\n        return 0;\n    }\n    let mut total = 0usize;\n    let mut start = 0usize;\n    while let Some(at) = memchr(needle[0], &hay[start..]) {\n        let pos = start + at;\n        if confirm(hay, pos, needle) {\n            total += 1;\n        }\n        start = pos + 1;\n    }\n    total\n}\n\n#[test]\nfn cold_confirm_still_confirms() {\n    let hay = b\"aXbXc\";\n    assert_eq!(count(hay, b\"X\"), 2);\n    assert_eq!(count(hay, b\"Xc\"), 1);\n    assert_eq!(count(hay, b\"zz\"), 0);\n    assert_eq!(count(b\"\", b\"x\"), 0);\n}".to_string(),
            context_notes: Some("SUGGEST-ONLY codegen-placement law, honestly NotBenchable: the source itself measures BOTH directions -- a fixture would be a coin flip against the ~50% falsification precedent, and the healer must never auto-apply an attribute the source half-refutes (the inline-hint-not-through-unmarked-wrapper verdict class: codegen attributes are LLVM-inliner claims over the real body). SOURCE (BurntSushi/memchr @ tag 2.8.3, commit 5fdb40c054e1fff359a2f7bdf7f87a13b34b465d, Unlicense/MIT; every quote verified byte-for-byte at the .raw/memchr pin clone this session). THE POSITIVE (src/arch/all/rabinkarp.rs:350-355, verbatim): 'Returns true when x[i] == y[i] for all 0 <= i < n.' + 'We forcefully don't inline this to hint at the compiler that it is unlikely to be called. This causes the inner rabinkarp loop above to be a bit tighter and leads to some performance improvement. See the memmem/krate/prebuilt/sliceslice-words/words benchmark.' with the attributes at 360-361: '#[cold]' + '#[inline(never)]' + 'unsafe fn is_equal_raw(x: *const u8, y: *const u8, n: usize) -> bool {' -- the rare heavyweight CONFIRM (the memcmp behind the prefilter), with a NAMED benchmark. THE COUNTER-CASE (src/arch/generic/packedpair.rs:302-306, verbatim): 'Accepts a chunk-relative offset and returns a haystack relative offset.' + 'This used to be marked #[cold] and #[inline(never)], but I couldn't observe a consistent measureable difference between that and just inlining it. So we go with inlining it.' (the source's own typo 'measureable' preserved) -- a tiny offset-ARITHMETIC helper on the match path: the attribute washed there. THE GENERIC-CODE CONSTRAINT (src/arch/generic/memchr.rs:84-91, verbatim): 'The fact that the code below is generic does somewhat inhibit us. For example, I've noticed that introducing an unlineable #[cold] function to handle the match case in the loop generates tighter assembly, but there is no way to do this in the generic code below because the generic code doesn't know what target_feature annotation to apply to the unlineable function.' + 'We could make such functions part of the Vector trait, but we instead live with the slightly sub-optimal codegen for now since it doesn't seem to have a noticeable perf difference.' -- in generic monomorphized code destined for #[target_feature] contexts the attribute is UNEXPRESSIBLE (the unlineable fn cannot carry the feature), which saves templated-kernel authors from trying. THE LAW, CONDITIONAL: cold the rare heavyweight confirm (benchmark-cited positive); never the small always-hot helper (measured wash); never in generic/target_feature-inlined code (unexpressible); and ALWAYS with a before/after measurement -- both counter-comments are measurement stories. REGIME: a compare/confirm fn called only after a cheap probabilistic test accepts -- rare relative to the scan loop, heavyweight enough (full-needle memcmp) to want out of the hot loop's code path. ANTI-REGIMES (all source-stated): tiny arithmetic helpers on the match path (the packedpair wash); generic monomorphized code inlined into #[target_feature(enable)] fns (the constraint above); any site where you didn't measure. DEDUP ADJUDICATION (T0c): distinct from utf8-fast-lane-cold-diagnostics (that rule REMOVES TRACKED STATE from the loop via a fast/cold lane split -- its context_notes already pre-adjudicate this boundary: 'the #[cold] ATTRIBUTE on a rare arm; THIS rule removes tracked state, the attribute is optional garnish' -- the naming cross-reference is honored: this rule lands in the Batch-245 slot that rule's notes reserved), runtime-bool-duplicates-inlinable-body (branch hoisting into a conditional guard -- LLVM inliner decision over a duplicated body, not cold-path placement), inline-hint-not-through-unmarked-wrapper (inline PROPAGATION through wrappers, not rarity hints), cold-l3-evict-min-of-n (cache-eviction policy -- 'cold' in the cache sense, not the codegen attribute), parallel-disjoint-pread-hides-cold-page-latency (page-cache latency, unrelated). Grep confirms no corpus rule owns #[cold] (the only prior occurrence is the utf8 rule's boundary note). Batch 245 / Plan 248.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// Bench spec for `self_loop_state_memchr_skip`: a [^a]+a-shaped 3-state
/// automaton over LCG text at controlled exit density. Buggy = the natural
/// per-byte state-step loop; fixed = the same loop with the accel arm (the
/// range check PAID on every visit) + one memchr over the exit byte.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn self_loop_state_memchr_skip_bench_spec() -> BenchSpec {
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

const EXIT: u8 = b'a';

static NEXT: [[u8; 256]; 3] = {
    let mut t = [[0u8; 256]; 3];
    let mut b = 0usize;
    while b <= 255 {
        if b as u8 != EXIT {
            t[0][b] = 1;
            t[2][b] = 1;
        }
        t[1][b] = ((b as u8) == EXIT) as u8 * 2;
        b += 1;
    }
    t
};

pub struct Input {
    hay: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x1234_5678_9ABC_DEF0;
    // exit-byte density ~5%: the win regime (average inter-exit run ~20
    // bytes). The anti-regime cell lives in the rule's own test, not the
    // timed harness -- the timed read must be the regime the rule claims.
    let hay: Vec<u8> = (0..n)
        .map(|_| {
            if lcg(&mut state) % 100 < 5 {
                b'a'
            } else {
                b'x'
            }
        })
        .collect();
    Input { hay }
}

mod buggy {
    use super::NEXT;
    pub fn count(hay: &[u8]) -> usize {
        let mut sid = 0usize;
        let mut runs = 0usize;
        let mut at = 0usize;
        while at < hay.len() {
            sid = NEXT[sid][hay[at] as usize] as usize;
            at += 1;
            if sid == 2 {
                runs += 1;
                sid = 0;
            }
        }
        runs
    }
}

mod fixed {
    use super::{memchr, EXIT, NEXT};
    const ACCEL_MIN: usize = 1;
    const ACCEL_MAX: usize = 1;
    pub fn count(hay: &[u8]) -> usize {
        let mut sid = 0usize;
        let mut runs = 0usize;
        let mut at = 0usize;
        while at < hay.len() {
            if (ACCEL_MIN..=ACCEL_MAX).contains(&sid) {
                at = memchr(EXIT, &hay[at..]).map_or(hay.len(), |i| at + i + 1);
                sid = 2;
            } else {
                sid = NEXT[sid][hay[at] as usize] as usize;
                at += 1;
            }
            if sid == 2 {
                runs += 1;
                sid = 0;
            }
        }
        runs
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.hay));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.hay));
}
"#
        .to_string(),
    }
}

/// Bench spec for `pair_probe_confirm_fused`: a jointly-rare pair regime —
/// individually-tolerable bytes (X/Q each a few percent via seeded strays),
/// the pair planted rare. Buggy = single-rarest-byte memchr probe + full
/// verify; fixed = the SWAR two-offset AND + verify at hits only.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn pair_probe_confirm_fused_bench_spec() -> BenchSpec {
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

// The source's regime: NO rare byte exists. Seven symbols at ~14% each;
// even the needle's rarest single byte is common. The pair is the
// discriminator (~2% at its two offsets vs ~14% for any single byte).
const ALPHABET: &[u8] = b"aeiont ";
const NEEDLE: &[u8] = b"nation";
// the needle's two probe offsets: (1, 3) = 'a', 'i' -- distinct indices,
// both individually common, jointly ~2%.
const OFF1: usize = 1;
const OFF2: usize = 3;

pub struct Input {
    hay: Vec<u8>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x0FED_CBA9_8765_4321;
    let mut hay = Vec::with_capacity(n);
    while hay.len() < n {
        hay.push(ALPHABET[(lcg(&mut state) % 7) as usize]);
        // plant the joint event at a controlled, visible rate
        if lcg(&mut state) % 700 == 0 && hay.len() + NEEDLE.len() < n {
            hay.extend_from_slice(NEEDLE);
        }
    }
    hay.truncate(n);
    Input { hay }
}

mod buggy {
    use super::{memchr, NEEDLE, OFF1};
    pub fn count(hay: &[u8]) -> usize {
        // the single-probe arm at its honest best: probe one byte of the
        // needle ('a' at OFF1), anchor the needle at hit_pos - OFF1,
        // verify every anchored hit
        let probe = NEEDLE[OFF1];
        let mut total = 0usize;
        let mut start = 0usize;
        while let Some(at) = memchr(probe, &hay[start..]) {
            let hit = start + at;
            start = hit + 1;
            if hit < OFF1 {
                continue;
            }
            let pos = hit - OFF1;
            if hay.len() >= pos + NEEDLE.len() && &hay[pos..pos + NEEDLE.len()] == NEEDLE {
                total += 1;
            }
        }
        total
    }
}

mod fixed {
    use super::{NEEDLE, OFF1, OFF2};

    fn byte_eq_mask(word: u64, want: u8) -> u64 {
        let x = word ^ (u64::from(want) * 0x0101_0101_0101_0101);
        (x.wrapping_sub(0x0101_0101_0101_0101) & !x) & 0x8080_8080_8080_8080
    }

    pub fn count(hay: &[u8]) -> usize {
        let (a, b) = (NEEDLE[OFF1], NEEDLE[OFF2]);
        let delta = OFF2.abs_diff(OFF1);
        let end = hay.len().saturating_sub(NEEDLE.len());
        let mut total = 0usize;
        // byte j of the fused mask = a match at absolute position
        // base + OFF1 + j for byte a AND base + OFF2 + j for byte b --
        // i.e. the needle anchored at base + OFF1 + j - OFF1 = base + j.
        let mut base = 0usize;
        while base + OFF2 + 8 <= hay.len() {
            let w1 = u64::from_le_bytes(hay[base + OFF1..base + OFF1 + 8].try_into().unwrap());
            let w2 = u64::from_le_bytes(hay[base + OFF2..base + OFF2 + 8].try_into().unwrap());
            let mut bits = byte_eq_mask(w1, a) & byte_eq_mask(w2, b);
            while bits != 0 {
                let j = bits.trailing_zeros() as usize / 8; // byte index from the bit-8j+7 flag
                let pos = base + j;
                if pos <= end && &hay[pos..pos + NEEDLE.len()] == NEEDLE {
                    total += 1;
                }
                bits &= bits - 1;
            }
            base += 8;
        }
        // tail: same anchor semantics as the body (anchor = base; byte a
        // at base+OFF1, byte b at base+OFF2) — every anchor from the body
        // end through `end` is covered here
        while base <= end {
            if hay[base + OFF1] == a && hay[base + OFF2] == b {
                if &hay[base..base + NEEDLE.len()] == NEEDLE {
                    total += 1;
                }
            }
            base += 1;
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.hay));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.hay));
}
"#
        .to_string(),
    }
}

/// Bench spec for `finder_hoist_reuse`: the same 16-byte marker needle
/// searched across many small log-like lines. Buggy = `memmem::find` per line
/// (constructs the full searcher per call at >=64B... lines here are ~96B so
/// the meta finder constructs per call); fixed = one Finder hoisted outside
/// the loop, reused per line.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn finder_hoist_reuse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
// [dep: memchr = "2"]
use memchr::memmem;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

const NEEDLE: &[u8] = b"[ERROR][disk3] ";

pub struct Input {
    lines: Vec<Vec<u8>>,
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0x7777_8888_9999_AAAA;
    let mut lines = Vec::with_capacity(n);
    for i in 0..n {
        // ~200B lines: at/above memchr's own 64B dispatch threshold, so the
        // per-call shape constructs the FULL meta searcher every line (the
        // <64B rabinkarp shortcut is the small-line anti-regime).
        let mut line = Vec::with_capacity(208);
        if i % 7 == 0 {
            line.extend_from_slice(NEEDLE);
        }
        for _ in 0..4 {
            let wlen = 12 + (lcg(&mut state) % 28) as usize;
            for _ in 0..wlen {
                line.push(b'a' + (lcg(&mut state) % 26) as u8);
            }
            line.push(b' ');
        }
        line.push(b'\n');
        lines.push(line);
    }
    Input { lines }
}

mod buggy {
    use super::memmem;
    pub fn count(lines: &[Vec<u8>], needle: &[u8]) -> usize {
        let mut total = 0usize;
        for line in lines {
            // per-call construction: rank scan + rabinkarp table + pair
            // selection run again for every line
            if memmem::find(line, needle).is_some() {
                total += 1;
            }
        }
        total
    }
}

mod fixed {
    use super::memmem::Finder;
    pub fn count(lines: &[Vec<u8>], needle: &[u8]) -> usize {
        let finder = Finder::new(needle);
        let mut total = 0usize;
        for line in lines {
            if finder.find(line).is_some() {
                total += 1;
            }
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count(&input.lines, NEEDLE));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count(&input.lines, NEEDLE));
}
"#
        .to_string(),
    }
}
