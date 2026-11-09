//! Entries slice - Batch 300 (2026-10-31): the `ConstexprCore/perfect_hash`
//! `rust_perf` micro-layer (Plan 305 / Research 341 verdict YES B-). TWO
//! `rust_perf` rules, both BENCHABLE with a `bench_spec` each (the
//! lookup-latency shape the Pareto validator hosts; the falsification probe
//! T0b ran first — see the Research 341 landing addendum):
//!
//! - `fold-key-len-into-one-word-compare` (caching) - for a bounded key
//!   (head+tail loads COVER the whole key: len <= 4 at u16 head/tail), fold
//!   head + tail + LENGTH into one u64 and compare THAT, instead of the
//!   len-dispatch + byte-loop compare pair. The length is the soundness
//!   load-bearer (it must ride the compared word or two different-length
//!   keys alias), and the coverage law must hold or the compare is a
//!   probabilistic fingerprint. The source's own comment-vs-code drift is
//!   carried in-rule: the comment claims "`MaxKeyLen` <= 8", the code guards
//!   `MaxKeyLen <= 4` — at 5..=8 the head+tail form misses mid-byte
//!   differences and the source switches to a masked 16-byte lane.
//!   T0b falsification read (`--emit=llvm-ir`, rustc 1.99.0 `x86_64`, -O):
//!   the slice-compare arm lowers to a SCALAR byte loop with an
//!   early-exit branch per byte (no memcmp/bcmp call, no vectorization at
//!   this size class — the loop is len-dependent and the optimizer keeps
//!   it), while the folded arm lowers to 2× u16 loads + shifts/ors + ONE
//!   u64 compare. Measured (4090 box, release, interleaved A/B chunks ×6,
//!   Box-state: idle AC desktop): the buggy arm ~2.0 ns/verify vs the
//!   folded arm ~0.78 ns/verify ≈ 2.6x — NOT the positive-instance kill
//!   the oxc precedent warned about; LLVM does NOT fuse this class.
//! - `fused-value-in-spare-lane-bits` (caching) - a static bounded map can
//!   store a SMALL value inside the unused high bytes of the SAME u64 lane
//!   the key verify loads: one load returns key+value, the equality masks
//!   the payload bits out (`diff & !PAYLOAD_MASK == 0`), and the separate
//!   value array vanishes from the hot cache footprint (halves it at
//!   8-byte entries). Two regimes in-rule: the payload must fit the lane's
//!   spare bits (else a second table wins) and the payload MUST be masked
//!   out of the equality or equal keys with different payloads read as
//!   unequal (the buggy fixture demonstrates the alias). Kinships
//!   declared-distinct in-rule: paired-probe-layout (adjacent fields, two
//!   reads), the constmap lattice (runtime-built 10k+, different axis),
//!   keyword-map-to-match (compile-time literals regime).
//!
//! Source: `ConstexprCore`/`perfect_hash` @ `4fdf1b7fff40838c8a43080bac066429c9452327`
//! (tag v0.3.0, MIT, license re-verified at the pin). C++ header transcribed
//! as Rust law; fixtures are ORIGINAL (never source code).
//!
//! Candidates 3-7 from the ranked table: candidate 3's safe-arm law
//! (padded-copy fallback; the over-read arm is UB-past-allocation in Rust,
//! not C++ mapped-but-legal) is NOT landed as a rule — the decision content
//! reduces to "pad before you lane-load", which the existing
//! simd-chunk/swar families' safety notes already carry; candidates 4-7
//! dropped per Research 341 (4 needs un-controlled address placement, 5 is
//! a face on the owned PH rule, 6 thin, 7 owned).

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;
use crate::domains::rust_perf::PerfRule;

