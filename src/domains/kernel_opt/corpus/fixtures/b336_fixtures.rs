//! Batch 336 (Plan 345) fixtures — the five `entries_gpu_149` rules as ORIGINAL
//! minimal Rust reproductions (nothing transcribed from the sources: the loopvq
//! Triton/Python shapes and the vllm-rlt Python/Triton shapes are re-EXTRESSED
//! as original Rust; the laws are the sources' own landed decisions). Must be
//! standalone-compiled AND executed green BEFORE entry:
//! `rustc --edition 2021 --test b336_fixtures.rs` (run in this directory).
//!
//! Laws pinned:
//! 1. `chunked-argmax-strict-gt-tie-identical` — chunking a materialize-then-argmax
//!    scoring pass bounds the intermediate while every code's index is bit-identical:
//!    per-entry ops unchanged, the cross-chunk update is STRICT >, so the first-index
//!    tie rule of the full argmax holds exactly (crafted ties assert it).
//! 2. `repeat-axis-aliased-index-arena` — one arena row set per JOINT code, aliased by
//!    the Tc engine slots through the existing indirection: consumers read by slot
//!    index unchanged, arena bytes fold by Tc (the Lc*D/c == L*D/G pin), storage
//!    counted once.
//! 3. `load-time-decode-blind-transform-absorption` — every quantization-space
//!    transform (joint-step coupling, sensitivity scale) is absorbed into the
//!    per-slot DECODE table at load; the decode path consumes plain coordinates and
//!    its code never mentions the features; decode outputs bit-identical.
//! 4. `graph-fallback-coverage-ladder` — derived bucket coverage from reachable row
//!    counts, pre-captured, round-UP selection, padding rows carried then skipped by
//!    an active-row predicate, fallbacks counted loud: zero eager submissions for
//!    reachable shapes, and the per-shape arm PROVES the cliff fires.
//! 5. `i64-index-promote-before-stride-multiply` — a byte address `index * stride`
//!    wraps in 32-bit index arithmetic once the pool exceeds 2^31 bytes; the promote
//!    must happen on the index BEFORE the multiply (the wrap-mode arm models release
//!    arithmetic; debug overflow checks would panic instead — the wrap is the
//!    release-mode hazard the rule closes).
//!
//! Pure Rust, no external crates, deterministic.

#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]

/// R1 shared: a synthetic scorer. rows = queries, cols = codebook entries.
/// Scores are deterministic f32 values in [-2, 2]; no RNG.
fn scores_full(rows: usize, cols: usize) -> Vec<Vec<f32>> {
    (0..rows)
        .map(|r| {
            (0..cols)
                .map(|c| {
                    let bits = (r.wrapping_mul(2654435761) ^ c.wrapping_mul(40503)) & 0xFFFF;
                    ((bits as f32 / 65535.0) * 4.0) - 2.0
                })
                .collect::<Vec<f32>>()
        })
        .collect()
}

/// R1 naive: materialize every score for the row, then a full argmax with the
/// first-index tie rule (strict > against the running best).
fn argmax_full(row: &[f32]) -> (usize, f32) {
    let mut best = f32::NEG_INFINITY;
    let mut idx = 0usize;
    for (c, &s) in row.iter().enumerate() {
        if s > best {
            best = s;
            idx = c;
        }
    }
    (idx, best)
}

/// R1 opt: chunked scan — the chunk-local argmax with the same per-entry ops,
/// then a cross-chunk update with STRICT > so the earliest chunk keeps ties.
fn argmax_chunked(row: &[f32], chunk: usize) -> (usize, f32) {
    let mut best = f32::NEG_INFINITY;
    let mut idx = 0usize;
    let mut k0 = 0usize;
    while k0 < row.len() {
        let end = (k0 + chunk).min(row.len());
        let mut loc_val = f32::NEG_INFINITY;
        let mut loc_arg = k0;
        for c in k0..end {
            let s = row[c];
            if s > loc_val {
                loc_val = s;
                loc_arg = c;
            }
        }
        // strict >: on equal values the EARLIER chunk's index wins, matching
        // the full scan's first-index rule exactly.
        if loc_val > best {
            best = loc_val;
            idx = loc_arg;
        }
        k0 = end;
    }
    (idx, best)
}

