//! Entries slice — Batch 298 (2026-10-27): the bbtree ordered-structure-
//! internals distill (Plan 289, Research 322). TWO own `rust_perf` rules
//! from XiangpengHao/bbtree @ `8adae97a0ed97e5c98fa2790312eb0aec9719846`
//! (MIT OR Apache-2.0, v0.1.5, edition-2024 `no_std`, zero runtime deps —
//! vstd is Verus ghost-only, gone after compilation; every quote
//! re-verified verbatim at the pin this batch):
//!
//! - `order-preserving-inline-key-encoding` (caching) — beside each key a
//!   fixed-width big-endian window of an order-preserving byte encoding,
//!   compared as an integer word first; the full key is read only where
//!   windows tie; each node's window starts past the bytes all of the
//!   node's keys share (the fences route, the windows rank).
//! - `count-based-simd-sorted-slot-search` (simd) — DROPPED AT MEASUREMENT
//!   (see the ⛔ note below: `ParetoDominated` 0/4/1 on the validator, the
//!   O(n) count loses to `partition_point` at every size ≥ 16; the negative
//!   lives here, the rule never entered the corpus)
//!
//! Rule 1 is ANNOTATED `NotBenchable` `bench_spec: None` — the win is
//! cache-miss-class on heap-backed keys inside a tree; a fair microbench
//! needs the node layout (fences + heap keys + windows), so the decision
//! content is the law + the tie-rate regime (the Batch-136
//! decision-content class).
//!
//! ⛔ **RULE 2 DROPPED AT MEASUREMENT (2026-10-29, the Batch-243/250
//! precedent):** the batch's second candidate
//! `count-based-simd-sorted-slot-search` landed a `bench_spec` at sizes
//! [8, 16, 32, 64, 128] claiming the whole-capacity branch-free count
//! beats `partition_point` — the `PerfBenchValidator`'s release measurement
//! (amo aarch64, probe-witness verified) REFUTED it:
//! **`ParetoDominated` — 0 faster / 4 slower / 1 tie** (n=8 Tie 1.00×;
//! n=16 0.00×; n=32 0.50×; n=64 0.33×; n=128 0.14×), and a standalone
//! release probe at the larger sizes the spec never reached confirmed the
//! law's O(n) shape never recovers (n=1024 16.4× worse, n=16384 159×,
//! n=262144 1924×, n=2097152 4363×; count only wins at n≤16 by ≤1.9×).
//! The landing note's "the law needs no intrinsics, LLVM auto-vectorizes
//! the plain count" premise is measured FALSE on this host — the source
//! (bbtree) wins with EXPLICIT SSE2/NEON vectors over a bounded node AND
//! against inline-key compares in cache-resident nodes, neither of which
//! a plain whole-capacity count over arbitrary input reproduces. The
//! negative lives here; no entries file carries the rule; the standalone
//! fixture evidence below predates the validator and proves the CORRECTNESS
//! arms only (the fixtures tested exactness, never speed).
//!
//! Rule 1's fixtures are ORIGINAL machinery (never bbtree code pasted — the
//! inline
//! key windows, the sign-flip integer encoding and the per-node prefix skip
//! are re-authored as plain-Rust stand-ins
//! with neutral names), each arm compiled and executed standalone before
//! this table was assembled:
//!
//! - `cd /tmp/plan289 && rustc --edition 2021 --test entries52_rule1_fixture.rs && ./entries52_rule1_fixture` — 5 passed / 0 failed (the law arm + the buggy arm as oracle)
//! - `cd /tmp/plan289 && rustc --edition 2021 --test rule1_buggy_fixture.rs && ./rule1_buggy_fixture` — 2 passed / 0 failed (the anti-pattern arm)
//!
//! the buggy/fixed arm pair shares the surrounding harness and differs
//! only on the law.
use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── caching: heap keys re-compared per probe vs integer windows
        // stored beside the key ─────────────────────────────────────────
        // Batch 298 / XiangpengHao/bbtree (Plan 289, Research 322): the
        // inline-key law (D18/D26/D31/D50) - order-preserving byte
        // encodings, big-endian zero-padded windows, per-node prefix
        // stripping, and the width/tie-rate two-regime.
        PerfRule {
            rule_id: "order-preserving-inline-key-encoding".to_string(),
            name: "in an ordered structure, store beside each key a fixed-width big-endian window of an order-preserving byte encoding and compare it as an integer word first, reading the full key only where windows tie, and start each node's window past the bytes all of the node's keys share (the fences route the target into the node's span first - the windows rank only in-span targets) - narrower windows pack more keys per cache line but tie more often (real neighbours tie 24-70% at 8-byte windows, 1-20% at 16), so long heap-backed keys with shared prefixes win biggest (u64 hits 198→138 ns, shared-prefix paths 547→495 ns, author-measured) while keys with no cheap order-preserving encoding (multi-field Ord, floats without the sign-bit remap - a wrong encoding is silently wrong) must stay on the full compare"
                .to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `order-preserving-inline-key-encoding`
// (riir-refine rust_perf corpus, Batch 298) -- the ANTI-PATTERN arm.
//
// A sorted Vec of heap byte-string keys searched by full `Ord`
// compares: every probe dereferences the heap key and compares from
// byte 0, re-comparing the bytes all of the node's keys share on every
// step. The LAW arm's integer windows (see the fixed arm fixture) are
// the oracle these tests check against. ORIGINAL machinery; the law it
// incurs is cited to bbtree in the rule's context_notes.

/// one node of the ordered structure, heap keys only
struct Node {
    /// heap byte-string keys, sorted
    keys: Vec<Vec<u8>>,
}

impl Node {
    fn build(mut keys: Vec<Vec<u8>>) -> Node {
        keys.sort();
        Node { keys }
    }

    /// THE PATTERN: a full Ord compare on every probe -- the heap key
    /// is dereferenced and compared from byte 0 (the shared prefix
    /// re-compared) every time.
    fn search_full(&self, target: &[u8]) -> Result<usize, usize> {
        self.keys.binary_search_by(|key| key.as_slice().cmp(target))
    }

    /// oracle: linear scan, no search structure to get wrong
    fn find_linear(&self, target: &[u8]) -> Result<usize, usize> {
        match self.keys.iter().position(|key| key.as_slice() == target) {
            Some(index) => Ok(index),
            None => {
                let pos = self
                    .keys
                    .partition_point(|key| key.as_slice() < target);
                Err(pos)
            }
        }
    }
}

/// tiny LCG, deterministic
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}

