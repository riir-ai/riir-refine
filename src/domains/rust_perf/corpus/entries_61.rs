//! Entries slice - Batch 329 (2026-11-04): one `rust_perf` rule from the
//!
//! Source: tembo/voxl @ `eba3b7ce` (MIT code), re-verified byte-exact at the
//! pin this batch; the verbatim law quotes live in each rule's `context_notes`
//! (the doc-lazy-continuation class: quoted prose in `//!` breaks the
//! paragraph parse). Fixtures are ORIGINAL plain-Rust stand-ins, never voxl
//! source. Batch 329 / Plan 337 / Research 367.

use crate::domains::rust_perf::PerfRule;
#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── dispatch: an event-sourced change map drives every stage —
        // whole-partition selection, work budgets, and skip-hint
        // forwarding into the downstream consumer's own analyzer ──────
        // Batch 329 / tembo/voxl @ eba3b7ce (video.rs::encode_loop,
        // stripe_layout doc, encoder.rs mb_info).
        PerfRule {
            rule_id: "event-damage-map-selective-partition-skip-with-consumer-hints"
                .to_string(),
            name: "a change-driven pipeline processes every partition every frame -> an event-sourced per-cell change map (rectangles drained during the event pump, no second pass) drives EVERY stage: whole-partition selection (a partition whose band misses the damage skips; the no-intersection frame early-outs), work budgets from per-partition changed-cell density, and skip-hint FORWARDING into the heavy downstream consumer's own analyzer (one byte per cell, a constant marks 'unchanged', so the consumer's internal analysis elides static areas instead of re-deciding them)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"// NAIVE: the frame pipeline re-decides and re-encodes EVERY cell of
// EVERY partition on EVERY frame, and the downstream analyzer re-runs
// its own per-cell classification over static areas it already saw.
// Idle frames pay the full frame cost forever (the source anchor: ~9 ms
// per frame even when nothing changed).

pub const CELL: usize = 16;

#[derive(Clone, Copy)]
pub struct Cell {
    pub luma: u8,
    pub mode: u8,
    pub bits: u32,
}

pub struct Frame<'a> {
    pub cells: &'a [Cell],
    pub width: usize,
    pub height: usize,
}

pub fn analyze(cell: &Cell) -> u32 {
    // the analyzer's per-cell decision work (mode search + rate estimate)
    let mut cost = u32::from(cell.luma) * 7;
    cost = cost.wrapping_mul(0x9E37_79B9) ^ u32::from(cell.mode);
    cost ^ (cost >> 13)
}

pub fn encode(cells: &[Cell], width: usize, height: usize) -> u64 {
    let mut out = 0u64;
    let stride = width.div_ceil(CELL);
    for row in 0..height.div_ceil(CELL) {
        for col in 0..stride {
            for y in 0..CELL {
                for x in 0..CELL {
                    let py = row * CELL + y;
                    let px = col * CELL + x;
                    if py < height && px < width {
                        let c = &cells[py * width + px];
                        out = out.wrapping_add(u64::from(analyze(c)));
                    }
                }
            }
        }
    }
    out
}
"#
            .to_string(),
            fixed_code: r#"// MAP-DRIVEN: an event-sourced change map (rectangles drained during the
// event pump) marks per-cell change state BETWEEN frames. The map then
// drives every stage: (1) whole-partition selection - a partition whose
// band misses every damage rectangle skips, and a frame with no
// intersection early-outs entirely; (2) work budget - the per-partition
// changed-cell count sizes the work handed downstream; (3) hint
// FORWARDING - the map is handed to the consumer as one-byte-per-cell
// skip hints (a constant marks 'unchanged'), so the consumer's own
// analyzer elides static areas without re-deciding them. Dirty cells
// produce exactly what they produced before - the map only ever elides.

pub const CELL: usize = 16;
pub const UNCHANGED: u8 = 1;

#[derive(Clone, Copy)]
pub struct Cell {
    pub luma: u8,
    pub mode: u8,
    pub bits: u32,
}

pub struct Frame<'a> {
    pub cells: &'a [Cell],
    pub width: usize,
    pub height: usize,
}

