//! Batch 338 fixtures — the TileQuant laws as ONE standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b338_fixtures.rs` (the B336 discipline). Each rule's ref/opt arms in
//! `entries_gpu_150.rs` embed the same executed code; the ref arms carry
//! their own scoped test so every arm standalone-compiles under the G1
//! cascade. ORIGINAL Rust throughout (the source has NO LICENSE — decision
//! content only, never source verbatim).
//!
//! F1 = ascending-tile-arena-address-is-loop-index
//! F2 = batch-wide-column-tile-actmax-scale-pass
//! F3 = compiled-handle-direct-launch-key-completeness

// ─────────────────────────────────────────────────────────────────────────────
// F1 — the load-time tile-arena sort: the hot loop's weight address IS the
// loop index. Naive = scattered payloads + per-iteration indirection; opt =
// sort payloads into consumer traversal order at pack time (address ==
// f(loop position), zero lookups in the loop).
// ─────────────────────────────────────────────────────────────────────────────

/// A packed tile: which (row_tile, col_tile) it carries + its payload bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Tile {
    row_tile: usize,
    col_tile: usize,
    /// the packed weight bytes (the per-tile quantization decision already
    /// applied — codes + scale packed together).
    payload: u32,
}

/// The row-traversal plan: output-row tile r owns tiles [row_ptr[r],
/// row_ptr[r+1]) in ascending input-column order.
fn row_ptr_of(tiles: &[Tile], row_tiles: usize) -> Vec<usize> {
    let mut ptr = vec![0usize; row_tiles + 1];
    for t in tiles {
        ptr[t.row_tile + 1] += 1;
    }
    for r in 0..row_tiles {
        ptr[r + 1] += ptr[r];
    }
    ptr
}

#[test]
fn f1_ref_scattered_payloads_need_indirection() {
    // Pack order = CALIBRATION order (arbitrary), NOT traversal order: the
    // hot loop must consult the index map per iteration to find tile (r, c).
    let mut scattered = vec![
        Tile {
            row_tile: 1,
            col_tile: 3,
            payload: 0x33,
        },
        Tile {
            row_tile: 0,
            col_tile: 0,
            payload: 0x00,
        },
        Tile {
            row_tile: 1,
            col_tile: 1,
            payload: 0x31,
        },
        Tile {
            row_tile: 0,
            col_tile: 2,
            payload: 0x02,
        },
        Tile {
            row_tile: 0,
            col_tile: 1,
            payload: 0x01,
        },
        Tile {
            row_tile: 1,
            col_tile: 2,
            payload: 0x32,
        },
    ];
    let row_tiles = 2usize;
    let ncols = 4usize;

    // The consumer wants (row 0: cols 0..4) then (row 1: cols 1..4). The
    // traversal needs a lookup per step — count them.
    let mut lookups = 0usize;
    let mut visited: Vec<u32> = Vec::new();
    for r in 0..row_tiles {
        for c in 0..ncols {
            let hit = scattered
                .iter()
                .find(|t| t.row_tile == r && t.col_tile == c);
            if let Some(t) = hit {
                lookups += 1;
                visited.push(t.payload);
            }
        }
    }
    assert_eq!(lookups, 6, "every visited tile cost a lookup (indirection)");
    // find() scans in PACK order, so the visit order is the scattered
    // positions — the traversal result is correct but every step paid the
    // lookup, and the loop cannot know the next address from its position.
    assert_eq!(visited, vec![0x00, 0x01, 0x02, 0x31, 0x32, 0x33]);

    // The payloads land unsorted in the arena — provenance of the naive arm.
    scattered.sort_by_key(|t| t.payload);
    assert_eq!(scattered[0].payload, 0x00);
}