/// shared-prefix byte strings: every probe walks the shared prefix
/// before the compare can differ -- the cost the law arm removes
#[test]
fn shared_prefix_keys_search_correctly_via_full_compares() {
    let mut rng = Lcg(42);
    let prefix = b"node/000042/chunk/".to_vec();
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for len in 0..=24usize {
        let mut key = prefix.clone();
        for _ in 0..len {
            key.push(rng.byte()); // includes 0x00 bytes on purpose
        }
        keys.push(key);
    }
    let node = Node::build(keys);
    for key in &node.keys {
        assert_eq!(node.search_full(key), node.find_linear(key));
    }
    for _ in 0..64 {
        let mut probe = prefix.clone();
        for _ in 0..(rng.next_u64() % 26) {
            probe.push(rng.byte());
        }
        assert_eq!(node.search_full(&probe), node.find_linear(&probe));
    }
}

/// the exhaustive-ish probe matrix: present, absent, below-min,
/// above-max, and the gap fills -- all answered correctly by full
/// compares alone (correct, and every answer costs heap derefs)
#[test]
fn boundary_probe_matrix_matches_the_linear_oracle() {
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..200u32 {
        let mut key = b"shared/prefix/".to_vec();
        key.extend_from_slice(&(i as u16).to_be_bytes());
        if i % 3 == 0 {
            key.extend_from_slice(b"-longer-tail");
        }
        keys.push(key);
    }
    let node = Node::build(keys);
    for tail in 0..=u16::MAX {
        let mut probe = b"shared/prefix/".to_vec();
        probe.extend_from_slice(&tail.to_be_bytes());
        assert_eq!(
            node.search_full(&probe),
            node.find_linear(&probe),
            "probe tail {tail:#06x}",
        );
    }
}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `order-preserving-inline-key-encoding`
// (riir-refine rust_perf corpus, Batch 298) -- ORIGINAL machinery, never
// bbtree code pasted; the law it demonstrates (inline key windows, the
// sign-flip integer encoding, the per-node prefix skip) is cited to its
// source in the rule's context_notes.
//
// BUGGY arm: a sorted Vec of heap byte-string keys searched by full
// `Ord` compares -- every probe dereferences the heap key and
// re-compares the bytes all of the node's keys share.
//
// FIXED arm: beside each key a fixed-width big-endian window of an
// order-preserving byte encoding, compared as an integer word FIRST;
// the full key is read only where windows tie; and each node's window
// starts past the bytes all of the node's keys share. Signed integers
// ride the sign-bit flip (x ^ (1<<63), big-endian, zero-padded).