/// Rectangles drained from the event pump between frames (row-major cell
/// coordinates). Draining happens where the events already arrive - the
/// pump - so building the map is folded into work the pipeline does anyway.
pub struct ChangeMap {
    pub marks: Vec<u8>,
    pub cols: usize,
    pub rows: usize,
}

impl ChangeMap {
    /// Drain damage rectangles into the map (marks persist until taken).
    pub fn mark_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        let (cx0, cx1) = (x0 / CELL, x1.div_ceil(CELL));
        let (ry0, ry1) = (y0 / CELL, y1.div_ceil(CELL));
        for r in ry0..ry1.min(self.rows) {
            for c in cx0..cx1.min(self.cols) {
                self.marks[r * self.cols + c] = 0; // 0 = changed
            }
        }
    }

    /// Take the map for this frame, leaving it cleared for the next.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::replace(&mut self.marks, vec![UNCHANGED; self.marks.len()])
    }
}

pub fn analyze(cell: &Cell) -> u32 {
    let mut cost = u32::from(cell.luma) * 7;
    cost = cost.wrapping_mul(0x9E37_79B9) ^ u32::from(cell.mode);
    cost ^ (cost >> 13)
}

pub fn encode(cells: &[Cell], width: usize, height: usize, hints: &[u8]) -> u64 {
    let mut out = 0u64;
    let stride = width.div_ceil(CELL);
    for row in 0..height.div_ceil(CELL) {
        for col in 0..stride {
            // (1) whole-partition selection: the partition's hint slice is
            // entirely UNCHANGED -> skip the partition whole.
            let all_unchanged = (0..CELL)
                .all(|y| {
                    (0..CELL).all(|x| {
                        let (py, px) = (row * CELL + y, col * CELL + x);
                        py >= height
                            || px >= width
                            || hints[py / CELL * stride + px / CELL] == UNCHANGED
                    })
                });
            if all_unchanged {
                continue;
            }
            for y in 0..CELL {
                for x in 0..CELL {
                    let (py, px) = (row * CELL + y, col * CELL + x);
                    if py < height && px < width {
                        let idx = py / CELL * stride + px / CELL;
                        // (3) hint forwarding: the analyzer elides cells the
                        // map marks unchanged; dirty cells unchanged.
                        if hints[idx] != UNCHANGED {
                            let c = &cells[py * width + px];
                            out = out.wrapping_add(u64::from(analyze(c)));
                        }
                    }
                }
            }
        }
    }
    out
}
"#
            .to_string(),
            context_notes: Some(
                "THE LAW (one mechanism, three firings - the whole pipeline is the rule): an \
event-sourced change map (raw rectangles drained during the event pump - no second \
pass, no server round trip) (1) SELECTS whole partitions (a partition whose row band \
misses the damage band gets no job; a frame with zero intersections early-outs \
entirely), (2) BUDGETS work (the per-partition changed-cell count sizes the hand-off \
and feeds the governor), (3) FORWARDS SKIP HINTS into the black-box heavy consumer \
(one byte per cell, a constant marks 'unchanged') so the consumer's own internal \
analysis elides static areas instead of re-deciding them - the same map band-limits \
the color conversion (only dirty rows converted): the map drives EVERY stage, not \
just one. SOURCE (tembo/voxl @ eba3b7ce, MIT code; every quote re-verified byte-exact \
at the pin this batch): video.rs::stripe_layout doc, verbatim: 'x264 pays ~9 ms of CPU \
per 2560x1600 frame even when nothing changed, and sliced threading triples that; \
independent single-threaded stripes are encoded in parallel (low latency) and only \
when they changed (low CPU).'; the selection gate, verbatim structure: 'selected' = \
'job.band.is_some_and(|b| (b.y0 as usize) < y + h && (b.y1 as usize) > y)' with the \
whole-frame early-out 'if !selected.iter().any(|&x| x) { let _ = ret.send(..); \
continue; }'; encoder.rs, verbatim: 'Per-macroblock \"unchanged\" hints from X damage \
let x264 skip analysis of static areas.' + encode() doc: '`mb_info` (one byte per \
macroblock, X264_MBINFO_CONSTANT for unchanged ones) lets x264 skip analysis of \
static areas'; the capture side drains X11 DAMAGE rectangles into the per-macroblock \
map during the event pump ('Rows touched since the last take (raw damage rectangles, \
no server round trip)'), and the same map band-limits the yuv conversion \
(cap.convert(&mut buf, t.y0, t.y1, ..) - only dirty rows). MAGNITUDE ANCHOR \
(provenance, never bars): ~1/dirty-fraction - their own figures are ~9 ms/frame \
idle-frame cost and the README's 1.39-cores-busy desktop (the pipeline does ~1/8 of \
the work at a 12% dirty share). REGIME: change-driven pipelines with a persistent \
change map and heavy per-cell downstream work - editors, image/video encoders, build \
caches, incremental compilers, diff engines, live-preview renderers; the win needs a \
consumer that CAN accept hints (or partitions it never needs to see). ANTI-REGIMES: \
work that must touch every cell for correctness (a lossy estimator needing global \
context; a semantic that changes definition between frames - stale hints then lie); \
consumers with no hint channel (then the map still drives selection + budgets - two \
of three firings stand); one-shot pipelines with no cross-frame state to persist. \
DISTINCT-FROM (grep-verified live at authoring over rust_perf + the kernel_opt kin \
set): skip-unchanged-writes-via-dirty-encoding (kernel_opt gpu_07) elides PER-ELEMENT \
WRITES via a dirty bit piggybacked on the element's own encoding - in-kernel store \
elision; THIS owns whole-partition selection + hint FORWARDING to an EXTERNAL \
consumer's analyzer (the corpus could not previously tell ANY downstream stage what \
to skip). predicate-gated-skip-empty-workset-step (gpu_10) owns a zero-count \
whole-step guard (no persistent map, no consumer hints). \
cost-model-counts-work-the-kernel-skips (gpu_133) owns cost-model FIDELITY for \
autotune ceilings (it counts what a kernel skips; THIS elides the work itself). \
prefilter-skip-density-demotion (entries_30) latches a probe inert by skip density \
(no map, no forwarding). mask-block-classification-skip-placement / \
swa-evicted-prefix-tile-analytic-skip (kernel_opt) own GPU tile placement. FIXTURE \
PROVENANCE: original machinery (never voxl source) - the pipeline re-authored as \
plain-Rust stand-ins; the bench harness models per-cell work at the verdict's \
pre-registered 10-15% dirty band (deterministic LCG map), both arms checksum-identical \
on dirty cells. MEASURED ParetoDominant 3/0/0 (M3 Max / Mac15,8, release, \
rust_perf_bench validator, 2026-11-04, the Batch-329 GOAT bench, quiet box disclosed): \
8.18x/6.96x/6.99x at 256/4096/65536 cells, allocs 0/0 both arms - the ratio tracks \
the theoretical 1/dirty-fraction ceiling (8.3x at the 12% band) almost exactly. \
Wave: Batch 329 / Plan 337 / Research 367. Cite: tembo/voxl @ \
eba3b7ced9f60e885bb7b3a96a12e3ccb347fdcd - src/video.rs (encode_loop selected gate + \
stripe_layout doc), src/encoder.rs (b_mb_info setup + encode() mb_info doc), \
src/capture.rs (mb_dirty map + take_mb_map), Batch 329 / Plan 337.".to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(event_damage_map_partition_hints_bench_spec()),
        },
    ]
}