#[test]
fn r1_chunked_argmax_tie_identity() {
    let rows = 16usize;
    let cols = 512usize;
    let m = scores_full(rows, cols);

    // (a) general identity: every row's chunked index == full index at chunk
    //     sizes 1, 7, 128, 512 (including chunk == cols).
    for chunk in [1usize, 7, 128, 512] {
        for r in 0..rows {
            let full = argmax_full(&m[r]);
            let chunked = argmax_chunked(&m[r], chunk);
            assert_eq!(full.0, chunked.0, "row {r} index drift at chunk {chunk}");
            assert_eq!(full.1.to_bits(), chunked.1.to_bits(), "row {r} value drift");
        }
    }

    // (b) the tie law: plant exact ties spanning chunk boundaries and assert
    //     the FIRST index wins in both arms.
    let mut tied = vec![0.5f32; 96];
    tied[3] = 2.0;
    tied[70] = 2.0; // tie across the chunk-64 boundary
    let full = argmax_full(&tied);
    let chunked = argmax_chunked(&tied, 64);
    assert_eq!(full.0, 3, "full argmax must keep the first index");
    assert_eq!(chunked.0, 3, "chunked argmax must keep the first index");

    // (c) a tie inside ONE chunk resolves the same way (the part-(b) tie at
    //     index 3 is reset first so 5 and 9 are the only maxima).
    tied[3] = 0.5;
    tied[70] = 0.0;
    tied[5] = 2.0;
    tied[9] = 2.0; // both inside chunk 0..64
    assert_eq!(argmax_chunked(&tied, 64).0, 5);
    assert_eq!(argmax_full(&tied).0, 5);
}

/// R2 shared: a slot = ut*Lb + l (ut = loop step, l = base layer). A joint
/// code covers Tc slots of one base layer: the code layer is (ut/Tc)*Lb + l,
/// block-major.
fn code_layer_of(slot: usize, lb: usize, tc: usize) -> usize {
    let (ut, l) = (slot / lb, slot % lb);
    (ut / tc) * lb + l
}

/// R2 naive: one arena row per ENGINE SLOT (Lb*T rows).
fn arena_naive(lb: usize, t: usize, row_words: usize) -> Vec<u64> {
    vec![0u64; lb * t * row_words]
}

/// R2 opt: ONE arena row set per CODE layer (Lc = Lb*T/Tc rows); the Tc slots
/// of a block alias it. Slot reads go through the SAME slot-indexed
/// indirection (code_layer_of), so consumers keep their addressing.
fn arena_aliased(lb: usize, t: usize, tc: usize, row_words: usize) -> Vec<u64> {
    let lc = lb * t / tc;
    vec![0u64; lc * row_words]
}

#[test]
fn r2_repeat_axis_aliased_arena() {
    let lb = 4usize; // base layers
    let t = 8usize; // loop steps
    let tc = 4usize; // steps jointly coded
    let words = 6usize; // packed index row width
    let slots = lb * t;

    // (a) the byte pin: aliased arena == naive arena / Tc (the redundancy fold).
    let naive = arena_naive(lb, t, words);
    let aliased = arena_aliased(lb, t, tc, words);
    assert_eq!(aliased.len() * tc, naive.len(), "Lc*D/c == L*D/G fold failed");

    // (b) consumer invariance: a writer that writes the block's index through
    //     the slot indirection lands the same bytes whichever arena backs it,
    //     and ALL Tc slots of the block read the same row.
    let code = 0xA5A5_1234u64; // the packed index a block's encode writes
    let mut naive_idx = naive;
    let mut alias_idx = aliased;
    for slot in 0..slots {
        let cl = code_layer_of(slot, lb, tc);
        // naive: every slot's own row
        for w in 0..words {
            naive_idx[slot * words + w] = code;
        }
        // aliased: the code layer's shared row (written once per block by the
        // encoder; reads come through the same slot indirection)
        for w in 0..words {
            alias_idx[cl * words + w] = code;
        }
    }
    for slot in 0..slots {
        let cl = code_layer_of(slot, lb, tc);
        let naive_row = &naive_idx[slot * words..(slot + 1) * words];
        let alias_row = &alias_idx[cl * words..(cl + 1) * words];
        assert_eq!(naive_row, alias_row, "slot {slot} read through the alias diverged");
    }

    // (c) block-major layer map: slots sharing a code layer are exactly the
    //     Tc slots of one (ut/Tc, l) block.
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); lb * t / tc];
    for slot in 0..slots {
        groups[code_layer_of(slot, lb, tc)].push(slot);
    }
    for g in groups {
        assert_eq!(g.len(), tc, "every code layer owns exactly Tc slots");
        let first_layer = g[0] % lb;
        let first_block = g[0] / lb / tc;
        for s in &g {
            assert_eq!(s % lb, first_layer, "same base layer");
            assert_eq!(s / lb / tc, first_block, "same block");
        }
    }
}