use std::cmp::Ordering;

/// the window width in bytes
const WINDOW: usize = 8;

/// Big-endian u64 window of `bytes` starting at `off`, zero-padded past
/// the end. Comparing these as integers IS comparing the encoded bytes
/// lexicographically over the window.
fn be_window(bytes: &[u8], off: usize) -> u64 {
    let mut window = 0u64;
    for i in 0..WINDOW {
        let byte = bytes.get(off + i).copied().unwrap_or(0);
        window = (window << 8) | u64::from(byte);
    }
    window
}

/// Order-preserving encoding of a signed integer: the bits with the sign
/// bit flipped, big-endian. Ordering on the encoding == ordering on i64.
fn i64_encoding(x: i64) -> [u8; 8] {
    (x as u64 ^ (1u64 << 63)).to_be_bytes()
}

/// bytes the node's first and last keys share. In a sorted node every
/// key starts with them (first <= k <= last cannot diverge inside the
/// extremes' shared run), so windows may start past them. A SEARCH
/// target must be routed against this run first (the fences' job):
/// the window compare is order-valid only for targets that share it.
fn shared_prefix_len(first: &[u8], last: &[u8]) -> usize {
    first.iter().zip(last).take_while(|(a, b)| a == b).count()
}

/// one node of the ordered structure
struct Node {
    /// bytes every key in this node shares; windows start past them
    off: usize,
    /// heap byte-string keys, sorted
    keys: Vec<Vec<u8>>,
    /// be_window(key, off) per key, stored BESIDE the key
    windows: Vec<u64>,
}

impl Node {
    fn build(mut keys: Vec<Vec<u8>>) -> Node {
        keys.sort();
        let off = match keys.as_slice() {
            [first, .., last] => shared_prefix_len(first, last),
            _ => 0,
        };
        let windows = keys.iter().map(|key| be_window(key, off)).collect();
        Node { off, keys, windows }
    }

    /// BUGGY arm: a full Ord compare on every probe -- the heap key is
    /// dereferenced and compared from byte 0 (the shared prefix
    /// re-compared) every time.
    fn search_full(&self, target: &[u8]) -> Result<usize, usize> {
        self.keys.binary_search_by(|key| key.as_slice().cmp(target))
    }

    /// FIXED arm: the node's SPAN is checked first -- the fences route
    /// the target into the node, so the windows are only ever asked to
    /// rank a target that shares the bytes the windows skip (a target
    /// diverging inside the skipped run orders by those bytes alone and
    /// never reaches the window compares); then the integer window
    /// compare, and the heap key is read only where windows tie.
    fn search_inline(&self, target: &[u8]) -> Result<usize, usize> {
        // fence routing: order the target against the node's span by
        // the shared run (the parent's fences' job in the real tree)
        match target.get(..self.off).cmp(&Some(&self.keys[0][..self.off])) {
            Ordering::Less => return Err(0),
            Ordering::Greater => return Err(self.keys.len()),
            Ordering::Equal => {}
        }
        let target_window = be_window(target, self.off);
        let mut lo = 0;
        let mut hi = self.keys.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let ord = match self.windows[mid].cmp(&target_window) {
                Ordering::Equal => self.keys[mid].as_slice().cmp(target),
                other => other,
            };
            match ord {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok(mid),
            }
        }
        Err(lo)
    }
}