/// The GOAT bench (Batch 329): per-cell work at the verdict's pre-registered
/// 10-15% dirty band. Buggy arm = full-frame re-analysis every frame; fixed
/// arm = map-driven partition selection + hint forwarding. Both arms checksum
/// the analyzed cells so the elision cannot change what dirty cells produce.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)]
fn event_damage_map_partition_hints_bench_spec() -> BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![256, 4_096, 65_536],
        harness: r#"
const CELL: usize = 16;
const UNCHANGED: u8 = 1;

fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state
}

#[derive(Clone, Copy)]
pub struct Cell { pub luma: u8, pub mode: u8, pub bits: u32 }

pub struct Input {
    cells: Vec<Cell>,
    /// The cell grid is cells_side x cells_side (row-major).
    cells_side: usize,
    hints: Vec<u8>,
    /// Per-partition changed-cell count (the map's density budget — the
    /// governor's x-input); the O(1) whole-partition skip reads this.
    changed: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    // Grid-shaped input; n = CELL count (one Cell = one 16x16 macroblock).
    // Deterministic LCG content; the work runs per CELL, not per pixel.
    let side = (n as f64).sqrt().max(4.0) as usize;
    let cols = side;
    let rows = side;
    let cells: Vec<Cell> = (0..cols * rows)
        .map(|i| {
            let s = (i as u64) ^ 0xDEAD_BEEF;
            Cell { luma: (s % 256) as u8, mode: ((s >> 8) % 4) as u8, bits: (s >> 16) as u32 }
        })
        .collect();
    // A 10-15% dirty band: ~12% of cells marked changed, in row-band
    // rectangles (like real damage events), deterministic.
    let mut hints = vec![UNCHANGED; cols * rows];
    let target = (cols * rows) * 12 / 100;
    let mut state = 0x5EED_1234u64;
    let mut marked = 0usize;
    let mut r = 0usize;
    while marked < target && r < rows {
        let c0 = (lcg(&mut state) as usize) % cols;
        let c1 = ((c0 + (lcg(&mut state) as usize) % (cols / 4 + 1)).min(cols)).max(c0 + 1);
        for c in c0..c1 {
            hints[r * cols + c] = 0;
            marked += 1;
            if marked >= target { break; }
        }
        r = (r + 1 + (lcg(&mut state) as usize) % 2) % rows;
    }
    // The density budget: changed-cell count per 16x16-cell PARTITION —
    // the O(1) whole-partition skip reads this (and the governor's x-input
    // is the same count). Partition grid is pcols x prows.
    let pcols = cols.div_ceil(16);
    let mut changed = vec![0u32; pcols * cols.div_ceil(16).max(1)];
    for (i, &h) in hints.iter().enumerate() {
        if h == 0 {
            let (r, c) = (i / cols, i % cols);
            changed[(r / 16) * pcols + (c / 16)] += 1;
        }
    }
    Input { cells, cells_side: side, hints, changed }
}

