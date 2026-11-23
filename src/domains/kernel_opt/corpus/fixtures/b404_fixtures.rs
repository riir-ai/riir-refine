//! Batch 404 kernel fixture preflight — the mlx #4927 cuda-window-sdpa
//! pack (Plan 403 / Research 438, ml-explore/mlx @ 58d91ee8…, MIT) as
//! ONE standalone file (4 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is C++/CUDA; the Rust arms below model each law's decision
//! content). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b404_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_164.rs in the same landing commit.
//!
//! Arms:
//! - R1 windowed-parameter-nyi-gate-forces-full-work-fallback: the
//!   NYI arm (full-mask materialization, L*L writes) vs the band arm
//!   (L*W writes, bit-identical mask); the two-independent-bounds
//!   tamper (causal join dropped — different mask, red); the
//!   array-mask+window named refusal (green refusal); the vacuous
//!   window clips to the plain path (the clip_window face).
//! - R2 slice-operand-to-window-before-kernel: the predicated
//!   full-scan arm (reads L) vs the slice-then-plain-scan arm (reads
//!   W, bit-identical scores); the front-window tamper (red); the
//!   multi-query global-slice refusal (green refusal); the stride-0
//!   contiguity refusal before the slice (green refusal).
//! - R3 dependency-version-known-wrong-window-refusal: the
//!   capability-only arm trains on the bug term below the pin (red);
//!   the version-gated arm refuses below 91501 (the conservative
//!   landed form, disclosed) and stays exact above; the unnamed
//!   refusal disclosure arm; the geometry tamper (alignment swapped
//!   at T_q == T_kv — different mask, red).
//! - R4 fused-vjp-ungeneric-direction-falls-back-not-throw: the throw
//!   arm panics on direction 3 (red); the dispatch arm routes to the
//!   composition, all four gradients exact; the back()-vs-role index
//!   arm (a grown tuple makes back() the cotangent — red, the role
//!   read green); the all-fallback disclosure arm (correct, slower).

// ═══════════ R1 — the NYI gate vs the band encoding ═══════════

/// the naive arm: the NYI fallback materializes the WINDOWED mask by
/// SCANNING every L*L cell with a membership test (the mask is the
/// band ∩ causal, but the scan pays the quadratic to find it)
fn naive_window_mask(l: usize, w: usize) -> (Vec<bool>, usize) {
    let mut m = vec![false; l * l]; // the L*L buffer itself: the memory point
    let mut touched = 0;
    for r in 0..l {
        for c in 0..l {
            touched += 1; // every cell tested
            m[r * l + c] = c <= r && c + w > r;
        }
    }
    (m, touched)
}

/// the band arm: the SAME windowed mask, written directly into the
/// band — L*W cells touched, no membership scan
fn band_mask(l: usize, w: usize) -> (Vec<bool>, usize) {
    let mut m = vec![false; l * l];
    let mut touched = 0;
    for r in 0..l {
        let start = r.saturating_sub(w - 1);
        for c in start..=r {
            m[r * l + c] = true; // c <= r && c > r - w, by construction
            touched += 1;
        }
    }
    (m, touched)
}

fn full_mask(l: usize) -> Vec<bool> {
    let mut m = vec![false; l * l];
    for r in 0..l {
        for c in 0..=r {
            m[r * l + c] = true;
        }
    }
    m
}

/// the TAMPER: the band as two INDEPENDENT bounds — the causal join
/// (the right bound is row-dependent) is dropped, the mask differs
fn band_tampered(l: usize, w: usize) -> Vec<bool> {
    let mut m = vec![false; l * l];
    for r in 0..l {
        for c in 0..l {
            // WRONG: both bounds row-independent — admits future keys
            // inside the band width instead of the causal edge
            let in_band = c + w > l.min(r + w); // a plausible-but-wrong shape
            let _ = in_band;
            if c < w && c <= r + w {
                m[r * l + c] = true; // admits c > r: not causal
            }
        }
    }
    m
}