/// tiny LCG, deterministic
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 33) as u8
    }
}

/// (a) THE THEOREM: cmp(window_a, window_b) never contradicts cmp(a, b).
/// Equal is allowed -- that is the tie the full compare resolves.
fn assert_window_order_matches(keys: &[Vec<u8>], off: usize) {
    for a in keys {
        for b in keys {
            match be_window(a, off).cmp(&be_window(b, off)) {
                Ordering::Equal => {}
                word_ord => assert_eq!(
                    word_ord,
                    a.cmp(b),
                    "window order contradicted key order: {a:?} vs {b:?} (off {off})",
                ),
            }
        }
    }
}

/// the full 2-byte space, exhaustively: sorting by integer window must
/// give exactly the numeric order
#[test]
fn u16_space_two_byte_windows_order_exactly_as_u16() {
    let keys: Vec<Vec<u8>> = (0..=u16::MAX)
        .map(|v| v.to_be_bytes().to_vec())
        .collect();
    let mut by_window: Vec<usize> = (0..keys.len()).collect();
    by_window.sort_by_key(|&i| be_window(&keys[i], 0));
    for (pos, &key_index) in by_window.iter().enumerate() {
        assert_eq!(
            key_index, pos,
            "window order diverged from numeric order at {pos}",
        );
    }
    // the 2-byte corners: zero, carries, sign boundaries, saturation
    let boundary: Vec<u16> = vec![
        0, 1, 2, 0x00ff, 0x0100, 0x7ffe, 0x7fff, 0x8000, 0x8001, 0xfffe, 0xffff,
    ];
    let boundary_keys: Vec<Vec<u8>> = boundary
        .iter()
        .map(|&v| v.to_be_bytes().to_vec())
        .collect();
    assert_window_order_matches(&boundary_keys, 0);
}

/// the sign-flip encoding orders i64s exactly as i64 Ord does
#[test]
fn i64_sign_flip_encoding_orders_as_i64() {
    let values: Vec<i64> = vec![
        i64::MIN,
        i64::MIN + 1,
        -(1i64 << 62),
        -(1i64 << 32) - 1,
        -(1i64 << 32),
        -257,
        -256,
        -1,
        0,
        1,
        255,
        256,
        (1i64 << 32) - 1,
        1i64 << 32,
        (1i64 << 62) - 1,
        1i64 << 62,
        i64::MAX - 1,
        i64::MAX,
    ];
    let keys: Vec<Vec<u8>> = values.iter().map(|&x| i64_encoding(x).to_vec()).collect();
    assert_window_order_matches(&keys, 0);
    for i in 0..values.len() {
        for j in 0..values.len() {
            assert_eq!(
                keys[i].cmp(&keys[j]),
                values[i].cmp(&values[j]),
                "sign-flip encoding broke order: {} vs {}",
                values[i],
                values[j],
            );
        }
    }
    // an LCG sweep, pairwise against the numeric order
    let mut rng = Lcg(0xfeed_beef_cafe_f00d);
    let sampled: Vec<i64> = std::iter::repeat_with(|| rng.next_u64() as i64)
        .take(2048)
        .collect();
    let sampled_keys: Vec<Vec<u8>> = sampled
        .iter()
        .map(|&x| i64_encoding(x).to_vec())
        .collect();
    for i in 0..sampled.len() {
        for j in 0..sampled.len() {
            assert_eq!(
                sampled_keys[i].cmp(&sampled_keys[j]),
                sampled[i].cmp(&sampled[j]),
            );
        }
    }
}

