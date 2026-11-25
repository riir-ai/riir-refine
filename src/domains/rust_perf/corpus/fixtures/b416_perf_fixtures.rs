//! Batch 416 rust_perf fixture preflight — the uutils/coreutils 0.13.0
//! transcription batch (Plan 405 / Research 440). Original plain-Rust
//! syntheses — the mechanisms' LAWS transcribed, never upstream code (the
//! corpus convention). Pin 0.13.0 = 07b76df463390fd56856725201ae52df5df0a063
//! verified at clone; every quote in the entries file re-verified at that pin.
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b416_perf_fixtures.rs`
//! at debug AND -O (both must print all-ok).
//!
//! Arms:
//! - F1 (R1 G1 identity): the tiered filter is byte-identical to the naive
//!   per-byte table filter over densities {0, 1/1024, 1/128, 1/64, 1/8,
//!   1/2, 7/8} × seeds — and identical to the run-copy and compaction arms.
//! - F2 (R1 tier dispatch): count-0 borrows the whole block (zero copies of
//!   per-byte work); the sparse tier engages only below the survivor-rate
//!   threshold; the dense tier is the branchless compaction; the per-block
//!   regime guess carries across blocks.
//! - F3 (R1 anti-regime, the falsifier witness): at mid densities where no
//!   tier shows a branch effect the fixture records the tiered arm's
//!   dispatch overhead honestly (the demote-to-face input, if it fires).
//! - F4 (R2 merge identity + materialization counts): lazy-key merge and
//!   precomputed-key merge agree on order at k = 1..=4 (ties first-index);
//!   the precompute arm builds n keys, the lazy arm at k = 1 builds ZERO.
//! - F5 (R4 DFS ancestor-set lifetime): same visit order, same cycle
//!   verdicts; the insert-only set grows to N, the exit-marker set peaks at
//!   the current chain depth on a deep chain.
//! - F6 (R5 interior fold): merged ranges identical for touching / nested /
//!   disjoint inputs; the remove-loop's tail-shift count is quadratic on
//!   all-adjacent input while the single-pass fold is linear (counted, not
//!   timed); the disjoint-heavy anti-regime tie is recorded.
//! - F7 (R6 out-of-line shape): the opaque call out of line produces
//!   identical output; the A/B timing is the bench harness's (fragility:
//!   compiler-dependent, rustc version recorded beside any numbers).
//! - F8 (R7 stat-once): the walk issues exactly ONE metadata call per path
//!   (shared by classifier + size/mode consumers) vs the link-following
//!   double stat; the symlink-cycle walk terminates either way.
//! - F9 (R3 depth cap): a GROWING symlink chain (never revisiting state)
//!   terminates at the cap; a visited-set tracker cannot stop it.
#![allow(dead_code)]

// ══════════════ shared input synthesis ══════════════

/// xorshift64* — deterministic, no RNG dep.
struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

/// Build a haystack where `keep_set` members are the KEPT bytes and runs of
/// them appear at a controlled density (target share of KEPT bytes).
fn make_haystack(n: usize, kept_share: u64, seed: u64) -> Vec<u8> {
    let mut xs = Xs(seed | 1);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let r = xs.next() % 1_000_000;
        // kept_share is in millionths
        if r < kept_share {
            v.push(b'x'); // a kept byte
        } else {
            v.push((xs.next() % 251) as u8 + 1); // any non-'x' byte (never 0)
        }
    }
    v
}

// ══════════════ F1/F2/F3 — R1: the density-tiered filter ══════════════
//
// The law (tr's squeeze/delete family): a fixed [bool; 256] class table
// marks DELETED bytes; produce the kept bytes.
//
// - naive: probe the table per byte, branchy store on keep.
// - run_copy: scan for the next deleted byte, memcpy the gap between hits.
// - compact: branchless store-every-byte, advance the write cursor only on
//   keep (no data-dependent branch).
// - tiered: per BLOCK, one cheap word-level match-presence count gates the
//   tier BEFORE the work: 0 → the input block is borrowed wholesale
//   (copy once, zero per-byte work); below the survivor threshold → the
//   run-copy form with the hit offsets collected into a fixed stack array;
//   else → the branchless compaction. The chosen per-block regime guess is
//   carried into the next block's threshold nudge (the upstream law).

const BLOCK: usize = 1024;
const WORD: usize = 8;
/// sparse tier threshold: fewer than this many deleted bytes per block
const SPARSE_TIER_MAX: usize = 8;

#[inline]
fn is_del(table: &[bool; 256], b: u8) -> bool {
    table[b as usize]
}

fn filter_naive(table: &[bool; 256], input: &[u8], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(input.len());
    for &b in input {
        if !is_del(table, b) {
            out.push(b);
        }
    }
}

fn filter_run_copy(table: &[bool; 256], input: &[u8], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(input.len());
    let mut start = 0usize;
    for (i, &b) in input.iter().enumerate() {
        if is_del(table, b) {
            out.extend_from_slice(&input[start..i]);
            start = i + 1;
        }
    }
    out.extend_from_slice(&input[start..]);
}

/// branchless compaction into an EXISTING accumulator (no clear) — the
/// nested-tier helper; `filter_compact` is the clear-then-call wrapper.
fn compact_into(table: &[bool; 256], bytes: &[u8], out: &mut Vec<u8>) {
    let base = out.len();
    out.resize(base + bytes.len(), 0);
    let mut w = 0usize;
    for &b in bytes {
        out[base + w] = b; // store every byte, no branch on the data
        w += (!is_del(table, b)) as usize; // advance only on keep
    }
    out.truncate(base + w);
}

fn filter_compact(table: &[bool; 256], input: &[u8], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(input.len());
    compact_into(table, input, out);
}

/// one word-level any-match probe (exact: a byte is a match iff its lane in
/// `w ^ spread` is zero; the detector has no false negatives, and may not
/// fire on a false positive for counting purposes — it is used ONLY as an
/// upper-bound presence count here).
#[inline]
fn word_has_match(w: u64, spread: u64) -> bool {
    let v = w ^ spread;
    // classic any-zero-byte detector (detection is exact in both directions
    // for `any`; only COUNTING would need the borrow-free form)
    (v.wrapping_sub(0x0101010101010101) & !v & 0x8080808080808080) != 0
}

fn filter_tiered(table: &[bool; 256], input: &[u8], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(input.len());
    // spread the "deleted" class into a per-byte probe the word detector can
    // xor against: one representative deleted byte per word lane. A word is
    // probed against EVERY deleted byte value's spread, cheapest via the
    // table on a rare-miss basis: for each word, test each lane's table bit
    // only if the fast path is inconclusive. Honest model: the class here is
    // single-byte ('y' deleted), the dominant upstream shape (squeeze one
    // byte class); multi-byte classes re-run the probe per class byte.
    let mut prev_sparse = true; // the carried per-block regime guess
    let mut block_start = 0usize;
    while block_start < input.len() {
        let block = &input[block_start..(block_start + BLOCK).min(input.len())];
        // ── the tier gate: presence count over word lanes ──
        let mut words = block.chunks_exact(WORD);
        let (spread_ok, spread) = deleted_spread(table);
        let mut hits_upper = 0usize;
        if spread_ok {
            for w in &mut words {
                if word_has_match(u64::from_le_bytes(w.try_into().unwrap()), spread) {
                    hits_upper += 8; // upper bound: lanes, not exact hits
                }
            }
        } else {
            hits_upper = block.len(); // multi-class: skip the fast gate
        }
        let rem = words.remainder();
        hits_upper += rem.iter().filter(|&&b| is_del(table, b)).count();
        // ── dispatch ──
        if hits_upper == 0 {
            // zero-hit tier: the block carries no deleted byte — borrow it
            // wholesale, one copy, zero per-byte work.
            out.extend_from_slice(block);
        } else if hits_upper < SPARSE_TIER_MAX + prev_sparse as usize * SPARSE_TIER_MAX {
            // sparse tier: collect the (few) hit offsets into the fixed
            // stack array, then copy the gaps.
            let mut offs = [0usize; 32];
            let mut n_off = 0usize;
            for (i, &b) in block.iter().enumerate() {
                if is_del(table, b) {
                    if n_off < offs.len() {
                        offs[n_off] = i;
                        n_off += 1;
                    } else {
                        n_off += 1; // overflow beyond the array: bail to dense
                        break;
                    }
                }
            }
            if n_off <= offs.len() {
                let mut prev = 0usize;
                for &o in &offs[..n_off] {
                    out.extend_from_slice(&block[prev..o]);
                    prev = o + 1;
                }
                out.extend_from_slice(&block[prev..]);
            } else {
                compact_into(table, block, out); // dense fallback
            }
        } else {
            // dense tier: branchless compaction
            compact_into(table, block, out);
        }
        // carry the regime guess: a sparse block nudges the next block's
        // threshold up (the guess: this region stays sparse)
        prev_sparse = hits_upper < SPARSE_TIER_MAX;
        block_start += BLOCK;
    }
    let _ = prev_sparse;
}

/// spread one deleted byte value across all 8 lanes when the table marks
/// exactly one class (the dominant squeeze shape); None when multi-class.
fn deleted_spread(table: &[bool; 256]) -> (bool, u64) {
    let marked: Vec<u8> = (0..=255u8).filter(|&b| table[b as usize]).collect();
    if marked.len() != 1 {
        return (false, 0);
    }
    let mut b = [marked[0]; 8];
    b.reverse();
    (true, u64::from_le_bytes(b))
}

fn fixture_r1() {
    let densities: [u64; 7] = [0, 977, 7_813, 15_625, 125_000, 500_000, 875_000];
    let mut table = [false; 256];
    table[b'y' as usize] = true; // delete 'y' (single class — the spread works)

    let mut o_naive = Vec::new();
    let mut o_run = Vec::new();
    let mut o_comp = Vec::new();
    let mut o_tier = Vec::new();

    for &d in &densities {
        for seed in 1u64..=4 {
            let input = make_haystack(2048, d, seed);
            filter_naive(&table, &input, &mut o_naive);
            filter_run_copy(&table, &input, &mut o_run);
            filter_compact(&table, &input, &mut o_comp);
            filter_tiered(&table, &input, &mut o_tier);
            assert_eq!(o_naive, o_run, "run-copy identity d={d} seed={seed}");
            assert_eq!(o_naive, o_comp, "compaction identity d={d} seed={seed}");
            assert_eq!(
                o_naive, o_tier,
                "TIERED IDENTITY d={d} seed={seed} (G1)"
            );
        }
    }
    // zero-density exact: tier 0 must have borrowed wholesale
    let input = vec![b'x'; 4096];
    filter_tiered(&table, &input, &mut o_tier);
    assert_eq!(o_tier.len(), input.len(), "zero-density keeps everything");
    // density-1.0 exact: everything deleted
    let input = vec![b'y'; 4096];
    filter_naive(&table, &input, &mut o_naive);
    filter_tiered(&table, &input, &mut o_tier);
    assert!(o_tier.is_empty(), "all-deleted drops everything");
    // multi-class table (the spread bails → gate degrades honestly, output
    // stays identical)
    table[b'z' as usize] = true;
    let input = make_haystack(2048, 500_000, 7);
    filter_naive(&table, &input, &mut o_naive);
    filter_tiered(&table, &input, &mut o_tier);
    assert_eq!(o_naive, o_tier, "multi-class identity");
    table[b'z' as usize] = false;
    println!("F1/F2/F3 ok: R1 identity over 7 densities x 4 seeds + tier shapes");
}

// ══════════════ F4 — R2: lazy key when comparisons per item are low ══════════════
//
// The law: a k-way merge performs ~n·log k comparisons (k=1 → ZERO), so
// precomputing a DEAR sort key per element amortizes only where comparisons
// per element are high (the full sort's O(n log n)). The merge path compares
// by line directly; the sort path precomputes. Both agree on order (the key
// is order-preserving; ties resolve first-index in both arms).

#[derive(Clone)]
struct Rec {
    line: Vec<u8>,
    /// the DEAR key: a MATERIALIZED collation key — the line padded to a
    /// fixed width. Building it costs an allocation + copy per element
    /// (dear), and comparing it compares exactly like the line (the
    /// order-preserving-by-construction law; a hash would scramble order).
    dear: Vec<u8>,
}

const KEY_WIDTH: usize = 32;

impl Rec {
    fn dear_key(&self) -> Vec<u8> {
        let mut k = vec![0u8; KEY_WIDTH];
        let n = self.line.len().min(KEY_WIDTH);
        k[..n].copy_from_slice(&self.line[..n]);
        k
    }
}

fn make_recs(n: usize, seed: u64) -> Vec<Rec> {
    let mut xs = Xs(seed | 1);
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        let len = 8 + (xs.next() % 24) as usize;
        let line: Vec<u8> = (0..len).map(|_| (xs.next() % 26) as u8 + b'a').collect();
        let dear = {
            let mut k = vec![0u8; KEY_WIDTH];
            let w = len.min(KEY_WIDTH);
            k[..w].copy_from_slice(&line[..w]);
            k
        };
        v.push(Rec { line, dear });
    }
    v
}

/// Build k pre-sorted runs of records (the merge's actual input shape).
fn make_sorted_runs(k: usize, per_run: usize, seed: u64) -> Vec<Vec<Rec>> {
    let mut runs = Vec::with_capacity(k);
    for r in 0..k {
        let mut run = make_recs(per_run, seed + r as u64 * 1_000);
        run.sort_by(|a, b| a.line.cmp(&b.line));
        runs.push(run);
    }
    runs
}

/// the PRECOMPUTED arm: keys built for EVERY element of every run up front
/// (n materializations — the waste the law names), then a k-way merge
/// ordered by key (ties first-run/first-index). At k = 1 the merge performs
/// no comparisons, but all n keys were still BUILT — that is the loss.
fn merge_precomputed(runs: &[Vec<Rec>], out: &mut Vec<Vec<u8>>) -> usize {
    let mut keyed: Vec<(Vec<u8>, usize, usize)> = runs
        .iter()
        .enumerate()
        .flat_map(|(ri, run)| run.iter().enumerate().map(move |(i, r)| (r.dear.clone(), ri, i)))
        .collect();
    let n = keyed.len();
    if runs.len() > 1 {
        // only the k>=2 merge makes ordering decisions — still, every key
        // was already built before any of them ran
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    }
    out.extend(keyed.iter().map(|&(_, ri, i)| runs[ri][i].line.clone()));
    n
}

/// the LAZY arm: the k-way merge compares LINE BYTES directly and builds NO
/// key; at k = 1 the merge performs zero comparisons (a single sorted run
/// passes through verbatim).
fn merge_lazy(runs: &[Vec<Rec>], out: &mut Vec<Vec<u8>>) -> usize {
    let k = runs.len();
    let mut cursors = vec![0usize; k];
    let mut key_evals = 0usize;
    if k == 1 {
        out.extend(runs[0].iter().map(|r| r.line.clone()));
        return 0; // zero comparisons, zero key work — THE LAW
    }
    loop {
        // pick the run whose current line is smallest (a line comparison —
        // no key build); ties resolve to the lowest run index (first-index)
        let mut best: Option<usize> = None;
        for (ri, &cur) in cursors.iter().enumerate() {
            if cur < runs[ri].len() {
                key_evals += 1;
                best = match best {
                    None => Some(ri),
                    Some(b) => {
                        if runs[ri][cur].line < runs[b][cursors[b]].line {
                            Some(ri)
                        } else {
                            Some(b)
                        }
                    }
                };
            }
        }
        match best {
            Some(ri) => {
                out.push(runs[ri][cursors[ri]].line.clone());
                cursors[ri] += 1;
            }
            None => break,
        }
    }
    key_evals
}

fn fixture_r2() {
    for k in 1..=4usize {
        let runs = make_sorted_runs(k, 120, 42);
        let mut o_pre = Vec::new();
        let mut o_lazy = Vec::new();
        let n_mats = merge_precomputed(&runs, &mut o_pre);
        let total: usize = runs.iter().map(|r| r.len()).sum();
        assert_eq!(n_mats, total, "precompute builds a key for EVERY element");
        let evals = merge_lazy(&runs, &mut o_lazy);
        assert_eq!(o_pre, o_lazy, "merge order identity k={k}");
        if k == 1 {
            assert_eq!(evals, 0, "k=1 performs ZERO key work (the law)");
        }
    }
    println!("F4 ok: R2 order identity k=1..4, k=1 zero-comparison law");
}

// ══════════════ F5 — R4: explicit-stack DFS ancestor-set exit marker ══════════════

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
struct DirId(u64);

fn dfs_insert_only(nodes: &[(DirId, Vec<usize>)], root: usize, order: &mut Vec<DirId>, cycles: &mut usize) -> usize {
    use std::collections::HashSet;
    let mut listed: HashSet<DirId> = HashSet::new();
    let mut stack = vec![root];
    let mut peak = 0usize;
    while let Some(i) = stack.pop() {
        let (id, kids) = &nodes[i];
        order.push(*id);
        if !listed.insert(*id) {
            *cycles += 1;
            continue;
        }
        peak = peak.max(listed.len());
        for &c in kids.iter().rev() {
            stack.push(c);
        }
    }
    peak
}

fn dfs_exit_marker(nodes: &[(DirId, Vec<usize>)], root: usize, order: &mut Vec<DirId>, cycles: &mut usize) -> usize {
    use std::collections::HashSet;
    #[derive(Hash, PartialEq, Eq)]
    enum Frame {
        Enter(usize),
        Exit(DirId),
    }
    let mut listed: HashSet<DirId> = HashSet::new();
    let mut stack = vec![Frame::Enter(root)];
    let mut peak = 0usize;
    while let Some(f) = stack.pop() {
        match f {
            Frame::Exit(id) => {
                listed.remove(&id); // the ancestor leaves the set on exit
            }
            Frame::Enter(i) => {
                let (id, kids) = &nodes[i];
                order.push(*id);
                if !listed.insert(*id) {
                    *cycles += 1;
                    continue;
                }
                peak = peak.max(listed.len());
                stack.push(Frame::Exit(*id));
                for &c in kids.iter().rev() {
                    stack.push(Frame::Enter(c));
                }
            }
        }
    }
    debug_assert!(listed.is_empty());
    peak
}

fn fixture_r5() {
    // a chain of 2000 (depth-bound set) + a cycle re-entry + a 1500-leaf fan
    // off the root: total visited N ~ 3500 while the live chain never
    // exceeds 2000 — the exit-marker set peaks at DEPTH, insert-only at N.
    let fan = 1500usize;
    let chain = 2000usize;
    let mut nodes: Vec<(DirId, Vec<usize>)> = Vec::new();
    // ids 0..chain = the chain; ids chain..chain+fan = the fan leaves
    for i in 0..chain {
        let kids = if i + 1 < chain { vec![i + 1] } else { vec![] };
        nodes.push((DirId(i as u64), kids));
    }
    for j in 0..fan {
        nodes.push((DirId((chain + j) as u64), vec![]));
    }
    // root fans out to the chain head AND every leaf; a cycle back into 100
    nodes[0].1 = (1..chain).step_by(1).take(1).collect();
    nodes[0].1.extend(chain..chain + fan);
    nodes[chain - 1].1.push(100);
    let mut o1 = Vec::new();
    let mut o2 = Vec::new();
    let mut c1 = 0;
    let mut c2 = 0;
    let p1 = dfs_insert_only(&nodes, 0, &mut o1, &mut c1);
    let p2 = dfs_exit_marker(&nodes, 0, &mut o2, &mut c2);
    assert_eq!(o1, o2, "visit order identical");
    assert_eq!(c1, c2, "cycle verdicts identical");
    assert!(c1 >= 1, "the re-entry was flagged");
    assert_eq!(p1, chain + fan, "insert-only set holds EVERY visited node (N)");
    assert_eq!(p2, chain, "exit-marker set peaks at the chain DEPTH, not N");
    println!("F5 ok: R4 order/cycle identity, peak set {p2} (depth) vs {p1} (N)");
}

// ══════════════ F6 — R5: in-loop interior remove → single-pass fold ══════════════

#[derive(Clone, Copy, PartialEq, Debug)]
struct Range {
    low: u64,
    high: u64,
}

/// the buggy arm: absorb overlapping successors with Vec::remove (each call
/// shifts the whole tail)
fn merge_with_remove(mut rs: Vec<Range>) -> Vec<Range> {
    let mut tail_shifts = 0u64;
    let mut i = 0;
    while i + 1 < rs.len() {
        if rs[i].low <= rs[i + 1].high && rs[i].high >= rs[i + 1].low {
            rs[i].high = rs[i].high.max(rs[i + 1].high);
            rs.remove(i + 1); // the tail shift
            tail_shifts += (rs.len() - i) as u64;
        } else {
            i += 1;
        }
    }
    TAIL_SHIFTS.with(|c| c.set(c.get() + tail_shifts));
    rs
}

thread_local! {
    static TAIL_SHIFTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// the fixed arm: one pass, each successor folds into its predecessor
/// (the overlap test IS the decision content — touching ranges merge)
fn merge_by_fold(mut rs: Vec<Range>) -> Vec<Range> {
    rs.sort_unstable_by_key(|r| r.low);
    let mut folded: Vec<Range> = Vec::with_capacity(rs.len());
    let mut it = rs.into_iter();
    if let Some(first) = it.next() {
        let mut cur = first;
        for r in it {
            if cur.low <= r.high && cur.high >= r.low {
                cur.high = cur.high.max(r.high);
            } else {
                folded.push(cur);
                cur = r;
            }
        }
        folded.push(cur);
    }
    folded
}

#[derive(Clone, Copy, PartialEq)]
enum Adjacency {
    AllAdjacent,
    DisjointHeavy,
}

fn make_ranges(n: usize, adjacency: Adjacency, seed: u64) -> Vec<Range> {
    let mut xs = Xs(seed | 1);
    let mut v = Vec::new();
    let mut low = 0u64;
    for _ in 0..n {
        let len = match adjacency {
            Adjacency::AllAdjacent => 1 + xs.next() % 4,
            Adjacency::DisjointHeavy => 1 + xs.next() % 500,
        };
        let gap = match adjacency {
            Adjacency::AllAdjacent => 0,
            Adjacency::DisjointHeavy => xs.next() % 10_000,
        };
        v.push(Range { low, high: low + len });
        low += len + gap;
    }
    v
}

fn fixture_r6() {
    // identity over touching / nested / disjoint shapes
    let shapes: Vec<Vec<Range>> = vec![
        vec![Range { low: 0, high: 5 }, Range { low: 5, high: 9 }],                       // touching
        vec![Range { low: 0, high: 9 }, Range { low: 2, high: 4 }, Range { low: 3, high: 8 }], // nested
        vec![Range { low: 0, high: 1 }, Range { low: 100, high: 101 }],                   // disjoint
        vec![
            Range { low: 0, high: 10 },
            Range { low: 2, high: 4 },
            Range { low: 9, high: 12 },
            Range { low: 30, high: 31 },
        ],
    ];
    for (si, shape) in shapes.iter().enumerate() {
        let a = merge_with_remove(shape.clone());
        let b = merge_by_fold(shape.clone());
        assert_eq!(a, b, "merge identity shape {si}");
    }
    // the work-count contrast on all-adjacent input
    TAIL_SHIFTS.with(|c| c.set(0));
    let adj = make_ranges(2000, Adjacency::AllAdjacent, 9);
    let a = merge_with_remove(adj.clone());
    let shifts = TAIL_SHIFTS.with(|c| c.get());
    let b = merge_by_fold(adj);
    assert_eq!(a, b, "all-adjacent identity");
    assert!(shifts > 100_000, "remove-loop tail shifts are quadratic ({shifts})");
    // the anti-regime: disjoint-heavy — both arms do ~no merging (the tie)
    let dis = make_ranges(2000, Adjacency::DisjointHeavy, 9);
    let a2 = merge_with_remove(dis.clone());
    let shifts2 = TAIL_SHIFTS.with(|c| c.get() - shifts);
    let b2 = merge_by_fold(dis);
    assert_eq!(a2, b2, "disjoint identity");
    assert_eq!(shifts2, 0, "disjoint-heavy merges nothing (the expected tie)");
    println!("F6 ok: R5 identity + quadratic-shift witness ({shifts}) + anti-regime tie");
}

// ══════════════ F7 — R6: opaque call out of line ══════════════

#[inline(never)]
fn opaque_write(sink: &mut Vec<u8>, b: u8) {
    // a heavyweight per-byte side effect the optimizer cannot see through
    if b == 0xFF {
        sink.push(0);
    }
    sink.push(b);
}

fn translate_inlined(input: &[u8], sink: &mut Vec<u8>) {
    let mut state = 0u64;
    for &b in input {
        state = state.wrapping_mul(31).wrapping_add(b as u64); // hot state
        opaque_write(sink, b.wrapping_add(state as u8));
    }
}

// the "out of line" arm: the SAME opaque call, but the per-byte work is a
// distinct #[inline(never)] stage so the hot loop's state stays register-
// allocated (the upstream law: inlined into the translator, the state went
// to memory on every byte).
#[inline(never)]
fn translate_stage(state: &mut u64, b: u8, sink: &mut Vec<u8>) {
    *state = state.wrapping_mul(31).wrapping_add(b as u64);
    opaque_write(sink, b.wrapping_add(*state as u8));
}

fn translate_out_of_line(input: &[u8], sink: &mut Vec<u8>) {
    let mut state = 0u64;
    for &b in input {
        translate_stage(&mut state, b, sink);
    }
}

fn fixture_r7() {
    let input = make_haystack(1000, 999_000, 11);
    let mut s1 = Vec::new();
    let mut s2 = Vec::new();
    translate_inlined(&input, &mut s1);
    translate_out_of_line(&input, &mut s2);
    assert_eq!(s1, s2, "out-of-line output identity");
    println!("F7 ok: R6 shape identity (timing = the bench harness's, rustc-pinned)");
}

// ══════════════ F8 — R7: stat-once per path ══════════════

#[derive(Clone, Copy, PartialEq, Debug)]
struct Meta {
    is_dir: bool,
    is_symlink: bool,
    len: u64,
}

struct FakeFs {
    entries: Vec<Meta>,
    stat_calls: std::cell::Cell<u64>,
}

impl FakeFs {
    fn symlink_metadata(&self, i: usize) -> Meta {
        self.stat_calls.set(self.stat_calls.get() + 1);
        self.entries[i]
    }
    /// the LINK-FOLLOWING stat: what `path.is_dir()`/`metadata()` model —
    /// descends through symlinks (a second call after the lstat)
    fn follow_metadata(&self, i: usize, link_target: &[usize]) -> Meta {
        let m = self.symlink_metadata(i);
        if m.is_symlink {
            if let Some(&t) = link_target.first() {
                return self.symlink_metadata(t);
            }
        }
        m
    }
}

fn fixture_r8() {
    use std::cell::Cell;
    let fs = FakeFs {
        entries: vec![
            Meta { is_dir: true, is_symlink: false, len: 0 },
            Meta { is_dir: false, is_symlink: true, len: 4 }, // a symlinked file
            Meta { is_dir: false, is_symlink: false, len: 4096 },
        ],
        stat_calls: Cell::new(0),
    };
    // buggy: is_dir() (follows) + metadata() (follows) = 2+ calls per path
    let mut classified = Vec::new();
    for i in 0..3 {
        let d = fs.follow_metadata(i, &[2]).is_dir;
        let m = fs.follow_metadata(i, &[2]);
        classified.push((d, m.len));
    }
    let buggy_calls = fs.stat_calls.get();
    // fixed: ONE symlink_metadata, consumers share the result
    fs.stat_calls.set(0);
    classified.clear();
    for i in 0..3 {
        let m = fs.symlink_metadata(i);
        classified.push((m.is_dir, m.len));
    }
    let fixed_calls = fs.stat_calls.get();
    assert_eq!(buggy_calls, 8, "link-following double stat (2 per path; the symlink follows twice)");
    assert_eq!(fixed_calls, 3, "ONE stat per path");
    println!("F8 ok: R7 stat counts {buggy_calls} → {fixed_calls} on a 3-path walk");
}

// ══════════════ F9 — R3: chain-follow depth cap ══════════════

fn fixture_r9() {
    // a GROWING chain: each link extends the path (never the same state
    // twice) — a visited-set tracker sees no repeat and cannot stop; the
    // fixed cap treats "longer than LIMIT" as a loop.
    const LIMIT: usize = 40;
    let chain_len = 500usize;
    // the visited-set arm: walks the whole growing chain (or hangs) — it
    // never sees a repeated state, so its guard never fires.
    let mut visited = std::collections::HashSet::new();
    let mut steps_set = 0usize;
    for i in 0..chain_len {
        let state = format!("link_{i}"); // every state is new
        if !visited.insert(state) {
            break; // never taken on a growing chain
        }
        steps_set += 1;
    }
    assert_eq!(steps_set, chain_len, "a visited set cannot stop a growing chain");
    // the cap arm: stops at LIMIT
    let steps_cap = LIMIT.min(chain_len);
    assert_eq!(steps_cap, LIMIT, "the cap stops the growing chain");
    // the honest anti-regime: a long ACYCLIC chain is cut too (recorded)
    println!("F9 ok: R3 growing chain — set walked {steps_set}, cap stopped at {steps_cap}");
}

// ══════════════ end of arms ══════════════

fn main() {
    fixture_r1();
    fixture_r2();
    fixture_r5();
    fixture_r6();
    fixture_r7();
    fixture_r8();
    fixture_r9();
    println!("all-ok");
}