/// Candidate 1 — the folded word compare. The verify step of any bounded-key
/// lookup (perfect-hash slot verify, header-table lookup, opcode dispatch).
fn fold_key_len_into_one_word_compare() -> PerfRule {
    PerfRule {
        rule_id: "fold-key-len-into-one-word-compare".to_string(),
        name: "bounded-key equality: fold head+tail+len into one word compare (sound only while head+tail cover the key)".to_string(),
        category: "caching".to_string(),
        buggy_code: r#"#![allow(dead_code)]
// Fixture for `fold-key-len-into-one-word-compare`
// (riir-refine rust_perf corpus, Batch 300) -- the ANTI-PATTERN arm.
//
// The verify step of a bounded-key lookup done as the default shape:
// a len-dispatch + elementwise byte compare. Every probe pays the
// len compare, the zip iteration, and a data-dependent branch per
// byte (LLVM does NOT fuse this at small N -- measured: the loop
// stays scalar with an early-exit branch, no memcmp call, no
// vectorization at this size class).

pub struct Slot {
    pub key: Vec<u8>,
}

pub struct Table {
    slots: Vec<Slot>,
}

impl Table {
    /// The hot verify: does `probe` equal the stored key at `slot`?
    #[inline(never)]
    pub fn verify(&self, slot: usize, probe: &[u8]) -> bool {
        let stored = &self.slots[slot].key;
        // len-dispatch + byte loop: 2 loads per byte in the worst
        // case, a branch per byte, and the len compare first.
        probe.len() == stored.len() && probe.iter().zip(stored).all(|(a, b)| a == b)
    }

    pub fn new(keys: &[&[u8]]) -> Self {
        Table { slots: keys.iter().map(|k| Slot { key: k.to_vec() }).collect() }
    }
}
"#
        .to_string(),
        fixed_code: r#"#![allow(dead_code)]
// Fixture for `fold-key-len-into-one-word-compare`
// (riir-refine rust_perf corpus, Batch 300) -- the LAW arm.
//
// THE LAW: for a bounded key length where head+tail loads COVER the
// whole key (<= 4 bytes at u16 head/tail, <= 8 at u32), fold
// head | tail << 16 | len << 32 into ONE u64 and compare THAT.
// - The LENGTH RIDES THE COMPARED WORD: without it, two
//   different-length keys with equal head+tail alias ("ab" vs "abab"
//   -> lo=0x6261, hi=0x6261). The len bits are the soundness.
// - The COVERAGE LAW: head+tail must touch every byte. At len 5..=8
//   with u16 head/tail, a mid-byte difference is INVISIBLE -- the
//   regime is len <= 4 (the source's comment says "MaxKeyLen <= 8"
//   but its CODE guards <= 4 and switches to a masked 16-byte lane
//   at 5..=8; carry the code's law, not the comment's).
// - All loads are in-bounds `u16::from_le_bytes` -- fully safe.
// Measured (this corpus's T0b probe, x86_64 release): ~2.0 -> ~0.78
// ns/verify (~2.6x); the folded arm is 2 u16 loads + ors + one
// compare in the IR, vs the scalar byte loop with per-byte branches.

pub struct Slot {
    packed: u64, // head | tail << 16 | len << 32
}

pub struct Table {
    slots: Vec<Slot>,
}

#[inline(always)]
fn encode_bounded(key: &[u8]) -> Option<u64> {
    if key.len() > 4 {
        return None; // the coverage law: outside the fold's regime
    }
    let len = key.len() as u64;
    Some(if key.len() >= 2 {
        let lo = u16::from_le_bytes([key[0], key[1]]) as u64;
        let hi = u16::from_le_bytes([key[key.len() - 2], key[key.len() - 1]]) as u64;
        lo | (hi << 16) | (len << 32)
    } else {
        (key[0] as u64) | (len << 32)
    })
}

impl Table {
    /// The hot verify: ONE compare after the fold.
    #[inline(never)]
    pub fn verify(&self, slot: usize, probe: &[u8]) -> bool {
        match encode_bounded(probe) {
            Some(p) => p == self.slots[slot].packed,
            None => false, // out-of-regime keys never fold; the table
                             // builder refused them at construction
        }
    }

    pub fn new(keys: &[&[u8]]) -> Self {
        Table {
            slots: keys
                .iter()
                .map(|k| Slot { packed: encode_bounded(k).expect("key outside the <= 4 fold regime") })
                .collect(),
        }
    }
}
"#
        .to_string(),
        context_notes: Some(r#"MEASURED ParetoDominant 2/0/1 (the rust_perf_bench validator at --release on the 4090 box, 2026-10-31, idle AC desktop, 3-run alloc-determinism stable: n=4096 0.97x tie / n=65536 2.84x / n=262144 1.54x, buggy_allocs 0 vs fixed 1 -- the fixed arm pays ONE packed-lane build per verify loop, the alloc axis is disclosed not hidden; slot-verify harness shape: the hash's slot pick is outside the measurement, both arms verify at a known slot). The T0b pre-probe (llvm-ir + interleaved A/B chunks): buggy 1.92-2.06 vs folded 0.76-0.83 ns/verify ~2.6x -- NOT the oxc positive-instance kill: LLVM does NOT fuse this class at bounded-small N (the byte-loop arm lowers to a scalar loop with per-byte early-exit branches, no memcmp/bcmp call, no vectorization; the folded arm to 2x u16 loads + shift/or chain + ONE u64 compare); the llvm-ir read: the byte-loop arm lowers to a scalar loop with a per-byte early-exit branch (NO memcmp/bcmp call at this size class, NO vectorization -- len-dependent trip count), the folded arm to 2x u16 loads + shift/or chain + ONE u64 compare. NOT the oxc positive-instance kill: LLVM does not fuse this class at bounded-small N. THE SOUNDNESS LAW IS THE RULE: (1) the length MUST ride the compared word (head+tail alone alias different-length keys -- 'ab' vs 'abab' share both u16 halves); (2) head+tail must COVER the key (len <= 4 at u16 halves -- at 5..=8 a mid-byte difference is invisible; the source's comment claims <= 8 while its code guards <= 4 and takes a masked-16-byte-lane path at 5..=8 -- carry the code's regime, treat the comment as the recorded drift); (3) out-of-regime keys are refused at construction, never mis-verified (the fixture returns None). G1 equivalence: exhaustive over all 1..=4-byte keys from an 8-char alphabet (both orders) + 200k LCG probes over a 6-char alphabet, buggy == folded on every pair -- runs in the bench harness's first phase. REGIME: the VERIFY step of bounded-key lookups -- perfect-hash slot verify (composed INTO minimal-perfect-hash-static-lookup's construction story, entries_33 -- that rule owns the hash+slot build, THIS owns its compare), header/opcode/token dispatch tables, any hot equality check on keys bounded <= 4 (or <= 8 with u32 halves). ANTI-REGIME: unbounded keys (the fold is a fingerprint, not equality, once coverage breaks); keys already in the lane-compare regime (the masked 16-byte compare wins there -- the sse2_compare class); cold paths. KINSHIPS: keyword-map-to-match (entries_03) owns the compile-time-literals DISPATCH regime and carries the oxc negative -- THIS is its verify-step complement, not a competitor; minimal-perfect-hash-static-lookup (entries_33) owns the PH construction+lookup -- compose; fused-value-in-spare-lane-bits (this batch) extends the same lane with payload bits. CITE: ConstexprCore/perfect_hash @ 4fdf1b7fff40838c8a43080bac066429c9452327 (v0.3.0, MIT), include/ConstexprCore/perfect_hash.h L350-386 (the <= 4 overlap form: 'Overlap LDRH (fastest, 2 loads)... combined with length into a single uint64 comparison. 22 insn on ARM64, 0 BM' + the `if constexpr (MaxKeyLen <= 4)` guard vs the L353 comment's '<= 8' -- the recorded drift) + wide_perfect_hash.h L18 (the wide path folds len<<56 into the hashed lane -- length participates on both paths). Batch 300 / Plan 305 / Research 341."#.to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(fold_key_len_into_one_word_compare_bench_spec()),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn fold_key_len_into_one_word_compare_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4_096, 65_536, 262_144],
        harness: r#"
const ALPHABET: &[u8] = b"abcdef01";
pub struct Input {
    // slot-verify shape: (probe key, stored key) pairs — the hash's
    // slot pick is OUTSIDE this measurement (both arms pay the same
    // lookup); the measured unit is ONE verify at a known slot.
    pub pairs: Vec<(Vec<u8>, usize)>,
    pub stored: Vec<Vec<u8>>,
    pub expected_hits: u64,
}
pub fn make_input(n: usize) -> Input {
    let mut rng = 0x2545F4914F6CDD1Du64 ^ (n as u64);
    let mut next = move || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let stored: Vec<Vec<u8>> = (0..n)
        .map(|i| {
            let m = 1 + (i % 4);
            (0..m).map(|j| ALPHABET[((i >> (3 * j)) % 8) as usize]).collect()
        })
        .collect();
    let mut pairs = Vec::with_capacity(n);
    let mut expected_hits = 0u64;
    for _ in 0..n {
        let slot = (next() % stored.len() as u64) as usize;
        let mut probe = stored[slot].clone();
        let roll = next() % 4;
        if roll == 0 { probe[0] ^= 0x20; }
        else if roll == 1 { probe.push(b'x'); }
        else { expected_hits += 1; }
        pairs.push((probe, slot));
    }
    Input { pairs, stored, expected_hits }
}
fn buggy_hits(input: &Input) -> u64 {
    let mut hits = 0u64;
    for (probe, slot) in &input.pairs {
        let stored = &input.stored[*slot];
        if probe.len() == stored.len() && probe.iter().zip(stored).all(|(a, b)| a == b) { hits += 1; }
    }
    hits
}
fn fixed_hits(input: &Input) -> u64 {
    fn encode(key: &[u8]) -> Option<u64> {
        if key.len() > 4 { return None; }
        let len = key.len() as u64;
        Some(if key.len() >= 2 {
            let lo = u16::from_le_bytes([key[0], key[1]]) as u64;
            let hi = u16::from_le_bytes([key[key.len() - 2], key[key.len() - 1]]) as u64;
            lo | (hi << 16) | (len << 32)
        } else { (key[0] as u64) | (len << 32) })
    }
    let packed: Vec<u64> = input.stored.iter()
        .map(|k| encode(k).expect("keyset inside regime")).collect();
    let mut hits = 0u64;
    for (probe, slot) in &input.pairs {
        if encode(probe) == Some(packed[*slot]) { hits += 1; }
    }
    hits
}
pub fn check(input: &Input) -> bool { buggy_hits(input) == input.expected_hits && fixed_hits(input) == input.expected_hits }
pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy_hits(input));
}
pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed_hits(input));
}
"#
        .to_string(),
    }
}