/// LCG byte strings under one long shared prefix: windows skip the
/// prefix, order is preserved, and both search arms agree everywhere
#[test]
fn lcg_shared_prefix_byte_strings_order_and_search_alike() {
    let mut rng = Lcg(42);
    let prefix = b"node/000042/chunk/".to_vec();
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for len in 0..=24usize {
        let mut key = prefix.clone();
        for _ in 0..len {
            key.push(rng.byte()); // includes 0x00 bytes on purpose
        }
        keys.push(key);
    }
    let node = Node::build(keys);
    // every key shares the whole literal prefix, so the node's windows
    // must start past it
    assert!(node.off >= prefix.len(), "prefix skip not taken: off={}", node.off);
    assert_window_order_matches(&node.keys, node.off);
    for key in &node.keys {
        assert_eq!(node.search_full(key), node.search_inline(key));
    }
    // absent probes from the same generator
    for _ in 0..64 {
        let mut probe = prefix.clone();
        for _ in 0..(rng.next_u64() % 26) {
            probe.push(rng.byte());
        }
        assert_eq!(
            node.search_full(&probe),
            node.search_inline(&probe),
            "probe {probe:?}",
        );
    }
}

/// shapes where one key prefixes another, and where the tail bytes are
/// 0x00 / 0xff: the zero-padding edge of the window contract
#[test]
fn one_key_prefixing_another_orders_and_searches_alike() {
    let keys: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        b"abc".to_vec(),
        b"abc\x00".to_vec(),
        b"abc\x00d".to_vec(),
        b"abcd".to_vec(),
        b"ab\x00".to_vec(),
        b"ab\xff".to_vec(),
        b"z".to_vec(),
    ];
    let node = Node::build(keys);
    assert_window_order_matches(&node.keys, node.off);
    let probes: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"aa".to_vec(),
        b"abb".to_vec(),
        b"abc\x00\x00".to_vec(),
        b"abc\x00e".to_vec(),
        b"abc\x00d\x00".to_vec(),
        b"abc\x00d!".to_vec(),
        b"abcd!".to_vec(),
        b"ab\x00\x00".to_vec(),
        b"ab\xff\x00".to_vec(),
        b"zz".to_vec(),
        vec![0xff],
    ];
    for probe in &probes {
        assert_eq!(
            node.search_full(probe),
            node.search_inline(probe),
            "probe {probe:?}",
        );
    }
}