/// R3 shared: a "bundle" = joint codebook over (channel, steps) with a
/// per-slot sensitivity scale. The decode table is built by ABSORBING both:
/// per-slot slices (the stride-Tc share) pre-divided by the scale.
#[derive(Clone)]
struct Bundle {
    /// coupled entries: [code_layer][code][j], j = channel*Tc + step
    coupled: Vec<Vec<Vec<f32>>>,
    /// per-slot scale s (positive); codes live in the s*x space
    scale: Vec<Vec<f32>>, // [slot][channel]
    tc: usize,
    channels: usize,
    codes: usize,
    lb: usize,
    t: usize,
}

impl Bundle {
    fn slots(&self) -> usize {
        self.lb * self.t
    }
    fn code_layer_of(&self, slot: usize) -> usize {
        code_layer_of(slot, self.lb, self.tc)
    }
    fn step_of(&self, slot: usize) -> usize {
        (slot / self.lb) % self.tc
    }

    /// R3 NAIVE decode: hand the kernel the COUPLED table + the scale; the
    /// decode path must know Tc (stride-Tc addressing) AND undo s per channel
    /// at every read.
    fn decode_aware(&self, slot: usize, code: usize) -> Vec<f32> {
        let cl = self.code_layer_of(slot);
        let step = self.step_of(slot);
        let entry = &self.coupled[cl][code];
        let mut out = vec![0.0f32; self.channels];
        for ch in 0..self.channels {
            let s = self.scale[slot][ch];
            out[ch] = entry[ch * self.tc + step] / s;
        }
        out
    }

    /// R3 OPT decode table: built ONCE at load — per-slot slices, pre-divided.
    fn decode_table_blind(&self) -> Vec<Vec<Vec<f32>>> {
        let mut tables = Vec::new();
        for slot in 0..self.slots() {
            let cl = self.code_layer_of(slot);
            let step = self.step_of(slot);
            let mut per_slot = Vec::with_capacity(self.codes);
            for code in 0..self.codes {
                let entry = &self.coupled[cl][code];
                let mut row = vec![0.0f32; self.channels];
                for ch in 0..self.channels {
                    let s = self.scale[slot][ch];
                    row[ch] = entry[ch * self.tc + step] / s;
                }
                per_slot.push(row);
            }
            tables.push(per_slot);
        }
        tables
    }
}

/// The R3 "decode kernel": consumes a plain [slot][code] -> channels table.
/// It mentions NEITHER the coupling NOR the scale — that is the blindness pin.
fn decode_kernel_blind(table: &[Vec<Vec<f32>>], slot: usize, code: usize) -> Vec<f32> {
    table[slot][code].clone()
}