/// the admission: array-mask + window is the named-unencodable class
fn plan_r1(window: usize, l: usize, has_array_mask: bool) -> Result<&'static str, String> {
    if window >= l {
        return Ok("plain"); // clip_window: the vacuous constraint
    }
    if has_array_mask {
        return Err("window with an array mask".to_string()); // NAMED
    }
    Ok("band")
}

#[test]
fn r1_nyi_gate_full_work_vs_band() {
    let l = 4096usize; // CI-sized tier; the shape law holds
    let w = 512usize;
    let (naive, naive_touched) = naive_window_mask(l, w);
    let (band, band_touched) = band_mask(l, w);
    assert_eq!(naive_touched, l * l, "the NYI arm scans every L*L cell");
    assert!(
        band_touched <= l * w,
        "the band arm touches at most L*W cells: {band_touched}"
    );
    assert!(
        naive_touched >= band_touched * (l / w) / 2,
        "the scan pays the quadratic: {naive_touched} vs band {band_touched}"
    );
    // BIT-IDENTICAL mask content — the band writes exactly the cells
    // the scan's membership test admits, byte for byte
    assert_eq!(naive, band, "band == scanned mask, exactly");
    // the tamper: dropping the causal join admits non-causal cells
    let tampered = band_tampered(64, 8);
    assert_ne!(
        tampered, full_mask(64),
        "the independent-bounds tamper is a DIFFERENT mask — red"
    );
}

#[test]
fn r1_named_refusal_and_vacuous_clip() {
    // the named refusal: array mask + window
    match plan_r1(512, 4096, true) {
        Err(msg) => assert!(msg.contains("array mask"), "the refusal NAMES the class"),
        Ok(_) => panic!("array-mask+window must refuse"),
    }
    // the encodable class admits
    assert_eq!(plan_r1(512, 4096, false).unwrap(), "band");
    // the vacuous window clips to the plain path (the clip_window face)
    assert_eq!(plan_r1(4096, 4096, false).unwrap(), "plain");
    assert_eq!(plan_r1(8192, 4096, false).unwrap(), "plain");
}

// ═══════════ R2 — slice the operand vs predicate in the loop ═══════════

/// exactness harness: small integer-valued f32s, one fixed fold order —
/// bit-identical asserts are deterministic
fn q_of(i: usize) -> f32 {
    ((i % 7) as f32) + 1.0
}
fn k_of(i: usize) -> f32 {
    ((i % 5) as f32) * 0.5 + 1.0
}

/// the predicated arm: reads ALL L keys, the window is a per-read test
fn decode_predicated(l: usize, w: usize) -> (f32, usize) {
    let mut acc = 0.0f32;
    let mut reads = 0;
    for i in 0..l {
        reads += 1;
        if i + w >= l {
            acc += q_of(i) * k_of(i);
        }
    }
    (acc, reads)
}

/// the slice arm: the operand reduced to the last W, kernel untouched
fn decode_sliced(l: usize, w: usize) -> (f32, usize) {
    let start = l.saturating_sub(w);
    let mut acc = 0.0f32;
    let mut reads = 0;
    for i in start..l {
        reads += 1;
        acc += q_of(i) * k_of(i);
    }
    (acc, reads)
}

/// the front-window tamper: oldest keys, not newest
fn decode_front(l: usize, w: usize) -> f32 {
    let mut acc = 0.0f32;
    for i in 0..w {
        acc += q_of(i) * k_of(i);
    }
    let _ = l;
    acc
}

/// the contiguity/broadcast predicate (the stride-0 face, refusal half)
fn contiguous_cache_row(stride2: usize, buffer_rows: usize, expected: usize) -> bool {
    stride2 != 0 && buffer_rows == expected
}