/// The heavy per-cell decision work (a mode-search-shaped inner loop:
/// the source's real per-MB analyze is ~us, not ~ns - the fixture must
/// carry that weight or the elision overhead outweighs the skip).
fn analyze(c: &Cell) -> u32 {
    let mut cost = u32::from(c.luma).wrapping_mul(0x9E37_79B9) ^ u32::from(c.mode);
    let mut s = u64::from(c.bits) | 1;
    for _ in 0..32 {
        s = s.wrapping_mul(0x9E37_79B9).wrapping_add(u64::from(cost));
        cost ^= (s >> 33) as u32;
    }
    cost
}

/// Buggy: full-frame re-analysis every frame - every partition, every cell.
/// The checksum rides black_box so the work is never dead.
pub fn run_buggy(input: &Input) {
    let mut out = 0u64;
    for c in &input.cells {
        out = out.wrapping_add(u64::from(analyze(c)));
    }
    std::hint::black_box(out);
}

/// Fixed: map-driven whole-partition selection (O(1) via the map's
/// per-partition changed-count) + per-cell hint forwarding (unchanged
/// cells elide the analyzer). Dirty cells produce exactly the buggy
/// arm's values.
pub fn run_fixed(input: &Input) {
    let mut out = 0u64;
    let side = input.cells_side;
    let hints = &input.hints;
    // partitions of 16x16 cells (the stripe geometry at cell granularity)
    let pcols = side.div_ceil(16);
    let prows = side.div_ceil(16);
    for prow in 0..prows {
        for pcol in 0..pcols {
            // (1) whole-partition selection: O(1) via the partition's
            // changed-cell count (the map's density budget)
            if input.changed[prow * pcols + pcol] == 0 {
                continue;
            }
            for y in 0..16 {
                let row = prow * 16 + y;
                if row >= side {
                    break;
                }
                for x in 0..16 {
                    let col = pcol * 16 + x;
                    if col >= side {
                        break;
                    }
                    let idx = row * side + col;
                    // (3) hint forwarding: unchanged cells elide
                    if hints[idx] == 0 {
                        let c = &input.cells[idx];
                        out = out.wrapping_add(u64::from(analyze(c)));
                    }
                }
            }
        }
    }
    std::hint::black_box(out);
}

"#
        .to_string(),
    }
}