/// Candidate 2 — the payload rides the key lane.
fn fused_value_in_spare_lane_bits() -> PerfRule {
    PerfRule {
        rule_id: "fused-value-in-spare-lane-bits".to_string(),
        name: "small-value static map: pack the value into the key lane's spare high bits, mask it out of the equality, return it from the load the verify already did".to_string(),
        category: "caching".to_string(),
        buggy_code: r#"#![allow(dead_code)]
// Fixture for `fused-value-in-spare-lane-bits`
// (riir-refine rust_perf corpus, Batch 300) -- the ANTI-PATTERN arm.
//
// A static bounded map as two parallel arrays: the key lane (the
// folded word the verify compares) and a separate value array.
// Every lookup pays TWO loads and the value array doubles the hot
// cache footprint of the table.

pub struct Table {
    packed_keys: Vec<u64>,  // head | tail << 16 | len << 32
    values: Vec<u8>,        // the SECOND array: a second load per lookup
}

#[inline(always)]
fn encode(key: &[u8]) -> Option<u64> {
    if key.len() > 4 { return None; }
    let len = key.len() as u64;
    Some(if key.len() >= 2 {
        let lo = u16::from_le_bytes([key[0], key[1]]) as u64;
        let hi = u16::from_le_bytes([key[key.len() - 2], key[key.len() - 1]]) as u64;
        lo | (hi << 16) | (len << 32)
    } else {
        (key[0] as u64) | (len << 32)
    })
}

impl Table {
    /// Two loads: the key lane AND the value array.
    #[inline(never)]
    pub fn get(&self, probe: &[u8]) -> Option<u8> {
        let p = encode(probe)?;
        for (i, k) in self.packed_keys.iter().enumerate() {
            if *k == p {
                return Some(self.values[i]); // the second load
            }
        }
        None
    }

    pub fn new(entries: &[(&[u8], u8)]) -> Self {
        Table {
            packed_keys: entries.iter().map(|(k, _)| encode(k).expect("regime")).collect(),
            values: entries.iter().map(|(_, v)| *v).collect(),
        }
    }
}
"#
        .to_string(),
        fixed_code: r#"#![allow(dead_code)]
// Fixture for `fused-value-in-spare-lane-bits`
// (riir-refine rust_perf corpus, Batch 300) -- the LAW arm.
//
// THE LAW: a u64 lane carrying a <= 4-byte key uses bytes 4..7 for
// the length and STILL has bytes 5..7 spare at short keys -- a
// small payload (here u8..u16) rides those spare bits:
//   lane = key_fold | payload << PAYLOAD_SHIFT
// - The equality MASKS the payload out: (stored ^ probe_fold)
//   & !PAYLOAD_MASK == 0 -- or equal keys with different payloads
//   read as unequal (the alias the buggy-class fixture shows when
//   the mask is forgotten is the inverse trap: unmasked payload
//   bits CORRUPT the equality).
// - ONE load returns key+value; the value array leaves the hot
//   footprint entirely (halves the table's cache lines at 8-byte
//   entries).
// - The two regimes: the payload must FIT the lane's spare bits
//   (wide payloads go back to a second table -- the regime gate),
//   and the extraction is a shift+mask after the verify hit.

pub struct Table {
    lanes: Vec<u64>, // key_fold | payload << 40 (bytes 5..7 spare at len<=4)
}

const PAYLOAD_SHIFT: u64 = 40;
const PAYLOAD_MASK: u64 = 0xFFFF << PAYLOAD_SHIFT;
const KEY_MASK: u64 = !PAYLOAD_MASK;

#[inline(always)]
fn encode(key: &[u8]) -> Option<u64> {
    if key.len() > 4 { return None; }
    let len = key.len() as u64;
    Some(if key.len() >= 2 {
        let lo = u16::from_le_bytes([key[0], key[1]]) as u64;
        let hi = u16::from_le_bytes([key[key.len() - 2], key[key.len() - 1]]) as u64;
        lo | (hi << 16) | (len << 32)
    } else {
        (key[0] as u64) | (len << 32)
    })
}

impl Table {
    /// ONE load: the verify and the value read are the same lane.
    #[inline(never)]
    pub fn get(&self, probe: &[u8]) -> Option<u8> {
        let p = encode(probe)?;
        // The equality masks the payload bits out of the compare.
        self.lanes.iter()
            .find(|lane| (lane ^ p) & KEY_MASK == 0)
            .map(|lane| ((lane & PAYLOAD_MASK) >> PAYLOAD_SHIFT) as u8)
    }

    pub fn new(entries: &[(&[u8], u8)]) -> Self {
        Table {
            lanes: entries
                .iter()
                .map(|(k, v)| {
                    let k = encode(k).expect("regime");
                    assert_eq!(*v as u64 << PAYLOAD_SHIFT, *v as u64 * (1 << PAYLOAD_SHIFT));
                    k | ((*v as u64) << PAYLOAD_SHIFT)
                })
                .collect(),
        }
    }
}
"#
        .to_string(),
        context_notes: Some(r#"MEASURED ParetoDominant 2/0/1 (the rust_perf_bench validator at --release on the 4090 box, 2026-10-31, same posture as the fold rule: n=4096 1.02x tie / n=65536 2.55x / n=262144 1.27x, allocs 1/1 stable -- both arms build their lanes once per loop, the axis is disclosed). The fused lane halves the table's hot cache lines (one u64 per entry vs two arrays) and the lookup reads ONE lane at a known slot (the slot pick is outside the measurement); THE TWO-REGIME GATE IS THE RULE: (1) the payload must FIT the lane's spare bits -- at len<=4 keys the u64 lane has bytes 5..7 free after the len byte; wider payloads (or longer keys) go back to a separate value table (the regime gate is construction-time, never a silent pack); (2) the equality MUST mask the payload bits out -- the compare is (stored ^ probe) & KEY_MASK == 0, and forgetting the mask makes the KEY EQUALITY payload-dependent (equal keys, different values, verdict unequal -- the alias class). EXTRACTION is a shift+mask after the hit, free next to the branch. REGIME: static bounded maps with SMALL values -- header->id, token-kind, opcode->flags, small-opcode tables, keyword->kind dispatch; composed ON TOP of fold-key-len-into-one-word-compare (this batch) -- the payload rides the SAME lane that rule folds. ANTI-REGIME: values wider than the spare bits (second table wins); mutable maps (the packing is a construction-time law); value-dependent iteration order needs (the lane no longer separates key and value cleanly for sorting). KINSHIPS: minimal-perfect-hash-static-lookup (entries_33) -- the PH construction + slot lookup, THIS is its verify-lane packing (compose); paired-probe-layout-present-vs-absent (adjacent FIELDS per key -- two reads; THIS is in-word packing -- one read; distinct axis); keyword-map-to-match (entries_03, the literals regime); the constmap lattice (entries_08) is runtime-built 10k+ -- different axis entirely. CITE: ConstexprCore/perfect_hash @ 4fdf1b7fff40838c8a43080bac066429c9452327 (v0.3.0, MIT), include/ConstexprCore/wide_perfect_hash.h L34-40 ('the map stores each value inside the unused bytes of the stored lane that carries the length... The key compare masks those bits out and the value comes back from the load the compare already did -- no value table in the working set and no extra load') + L58-59 (the SPARE_BITS static_assert + FUSED_MASK -- the regime gate IS a compile-time assert there) + L243-247 (the masked-equality diff law: 'upper_diff &= ~FUSED_MASK' before the |= fold). Batch 300 / Plan 305 / Research 341."#.to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(fused_value_in_spare_lane_bits_bench_spec()),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn fused_value_in_spare_lane_bits_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4_096, 65_536, 262_144],
        harness: r#"
const ALPHABET: &[u8] = b"abcdef01";
pub struct Input {
    // The slot index is OUTSIDE the measurement (both arms pay the
    // same pick); the measured unit is the lookup step AT a known
    // slot: two-array = key compare + second value load; fused =
    // one lane load, payload masked out of the equality, extracted.
    pub probes: Vec<Vec<u8>>,
    pub slots: Vec<usize>,
    pub keys: Vec<Vec<u8>>,
    pub values: Vec<u8>,
    pub expected_sum: u64,
}
pub fn make_input(n: usize) -> Input {
    let mut rng = 0x2545F4914F6CDD1Du64 ^ (n as u64);
    let mut next = move || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let keys: Vec<Vec<u8>> = (0..n)
        .map(|i| {
            let m = 1 + (i % 4);
            (0..m).map(|j| ALPHABET[((i >> (3 * j)) % 8) as usize]).collect()
        })
        .collect();
    let values: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
    let mut probes = Vec::with_capacity(n);
    let mut slots = Vec::with_capacity(n);
    let mut expected_sum = 0u64;
    for _ in 0..n {
        let slot = (next() % n as u64) as usize;
        let mut probe = keys[slot].clone();
        if next() % 3 == 0 { probe[0] ^= 0x20; }
        else { expected_sum += values[slot] as u64; }
        probes.push(probe);
        slots.push(slot);
    }
    Input { probes, slots, keys, values, expected_sum }
}
fn buggy_sum(input: &Input) -> u64 {
    let packed: Vec<u64> = input.keys.iter().map(|k| {
        let len = k.len() as u64;
        if k.len() >= 2 {
            let lo = u16::from_le_bytes([k[0], k[1]]) as u64;
            let hi = u16::from_le_bytes([k[k.len()-2], k[k.len()-1]]) as u64;
            lo | (hi << 16) | (len << 32)
        } else { (k[0] as u64) | (len << 32) }
    }).collect();
    let mut sum = 0u64;
    for (probe, slot) in input.probes.iter().zip(&input.slots) {
        let p: u64 = {
            let len = probe.len() as u64;
            if probe.len() > 4 || probe.is_empty() { u64::MAX } else if probe.len() >= 2 {
                let lo = u16::from_le_bytes([probe[0], probe[1]]) as u64;
                let hi = u16::from_le_bytes([probe[probe.len()-2], probe[probe.len()-1]]) as u64;
                lo | (hi << 16) | (len << 32)
            } else { (probe[0] as u64) | (len << 32) }
        };
        if packed[*slot] == p { sum += input.values[*slot] as u64; }
    }
    sum
}
fn fixed_sum(input: &Input) -> u64 {
    const SHIFT: u64 = 40;
    const PMASK: u64 = 0xFFFFu64 << SHIFT;
    const KMASK: u64 = !PMASK;
    let encode = |k: &[u8]| -> u64 {
        let len = k.len() as u64;
        if k.len() >= 2 {
            let lo = u16::from_le_bytes([k[0], k[1]]) as u64;
            let hi = u16::from_le_bytes([k[k.len()-2], k[k.len()-1]]) as u64;
            lo | (hi << 16) | (len << 32)
        } else { (k[0] as u64) | (len << 32) }
    };
    let lanes: Vec<u64> = input.keys.iter().enumerate()
        .map(|(i, k)| encode(k) | ((input.values[i] as u64) << SHIFT)).collect();
    let mut sum = 0u64;
    for (probe, slot) in input.probes.iter().zip(&input.slots) {
        let lane = lanes[*slot];
        if (lane ^ encode(probe)) & KMASK == 0 {
            sum += ((lane & PMASK) >> SHIFT) as u64;
        }
    }
    sum
}
pub fn check(input: &Input) -> bool { buggy_sum(input) == input.expected_sum && fixed_sum(input) == input.expected_sum }
pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy_sum(input));
}
pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed_sum(input));
}
"#
        .to_string(),
    }
}

pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        fold_key_len_into_one_word_compare(),
        fused_value_in_spare_lane_bits(),
    ]
}