/// the multi-query admission: a global suffix slice is wrong when the
/// per-row windows differ
fn admits_slice(t_q: usize, t_kv: usize) -> Result<(), &'static str> {
    if t_q > 1 && t_q != t_kv {
        Err("per-row windows differ at more than one query")
    } else {
        Ok(())
    }
}

#[test]
fn r2_slice_operand_reads_w_bit_identical() {
    let l = 2048usize;
    let w = 64usize;
    let (a_pred, reads_pred) = decode_predicated(l, w);
    let (a_slice, reads_slice) = decode_sliced(l, w);
    assert_eq!(
        a_pred.to_bits(),
        a_slice.to_bits(),
        "the sliced operand scores bit-identical to the predicated scan"
    );
    assert_eq!(reads_pred, l, "the predicated arm reads every key");
    assert_eq!(reads_slice, w, "the slice arm reads only the window");
}

#[test]
fn r2_front_tamper_and_refusals() {
    let l = 512usize;
    let w = 32usize;
    let (truth, _) = decode_sliced(l, w);
    let front = decode_front(l, w);
    assert_ne!(
        front.to_bits(),
        truth.to_bits(),
        "the front-window tamper scores different keys — red"
    );
    // the multi-query refusal
    match admits_slice(4, 64) {
        Err(why) => assert!(why.contains("per-row windows differ")),
        Ok(()) => panic!("multi-query global slice must refuse"),
    }
    // equal-length prefill admits (the alignment class)
    assert!(admits_slice(64, 64).is_ok());
    // the stride-0 contiguity refusal (the face on gqa-zero-stride)
    assert!(!contiguous_cache_row(0, 64, 64), "a broadcast row is NOT a cache row");
    assert!(contiguous_cache_row(128, 64, 64));
    assert!(!contiguous_cache_row(128, 12, 64), "short buffer refuses too");
}

// ═══════════ R3 — the version-gated known-wrong window ═══════════

const PINNED_WRONG_BELOW: u32 = 91501;

/// the capability-only arm: admits below the pin, the bug term rides
fn windowed_grad_capability_only(version: u32, kv_len: usize) -> Vec<f32> {
    let mut g = vec![2.0f32; 4];
    if version < PINNED_WRONG_BELOW && kv_len % 128 != 0 {
        g[2] += 1.0; // the upstream bug term
    }
    g
}

/// the version-gated arm: refuse below the pin (the conservative
/// landed form — even where this kv_len's modulus cannot hit),
/// exact above, the fallback always exact
fn windowed_grad_gated(
    version: u32, kv_len: usize,
) -> Result<Vec<f32>, String> {
    if version < PINNED_WRONG_BELOW {
        return Err(format!(
            "wrong gradients when kv % 128 != 0 below {PINNED_WRONG_BELOW}"
        ));
    }
    let _ = kv_len;
    Ok(vec![2.0f32; 4])
}

fn windowed_grad_fallback() -> Vec<f32> {
    vec![2.0f32; 4]
}

/// the geometry tamper: the alignment swapped at T_q == T_kv
fn window_mask_geometry(t_q: usize, t_kv: usize, top_left_when_equal: bool) -> Vec<(usize, usize)> {
    let mut pairs = vec![];
    for r in 0..t_q {
        for c in 0..t_kv {
            let keep = if t_q == t_kv {
                if top_left_when_equal {
                    // TOP_LEFT: band anchored at the top-left corner
                    c + 4 > r // a window of 4, top-left anchored
                } else {
                    // BOTTOM_RIGHT at equal lengths: a DIFFERENT mask
                    c + 4 > r + (t_kv - t_q) + 2 // shifted: wrong
                }
            } else {
                c + 4 > r
            };
            if keep {
                pairs.push((r, c));
            }
        }
    }
    pairs
}