/// (b) both arms return identical answers over an exhaustive-ish probe
/// matrix on a mixed node, and the window provably starts past the
/// shared bytes
#[test]
fn both_arms_return_identical_answers_across_the_matrix() {
    let mut keys: Vec<Vec<u8>> = Vec::new();
    for i in 0..200u32 {
        let mut key = b"shared/prefix/".to_vec();
        key.extend_from_slice(&(i as u16).to_be_bytes());
        if i % 3 == 0 {
            key.extend_from_slice(b"-longer-tail");
        }
        keys.push(key);
    }
    let node = Node::build(keys);
    // all 200 tails are below 0x0100, so the shared run runs one byte
    // INTO the tail: off = prefix.len() + 1
    assert_eq!(node.off, b"shared/prefix/".len() + 1);
    // every 2-byte tail value as a probe: present, absent, below-min,
    // above-max, and both zero bytes of every boundary
    for tail in 0..=u16::MAX {
        let mut probe = b"shared/prefix/".to_vec();
        probe.extend_from_slice(&tail.to_be_bytes());
        assert_eq!(
            node.search_full(&probe),
            node.search_inline(&probe),
            "probe tail {tail:#06x}",
        );
    }
}
"#
            .to_string(),
            context_notes: Some(
                "Grade B+ - NotBenchable on this validator (the author figures are measured on the real tree; the win is cache-miss-class on heap-backed keys inside a node - a fair microbench needs the node layout, the fences, the heap keys and the windows, so the decision content is the ENCODING law + the tie-rate regime, the Batch-136 decision-content class). SOURCE (XiangpengHao/bbtree @ 8adae97a0ed97e5c98fa2790312eb0aec9719846, MIT OR Apache-2.0, v0.1.5, edition-2024 no_std, zero runtime deps - vstd is Verus ghost-only, gone after compilation; every quote re-verified verbatim at the pin this batch). THE CONTRACT, source verbatim (inline_key.rs:69-70): 'The contract: every key has an encoding such that if `a`'s encoding sorts before `b`'s, both padded with zeros, then `a < b`.' - and inline_key_at(offset) is that encoding's bytes offset..offset + 8, big-endian, zero past its end, so each window is one u64 word and comparing windows integer-wise never CONTRADICTS the key order (Equal is the tie the full compare resolves; a target diverging inside a node's skipped shared run is ordered by those bytes alone - the fences route it, the parent never delivers an out-of-span target to the windows). Signed integers ride the sign-bit remap, source verbatim (inline_keys.rs:1142): 'An `i64`'s encoding is its bits with the sign bit flipped.' - x ^ (1<<63), big-endian; floats need the analogous remap (IEEE byte order is wrong for negatives) and a WRONG encoding is SILENTLY wrong - the encoding is a total-order contract, not a hash. THE WIDTH TWO-REGIME, source verbatim (inline_key.rs:34-35): 'Narrower inline keys pack more of a node's inline keys into each cache line, and compare in more vector lanes at once, but tie more often:' - real neighbours tie 24-70% at 8-byte windows vs 1-20% at 16 (author-measured over the source's own key sets); a tie costs exactly the heap read the rule exists to avoid, so WIDTH is a measured trade, not a default. THE PREFIX SKIP, source verbatim (prefix.rs:1-2): 'Per-node prefix stripping (D26, `design/05-fence-key.md`): a node with inline keys skips the leading bytes its two fences share, and stores each' key's window past them - in a sorted node every key starts with the extremes' shared run, so the skipped bytes never order anything inside the node and each window starts deeper in the key. AUTHOR BASIS (the tree's own decision data, D31/D50): u64-key hits 198->138 ns with the window compare; shared-prefix heap-string paths 547->495 ns; random-prefix keys pay 4-23% (window ties send them back to the heap read plus a wasted word compare). REGIME: ordered structures over long heap-backed byte strings with shared prefixes (paths, URLs, names, versioned ids) - the window turns per-probe heap derefs + shared-prefix re-compares into register integer compares; u64/int keys already fit the window. ANTI-REGIME: keys with no cheap order-preserving byte encoding (multi-field Ord tuples, floats without the sign-bit remap, user-defined Orders) - a wrong encoding silently mis-orders (correctness, not perf); tiny maps (the window build dwarfs a handful of compares); unsorted or rebuilt-per-query collections (windows are precomputed BESIDE the keys - insert-heavy structures must maintain them). DISTINCT-FROM (grep-verified in this corpus): bsearch-sorted-slice (entries_01) owns 'use binary_search on sorted' - the OPPOSITE direction (linear scan -> binary search), and this rule's anti-regime anchor: when keys stay on the full compare, binary search IS the right shape and only the compare payload changes here; align-64-false-sharing (entries_02) owns coherence-LINE padding - this rule owns the ORDER-PRESERVING ENCODING of the key material itself, not where the bytes sit relative to cache lines. THE COMPILE-TIME FACE: the compile-time hand-specialization face of this family (the dtolnay typeid trick) lands as a rider citation - see the batch's Research 322 typeid-hand-specialization row; the D28 kinship cite to loop-invariant-dispatch-resolution-hoist (entries_48) belongs on the same line (both move an invariant resolution off the per-probe path). Fixtures ORIGINAL (inline-key windows, sign-flip encoding and per-node prefix skip re-authored as plain-Rust stand-ins with neutral names; no bbtree code pasted); each arm compiled + executed standalone: rustc --edition 2021 --test entries52_rule1_fixture.rs (the law arm, the buggy arm as its oracle: 5 passed / 0 failed) and rule1_buggy_fixture.rs (the anti-pattern arm: 2 passed / 0 failed). CITE: Batch 298 / Plan 289 / Research 322."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