#[test]
fn r3_decode_blind_absorption_bit_identity() {
    let lb = 4usize;
    let t = 8usize;
    let tc = 4usize;
    let channels = 8usize;
    let codes = 16usize;
    let slots = lb * t;

    // deterministic coupled entries + positive scales
    let coupled: Vec<Vec<Vec<f32>>> = (0..lb * t / tc)
        .map(|cl| {
            (0..codes)
                .map(|code| {
                    (0..channels * tc)
                        .map(|j| {
                            let bits = (cl
                                .wrapping_mul(97)
                                ^ code.wrapping_mul(31)
                                ^ j.wrapping_mul(7))
                                & 0x3FF;
                            (bits as f32 / 1024.0) * 3.0 - 1.5
                        })
                        .collect::<Vec<f32>>()
                })
                .collect::<Vec<Vec<f32>>>()
        })
        .collect();
    let scale: Vec<Vec<f32>> = (0..slots)
        .map(|slot| {
            (0..channels)
                .map(|ch| {
                    let bits = (slot.wrapping_mul(13) ^ ch.wrapping_mul(5)) & 0xF;
                    0.5 + bits as f32 / 16.0 // in (0.5, 1.5), always positive
                })
                .collect::<Vec<f32>>()
        })
        .collect();

    let b = Bundle { coupled, scale, tc, channels, codes, lb, t };

    // (a) bit-identity: the blind kernel's outputs equal the aware kernel's
    //     for every (slot, code) — exact bit patterns, not approximations.
    let blind = b.decode_table_blind();
    for slot in 0..slots {
        for code in 0..codes {
            let aware = b.decode_aware(slot, code);
            let plain = decode_kernel_blind(&blind, slot, code);
            for ch in 0..channels {
                assert_eq!(
                    aware[ch].to_bits(),
                    plain[ch].to_bits(),
                    "slot {slot} code {code} ch {ch} drifted"
                );
            }
        }
    }

    // (b) the blindness pin: the blind table's SHAPE carries no Tc factor —
    //     per-slot geometry is (codes, channels) = the ordinary geometry.
    assert_eq!(blind.len(), slots);
    assert_eq!(blind[0].len(), codes);
    assert_eq!(blind[0][0].len(), channels);

    // (c) the table bytes fold too: the coupled table has Lc rows; the blind
    //     per-slot views alias them — the loader builds Lb*T views over Lc rows.
    assert_eq!(b.coupled.len(), lb * t / tc);
}

/// R4 shared: a shape-keyed graph cache. Keys are row counts.
struct GraphCacheLadder {
    buckets: Vec<usize>, // derived, ascending, PRE-CAPTURED
    captured: usize,
    fallbacks: usize,
}

impl GraphCacheLadder {
    /// The ladder: retain the largest preferred (reachable) sizes within the
    /// budget, then halve down from the smallest retained size while budget
    /// remains — coverage derived from the workload's reachable rows.
    fn derive(preferred: &[usize], cap: usize) -> Vec<usize> {
        let mut pref: Vec<usize> = preferred.iter().copied().filter(|s| *s > 0).collect();
        pref.sort_unstable();
        pref.dedup();
        let sizes: Vec<usize> = if pref.is_empty() {
            vec![1]
        } else {
            pref[pref.len().saturating_sub(cap)..].to_vec()
        };
        let mut out = sizes.clone();
        let mut size = out[0];
        while size > 1 && out.len() < cap {
            size = (size + 1) / 2;
            out.insert(0, size);
        }
        out
    }

    fn new(preferred: &[usize], cap: usize) -> Self {
        Self { buckets: Self::derive(preferred, cap), captured: 0, fallbacks: 0 }
    }

    /// R4 naive: one bucket per exact shape, captured lazily, full => eager.
    fn select_naive(shapes_seen: &[usize], cap: usize) -> usize {
        let mut entries: Vec<usize> = Vec::new();
        let mut fallbacks = 0usize;
        for &s in shapes_seen {
            if !entries.contains(&s) {
                if entries.len() >= cap {
                    fallbacks += 1;
                } else {
                    entries.push(s);
                }
            }
        }
        fallbacks
    }

    /// R4 opt: round-UP bucket selection + pre-captured coverage + counted
    /// fallback (count > largest bucket is the only remaining fallback).
    fn select(&mut self, count: usize) -> Option<usize> {
        match self.buckets.last() {
            Some(&top) if count > 0 && count <= top => {
                let bucket =
                    self.buckets.iter().find(|&&b| count <= b).copied().unwrap_or(count);
                if bucket > self.captured {
                    self.captured += 1; // pre-captured in prepare(); replay here
                }
                Some(bucket)
            }
            _ => {
                self.fallbacks += 1;
                None
            }
        }
    }
}