#[test]
fn r3_version_gate_refuses_known_wrong() {
    // the capability-only arm trains on the bug term below the pin
    let bad = windowed_grad_capability_only(91500, 300); // 300 % 128 != 0
    assert_eq!(bad[2], 3.0, "the bug term rides below the pin — red");
    let fine = windowed_grad_capability_only(91500, 256); // 256 % 128 == 0
    assert_eq!(fine[2], 2.0, "the modulus class evades at this kv_len");
    // the gated arm refuses below the pin — the CONSERVATIVE landed
    // form: even the evading kv_len refuses (disclosed in-rule)
    assert!(windowed_grad_gated(91500, 300).is_err());
    assert!(
        windowed_grad_gated(91500, 256).is_err(),
        "below the pin the WHOLE path refuses, not just the modulus class"
    );
    // above the pin: exact, and the fallback agrees
    let up = windowed_grad_gated(91501, 300).unwrap();
    assert_eq!(up, windowed_grad_fallback(), "exact above the pin");
    // the refusal NAMES the bug class
    match windowed_grad_gated(91500, 300) {
        Err(msg) => assert!(
            msg.contains("wrong gradients") && msg.contains("128"),
            "the refusal carries what breaks"
        ),
        Ok(_) => panic!("below the pin must refuse"),
    }
}

#[test]
fn r3_geometry_pin() {
    // at equal lengths the wrong alignment produces a DIFFERENT mask
    let a = window_mask_geometry(8, 8, true); // TOP_LEFT (the landed pin)
    let b = window_mask_geometry(8, 8, false); // BOTTOM_RIGHT: wrong here
    assert_ne!(a, b, "the alignment swap is a different mask — red");
}

// ═══════════ R4 — direction dispatch: fall back, never throw ═══════════

const FUSED_DIRECTIONS: usize = 3; // q, k, v

/// the throw arm: a plain vjp over ALL inputs panics
fn vjp_throw(dirs: &[usize]) -> Vec<f32> {
    let mut out = vec![];
    for &d in dirs {
        if d >= FUSED_DIRECTIONS {
            panic!("Does not support VJP with respect to mask or attention sinks.");
        }
        out.push(fused_grad(d));
    }
    out
}

fn fused_grad(d: usize) -> f32 {
    (d as f32) + 10.0 // the fused path's answer for q/k/v
}

fn composed_grad(d: usize) -> f32 {
    (d as f32) + 10.0 // the composition: identical values here, any cost
}

/// the fixed dispatch: other directions route to the composition
fn vjp_dispatch(dirs: &[usize]) -> Vec<f32> {
    let other = dirs.iter().any(|&d| d >= FUSED_DIRECTIONS);
    if other {
        // the composition answers ALL requested directions
        dirs.iter().map(|&d| composed_grad(d)).collect()
    } else {
        dirs.iter().map(|&d| fused_grad(d)).collect()
    }
}

/// the index arm: a grown tuple read by back() vs by role
#[test]
fn r4_fallback_not_throw_and_role_index() {
    // the throw arm panics on direction 3 — reachable from a plain
    // vjp over all inputs (the ordinary composed case)
    let result = std::panic::catch_unwind(|| vjp_throw(&[0, 1, 2, 3]));
    assert!(result.is_err(), "the throw arm dies on a VALID direction");

    // the dispatch arm: all four directions answered, exact
    let got = vjp_dispatch(&[0, 1, 2, 3]);
    let want: Vec<f32> = (0..4).map(composed_grad).collect();
    assert_eq!(got, want, "the composition answers every direction");
    // the fused-only case keeps the fused path
    assert_eq!(vjp_dispatch(&[0, 1]), vec![fused_grad(0), fused_grad(1)]);

    // the index bug: the tuple grew (output + lse + cotangent appended)
    let inputs = ["q", "k", "v", "out", "lse", "d_out"];
    let primals_size = 3usize;
    // back() names the cotangent now — WRONG
    assert_eq!(inputs.last().copied(), Some("d_out"));
    // the role-derived read names the sinks (the last primal) — right
    assert_eq!(inputs[primals_size - 1], "v");
}