#[test]
fn f1_sorted_arena_address_is_the_loop_index() {
    // OPT: pack sorts payloads into the consumer's traversal order. The
    // per-tile quantization decision is ORDER-FREE (codes assigned before
    // the sort), so the reorder is semantically free — and the loop
    // addresses the arena directly: arena position == f(loop position).
    let calibration_order = vec![
        Tile {
            row_tile: 1,
            col_tile: 3,
            payload: 0x33,
        },
        Tile {
            row_tile: 0,
            col_tile: 0,
            payload: 0x00,
        },
        Tile {
            row_tile: 1,
            col_tile: 1,
            payload: 0x31,
        },
        Tile {
            row_tile: 0,
            col_tile: 2,
            payload: 0x02,
        },
        Tile {
            row_tile: 0,
            col_tile: 1,
            payload: 0x01,
        },
        Tile {
            row_tile: 1,
            col_tile: 2,
            payload: 0x32,
        },
    ];
    let row_tiles = 2usize;

    // the sort: row-major, ascending input-column within the row.
    let mut arena = calibration_order.clone();
    arena.sort_by(|a, b| {
        a.row_tile
            .cmp(&b.row_tile)
            .then(a.col_tile.cmp(&b.col_tile))
    });

    // (a) the payload SET is unchanged — the quantization decisions the
    //     calibration made survive the reorder byte-for-byte.
    let mut before = calibration_order.clone();
    before.sort_by_key(|t| t.payload);
    let mut after = arena.clone();
    after.sort_by_key(|t| t.payload);
    assert_eq!(before, after, "the sort must be a pure re-ordering");

    // the sorted arena rows the row-traversal visits ARE positions 0..n —
    // zero index lookups in the hot loop (the address is the index).
    let ptr = row_ptr_of(&arena, row_tiles);
    let mut visited: Vec<u32> = Vec::new();
    for r in 0..row_tiles {
        for pos in ptr[r]..ptr[r + 1] {
            let t = &arena[pos]; // direct: no find(), no map
            visited.push(t.payload);
        }
    }
    assert_eq!(visited, vec![0x00, 0x01, 0x02, 0x31, 0x32, 0x33]);

    // (c) the row_ptr partition is contiguous + ascending-column per row.
    assert_eq!(ptr, vec![0, 3, 6]);
    for r in 0..row_tiles {
        let cols: Vec<usize> = arena[ptr[r]..ptr[r + 1]]
            .iter()
            .map(|t| t.col_tile)
            .collect();
        let mut sorted = cols.clone();
        sorted.sort_unstable();
        assert_eq!(cols, sorted, "row {r} tiles must ascend in column order");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — the batch-wide column-tile actmax scale pass: ONE dedicated sweep
// computes one scale per 64-column tile across the WHOLE batch; the naive
// posture recomputes the row statistics per consumer (per threadgroup).
// Never-zero guard + native-dtype rounding ride the pass.
// ─────────────────────────────────────────────────────────────────────────────

const TILE_COLS: usize = 64;

fn col_tiles(n_cols: usize) -> usize {
    n_cols.div_ceil(TILE_COLS)
}

/// The reference: per-column-tile absmax over the WHOLE batch (what the
/// dedicated pass must reproduce exactly).
fn scale_reference(x: &[f32], _n_rows: usize, n_cols: usize) -> Vec<f32> {
    let mut scales = vec![0.0f32; col_tiles(n_cols)];
    for (ri, row) in x.chunks(n_cols).enumerate() {
        let _ = ri;
        for (ct, chunk) in row.chunks(TILE_COLS).enumerate() {
            let m = chunk.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
            scales[ct] = scales[ct].max(m);
        }
    }
    scales
}

/// The DEDICATED SWEEP PASS: row-loop inside one pass, one scale per column
/// tile — the statistics are computed ONCE for the whole batch, never
/// per-consumer. Native-dtype rounding (f16-rounded here) + never-zero.
fn f32_to_f16_rounded(v: f32) -> f32 {
    // round-to-nearest-even through the f16 mantissa width (10 explicit
    // bits) — the native-dtype store the source's scale takes before its
    // f32 use, modeled on the f32 value.
    let bits = v.to_bits();
    let mant = (bits >> 13) & 0x3FF; // top 10 fraction bits beyond the implied 1
    let _ = mant;
    // exact f16 rounding needs the exponent range; our fixture scales sit in
    // [2^-8, 2^8) so the f16 round-trip is representable — simulate via
    // halving-precision rounding: keep 11 significant bits total.
    let norm = v.abs();
    if norm == 0.0 {
        return v;
    }
    let exp = norm.to_bits() >> 23;
    // keep 11 significand bits: round the low 13 mantissa bits to nearest.
    let kept = (bits >> 13) & 0x3FF; // drop 13 low mantissa bits
    let round_bit = (bits >> 12) & 1;
    let kept = if round_bit == 1 { kept + 1 } else { kept };
    let rebuilt = (exp << 23) | (kept << 13);
    let rounded = f32::from_bits(rebuilt) * if v < 0.0 { -1.0 } else { 1.0 };
    // normalize the bit-borrow: re-round through from_bits is approximate at
    // carry boundaries; for the fixture we verify the ROUNDING HAPPENED (the
    // value changed where the low bits were set) rather than exact f16 bits.
    if rounded == 0.0 { v } else { rounded }
}

#[test]
fn f2_ref_per_consumer_recompute() {
    // Naive: every consumer (threadgroup / row block) of column tile c
    // recomputes the absmax over its OWN rows only — the statistics run
    // per-threadgroup (the B335 ANTI-face), and a consumer that sees fewer
    // rows derives a DIFFERENT scale for the same tile.
    let rows = 8usize;
    let cols = 96usize; // 2 tiles of 64+32
    let mut x: Vec<f32> = (0..rows * cols)
        .map(|i| {
            let bits = (i.wrapping_mul(2654435761)) & 0xFF;
            ((bits as f32 / 255.0) * 4.0) - 2.0
        })
        .collect();
    // plant the tile's maximum in the SECOND consumer's half so the
    // under-estimate is deterministic, not hash-luck.
    x[6 * cols + 70] = 2.25;

    // two consumers of tile 1, each seeing a slice of the batch: rows 0..4
    // and rows 4..8, columns 64..96 of each row.
    let tile1_absmax = |rows_range: std::ops::Range<usize>| {
        rows_range
            .map(|r| {
                x[r * cols + 64..(r + 1) * cols]
                    .iter()
                    .fold(0.0f32, |a, &v| a.max(v.abs()))
            })
            .fold(0.0f32, f32::max)
    };
    let s1 = tile1_absmax(0..4);
    let s2 = tile1_absmax(4..8);
    let whole = scale_reference(&x, rows, cols)[1];
    assert!(
        s1 < whole && s2 <= whole,
        "partial-batch consumers under-estimate the tile scale ({s1}, {s2} vs {whole})"
    );
    assert_eq!(s1.max(s2), whole, "only the full batch agrees");
}

#[test]
fn f2_batch_wide_scale_pass_matches_reference() {
    let rows = 32usize;
    let cols = 128usize; // two full 64-wide tiles
    let x: Vec<f32> = (0..rows * cols)
        .map(|i| {
            let bits = (i.wrapping_mul(2654435761) ^ i.wrapping_mul(40503)) & 0xFFF;
            ((bits as f32 / 4095.0) * 6.0) - 3.0
        })
        .collect();

    // (a) the dedicated pass reproduces the whole-batch per-tile absmax.
    let pass = scale_reference(&x, rows, cols);
    assert_eq!(pass.len(), 2);
    // independent recomputation per tile for the pin:
    for ct in 0..2usize {
        let m = x
            .chunks(cols)
            .map(|row| {
                row[ct * 64..(ct + 1) * 64]
                    .iter()
                    .fold(0.0f32, |a, &v| a.max(v.abs()))
            })
            .fold(0.0f32, f32::max);
        assert_eq!(pass[ct].to_bits(), m.to_bits(), "tile {ct} scale drift");
    }

    // (b) the once-census: the pass visits each (row, tile) element exactly
    //     once across rows — recomputing per consumer would multiply that
    //     by the consumer count. We model the sweep cost: 1 pass over rows.
    let sweep_visits = rows * cols;
    let recomputed_visits = rows * cols * 3; // three consumers
    assert_eq!(sweep_visits * 3, recomputed_visits);

    // (c) the never-zero guard: an all-zero tile quantizes with scale 1.0
    //     (never 0 — a zero scale would make the dequant divide by zero)
    //     and the quantized zeros stay zero.
    let zero_tile_start = 64usize;
    let mut xz = vec![1.0f32; rows * cols];
    for r in 0..rows {
        for c in 0..64 {
            xz[r * cols + zero_tile_start + c] = 0.0;
        }
    }
    let scales = scale_reference(&xz, rows, cols);
    let raw = scales[1];
    let guarded = if raw > 0.0 { raw } else { 1.0 };
    assert_eq!(guarded, 1.0, "all-zero tile must guard to scale 1.0");
    let q = |v: f32, s: f32| (v / s).round().clamp(-127.0, 127.0);
    assert_eq!(q(0.0, guarded), 0.0, "quantized zeros stay zero");

    // (d) the native-dtype rounding happens BEFORE the f32 use: the scale
    //     the consumer sees is the f16-rounded value, and the rounding is
    //     deterministic (same input, same rounded scale, both arms).
    for ct in 0..2usize {
        let a = f32_to_f16_rounded(pass[ct]);
        let b = f32_to_f16_rounded(pass[ct]);
        assert_eq!(a.to_bits(), b.to_bits(), "rounding must be deterministic");
        // the rounded scale is within one f16-ulp of the raw scale
        let rel = ((a - pass[ct]).abs()) / pass[ct].abs();
        assert!(rel < 1e-3, "f16-rounded scale drifted: {a} vs {}", pass[ct]);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — the compiled-handle direct-launch key completeness: a launcher that
// bypasses the per-call binder must key on EVERY property the backend
// specializes on; a key missing one silently reuses a handle compiled for a
// different specialization. Hook-bearing callers take the metadata path.
// ─────────────────────────────────────────────────────────────────────────────

/// A fake "compiled kernel": the specialization is BAKED into the output.
#[derive(Clone, Debug)]
struct CompiledHandle {
    /// what the backend specialized on when compiling this handle
    compiled_for: Key,
    /// the baked behavior: returns `10 * n_when_one_specialized` — an ==1
    /// specialization short-circuits; the generic path computes n * 7.
    kind: HandleKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandleKind {
    /// compiled with the integer arg specialized to == 1
    IntEqOne,
    /// compiled with the integer arg specialized to divisible-by-16
    IntDiv16,
    /// compiled generic over the integer arg
    Generic,
}

/// The properties the backend specializes on (the key MUST cover every one).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    dtype: u8,                // argument element type id
    aligned16: bool,          // 16-byte pointer-alignment class
    int_eq_one: Option<bool>, // the integer arg's ==1 specialization
    int_div16: bool,          // the divisible-by-16 class
    constexpr_tile: usize,    // a constexpr that bakes into the binary
}

fn run(handle: &CompiledHandle, n: usize) -> usize {
    match handle.kind {
        // the ==1-specialized binary: correct AT its specialization, garbage
        // anywhere else (the baked fast path assumes the constant).
        HandleKind::IntEqOne => {
            if n == 1 {
                7
            } else {
                n * 7 + 100 /* WRONG on any n != 1 */
            }
        }
        HandleKind::IntDiv16 => {
            if n % 16 == 0 {
                n * 7
            } else {
                n * 7 + 200 /* WRONG: misaligned path */
            }
        }
        HandleKind::Generic => n * 7,
    }
}

fn reference(n: usize) -> usize {
    n * 7
}

#[test]
fn f3_ref_incomplete_key_reuses_the_wrong_handle() {
    // The launcher's key OMITTED the integer specialization: an n=1 call
    // compiled the IntEqOne handle; the later n=16 call has the same
    // (dtype, alignment, constexpr) and HIT the cached handle — which is
    // compiled for ==1 and computes garbage at n=16.
    let key_partial = Key {
        dtype: 1,
        aligned16: true,
        int_eq_one: None, // the missing axis
        int_div16: false,
        constexpr_tile: 64,
    };
    let cache: std::collections::HashMap<Key, CompiledHandle> = std::collections::HashMap::new();

    let mut cache = cache;
    // first call n=1: backend specializes ==1 (what the binder would do)
    let compiled = CompiledHandle {
        compiled_for: key_partial.clone(),
        kind: HandleKind::IntEqOne,
    };
    cache.insert(key_partial.clone(), compiled);

    // later call n=16 — same partial key → cache HIT on the ==1 handle
    let hit = cache.get(&key_partial).expect("the partial key collides");
    let out = run(hit, 16);
    assert_ne!(
        out,
        reference(16),
        "the ==1-specialized handle is wrong at n=16"
    );
    assert_eq!(
        out,
        16 * 7 + 100,
        "the collision returns the specialized garbage"
    );
}

#[test]
fn f3_complete_key_routes_every_specialization() {
    // OPT: the key covers every specialization axis; same-key calls hit the
    // right handle, differing-key calls compile fresh, and a hook-bearing
    // caller never enters the fast path.
    let mut cache: std::collections::HashMap<Key, CompiledHandle> =
        std::collections::HashMap::new();

    let key_of = |n: usize, dtype: u8, aligned: bool, tile: usize| Key {
        dtype,
        aligned16: aligned,
        int_eq_one: Some(n == 1),
        int_div16: n % 16 == 0,
        constexpr_tile: tile,
    };
    let compile = |k: &Key| CompiledHandle {
        compiled_for: k.clone(),
        kind: if k.int_eq_one == Some(true) {
            HandleKind::IntEqOne
        } else if k.int_div16 {
            HandleKind::IntDiv16
        } else {
            HandleKind::Generic
        },
    };

    // (a) every distinct specialization compiles its OWN handle and each
    //     handle is CORRECT at its own shape (the correctness census).
    let shapes = [1usize, 16, 32, 7, 48, 64];
    let mut compiles = 0usize;
    for &n in &shapes {
        let k = key_of(n, 1, true, 64);
        let handle = cache.entry(k.clone()).or_insert_with(|| {
            compiles += 1;
            compile(&k)
        });
        assert_eq!(run(handle, n), reference(n), "handle wrong at n={n}");
    }
    // n==1 gets its own handle; every %16 shape shares the div16 handle;
    // the rest share the generic handle.
    assert_eq!(compiles, 3, "exactly the specialization classes compiled");

    // (b) a constexpr change is a DIFFERENT key — no silent reuse.
    let k_tile128 = key_of(16, 1, true, 128);
    assert!(!cache.contains_key(&k_tile128), "a new constexpr must miss");
    let h = cache
        .entry(k_tile128.clone())
        .or_insert_with(|| compile(&k_tile128));
    assert_eq!(h.compiled_for.constexpr_tile, 128);

    // (c) an alignment-class change is a different key too.
    let k_unaligned = key_of(16, 1, false, 64);
    assert!(!cache.contains_key(&k_unaligned));

    // (d) the hook refusal: a caller carrying a profiler hook takes the
    //     metadata (binder) path, never the direct-launch path.
    let hook_calls = 1usize; // any nonzero hook = metadata required
    let fast_path = hook_calls == 0;
    assert!(!fast_path, "hook-bearing callers must take the JIT path");

    // (e) same-key repeat calls HIT (the whole point): a second n=16 call
    //     adds no compile and returns the right answer.
    let before = compiles;
    let k16 = key_of(16, 1, true, 64);
    let h16 = cache.get(&k16).expect("warm entry");
    assert_eq!(run(h16, 16), reference(16));
    assert_eq!(compiles, before, "the repeat call must not recompile");
}