#[test]
fn r4_graph_fallback_coverage_ladder() {
    // The workload: staggered arrivals covering every row count 1..=32.
    let reachable: Vec<usize> = (1..=32).collect();

    // (a) THE CLIFF (naive, cap 16): distinct shapes 1..=16 fit; every later
    //     DISTINCT shape falls back eager — 16 fallbacks on distinct shapes.
    let naive_fallbacks = GraphCacheLadder::select_naive(&reachable, 16);
    assert_eq!(naive_fallbacks, 16, "per-shape cache must cliff on the tail");

    // (b) DERIVED COVERAGE (opt, cap 16): the ladder from the reachable set
    //     pre-captures; round-UP selection covers EVERY reachable row count.
    let mut cache = GraphCacheLadder::new(&reachable, 16);
    assert!(cache.buckets.len() <= 16);
    assert_eq!(*cache.buckets.last().unwrap(), 32, "the max reachable shape is retained");
    for count in 1..=32 {
        let bucket = cache.select(count).expect("no fallback inside coverage");
        assert!(bucket >= count, "round-up: bucket must cover the count");
    }
    assert_eq!(cache.fallbacks, 0, "zero fallbacks for reachable shapes");

    // (c) the padding contract: rows ABOVE the active count are carried
    //     through capture and SKIPPED at execution by the active predicate.
    let active = 3usize;
    let padded_rows: Vec<usize> = (0..8).collect();
    let written: Vec<usize> = padded_rows
        .iter()
        .copied()
        .filter(|&row| row < active) // the device-side active-row predicate
        .collect();
    assert_eq!(written, vec![0, 1, 2], "padding rows must not write");

    // (d) the loud cliff remains: a shape beyond coverage counts a fallback
    //     (the health metric), never a silent wrong answer.
    assert!(cache.select(64).is_none());
    assert_eq!(cache.fallbacks, 1);

    // (e) the halving law on a clean ladder: one preferred shape (the max
    //     reachable) + a budget of 4 derives [4, 8, 16, 32] — the budget
    //     spends DOWNWARD from the retained set by successive halves.
    assert_eq!(GraphCacheLadder::derive(&[32], 4), vec![4, 8, 16, 32]);
    let b = &cache.buckets;
    for w in b.windows(2) {
        assert!(w[0] <= w[1], "buckets ascend");
    }
}

/// R5: byte address = block_index * block_bytes + offset. Blocks live in a
/// pool of 2^31+ bytes. `wrapping_mul` MODELS release-mode i32 arithmetic
/// (debug overflow checks would panic — the wrap is the hazard class).
fn address_i32_wrapping(block: i32, block_bytes: i32, offset: i32) -> i32 {
    block.wrapping_mul(block_bytes).wrapping_add(offset)
}

fn address_i64(block: i64, block_bytes: i64, offset: i64) -> i64 {
    block * block_bytes + offset
}

#[test]
fn r5_i64_index_promote_before_stride_multiply() {
    // A 27B-class KV pool: 2^16 blocks x 2^16 bytes = 2^32 bytes — far
    // beyond the 32-bit byte-address boundary.
    let blocks: i64 = 1 << 16;
    let block_bytes: i64 = 1 << 16;
    assert_eq!(blocks * block_bytes, 1i64 << 32, "pool exceeds the i32 boundary");

    // (a) the wrap: the LAST block's address wraps negative in 32-bit math.
    let last = (blocks - 1) as i32;
    let bb = block_bytes as i32;
    let wrapped = address_i32_wrapping(last, bb, 0);
    assert!(wrapped < 0, "release-mode i32 wrap (modeled by wrapping_mul)");

    // (b) promote BEFORE the multiply: index -> i64 first, then stride.
    let exact = address_i64(last as i64, block_bytes, 0);
    assert_eq!(exact, (1i64 << 32) - block_bytes, "the true byte address");

    // (c) the too-late form: promoting AFTER a 32-bit multiply is already
    //     wrong — the wrap happened inside the multiply.
    let too_late = (last.wrapping_mul(bb)) as i64;
    assert_ne!(too_late, exact, "post-multiply promotion cannot recover");

    // (d) small pools stay exact in BOTH forms (no regression at the low end).
    let small_block = 3i32;
    assert_eq!(
        address_i32_wrapping(small_block, bb, 7),
        address_i64(small_block as i64, block_bytes, 7) as i32
    );
}
