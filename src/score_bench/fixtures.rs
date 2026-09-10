//! Frozen clippy-bench fixture set — Plan 014 Phase 1.5.
//!
//! MINED, not authored: every span is a real code chunk extracted from a
//! PRE-SWEEP commit of a sibling repo (`git show <sha>:<path>`), labeled by
//! real `cargo clippy` (all+pedantic+nursery) in a bare stub crate at freeze
//! time. Provenance recorded per fixture; the set is FROZEN — corpus growth
//! never moves the denominator.
//!
//! Regenerate/extend ONLY via `examples/score_bench_mine.rs` (Plan 014
//! discipline: new fixtures = a new bench version, not silent edits).
//!
//! v2 extension (P9, 2026-08-25): `riir-ai-ruliology-player-001` — the
//! production verify-revert span that motivated the `map_unwrap_or`
//! trait-object-cast guard (riir-clippy post-mining P9). Added as a
//! deliberate bench-version step: it pins the guard on the `parse_safety`
//! axis (pre-guard, the healed output E0308s; post-guard it compiles and
//! the lint persists as the documented we-stay-silent divergence).

/// The frozen v1 fixture set.
#[must_use]
pub fn fixtures() -> Vec<crate::score_bench::BenchFixture> {
    use crate::score_bench::BenchFixture;
    FIXTURE_DATA
        .iter()
        .map(|f| BenchFixture {
            id: f.id,
            provenance: f.provenance,
            code: f.code,
            expect_lints: f.expect_lints,
            clean: f.clean,
        })
        .collect()
}

/// Raw frozen fixture storage (mined 2026-08-19; see provenance fields).
struct RawFixture {
    id: &'static str,
    provenance: &'static str,
    code: &'static str,
    expect_lints: &'static [&'static str],
    clean: bool,
}

static FIXTURE_DATA: &[RawFixture] = &[
    RawFixture {
        id: "katgpt-rs-bench_ptrm_width_scaling-002",
        provenance: "katgpt-rs@9e0e6a57:bench_ptrm_width_scaling.rs",
        code: r"/// Compute greedy argmax path as baseline reference.
fn greedy_path(marginals: &[Vec<f32>]) -> Vec<usize> {
    marginals
        .iter()
        .map(|m| {
            m.iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0)
        })
        .collect()
}
",
        expect_lints: &["map_unwrap_or"],
        clean: false,
    },
    RawFixture {
        id: "composite-return_plus_field_names",
        provenance: "authored-composite:T3.5.5:return_plus_field_names",
        code: r"pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

pub fn make_viewport(width: u32, height: u32) -> Viewport {
    return Viewport { width: width, height: height };
}
",
        expect_lints: &["needless_return", "redundant_field_names"],
        clean: false,
    },
    RawFixture {
        id: "composite-return_plus_identity_op",
        provenance: "authored-composite:T3.5.5:return_plus_identity_op",
        code: r"pub fn scale_ticks(ticks: u32) -> u32 {
    return ticks * 1;
}
",
        expect_lints: &["identity_op", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-return_plus_useless_format",
        provenance: "authored-composite:T3.5.5:return_plus_useless_format",
        code: r#"pub fn banner_text() -> String {
    return format!("wave cleared");
}
"#,
        expect_lints: &["needless_return", "useless_format"],
        clean: false,
    },
    RawFixture {
        id: "composite-return_plus_double_parens",
        provenance: "authored-composite:T3.5.5:return_plus_double_parens",
        code: r"pub fn next_index(current: i32) -> i32 {
    return ((current + 1));
}
",
        expect_lints: &["double_parens", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-unit_return_plus_needless_return",
        provenance: "authored-composite:T3.5.5:unit_return_plus_needless_return",
        code: r"pub fn record_tick(tick: u32) -> () {
    let _ = tick;
    return;
}
",
        expect_lints: &["needless_return", "unused_unit"],
        clean: false,
    },
    RawFixture {
        id: "composite-range_loop_plus_identity_op",
        provenance: "authored-composite:T3.5.5:range_loop_plus_identity_op",
        code: r"pub fn total_damage(hits: &[u32]) -> u32 {
    let mut total = 0;
    for i in 0..hits.len() {
        total += hits[i] * 1;
    }
    total
}
",
        expect_lints: &["identity_op", "needless_range_loop"],
        clean: false,
    },
    RawFixture {
        id: "composite-single_match_plus_return",
        provenance: "authored-composite:T3.5.5:single_match_plus_return",
        code: r"pub fn first_score(entry: Option<u32>) -> u32 {
    match entry {
        Some(value) => return value,
        _ => {}
    }
    return 0;
}
",
        expect_lints: &["needless_return", "single_match"],
        clean: false,
    },
    RawFixture {
        id: "composite-filter_next_plus_redundant_closure",
        provenance: "authored-composite:T3.5.5:filter_next_plus_redundant_closure",
        code: r"pub fn first_alive(values: &[u32]) -> Option<&u32> {
    values.iter().filter(|value| is_alive(value)).next()
}

fn is_alive(value: &&u32) -> bool {
    **value > 0
}
",
        expect_lints: &["filter_next"],
        clean: false,
    },
    RawFixture {
        id: "composite-manual_map_plus_return",
        provenance: "authored-composite:T3.5.5:manual_map_plus_return",
        code: r"pub fn doubled(entry: Option<u32>) -> Option<u32> {
    return match entry {
        Some(value) => Some(value * 2),
        None => None,
    };
}
",
        expect_lints: &["manual_map", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-needless_bool_plus_return",
        provenance: "authored-composite:T3.5.5:needless_bool_plus_return",
        code: r"pub fn is_ready(flag: bool) -> bool {
    return if flag { true } else { false };
}
",
        expect_lints: &["needless_bool", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-collapsible_if_plus_return",
        provenance: "authored-composite:T3.5.5:collapsible_if_plus_return",
        code: r"pub fn gate(outer: bool, inner: bool) -> u32 {
    if outer {
        if inner {
            return 1;
        }
    }
    return 0;
}
",
        expect_lints: &["collapsible_if", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-static_lifetime_plus_return",
        provenance: "authored-composite:T3.5.5:static_lifetime_plus_return",
        code: r#"pub const REALM_TAG: &'static str = "ember-keep";

pub fn realm() -> &'static str {
    return REALM_TAG;
}
"#,
        expect_lints: &["needless_return", "redundant_static_lifetimes"],
        clean: false,
    },
    RawFixture {
        id: "composite-iter_nth_zero_plus_return",
        provenance: "authored-composite:T3.5.5:iter_nth_zero_plus_return",
        code: r"pub fn leader(values: &[u32]) -> Option<&u32> {
    return values.iter().nth(0);
}
",
        expect_lints: &["needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-redundant_pattern_matching_plus_return",
        provenance: "authored-composite:T3.5.5:redundant_pattern_matching_plus_return",
        code: r"pub fn has_value(entry: Option<u32>) -> bool {
    return match entry {
        Some(_) => true,
        None => false,
    };
}
",
        expect_lints: &["needless_return", "redundant_pattern_matching"],
        clean: false,
    },
    RawFixture {
        id: "composite-unnecessary_lazy_plus_return",
        provenance: "authored-composite:T3.5.5:unnecessary_lazy_plus_return",
        code: r"pub fn resolve(entry: Option<u32>) -> u32 {
    return entry.unwrap_or_else(|| 0);
}
",
        expect_lints: &["needless_return", "unnecessary_lazy_evaluations"],
        clean: false,
    },
    RawFixture {
        id: "composite-map_unwrap_or_plus_return",
        provenance: "authored-composite:T3.5.5:map_unwrap_or_plus_return",
        code: r"pub fn width_or_default(entry: Option<u32>) -> u32 {
    return entry.map(|value| value * 2).unwrap_or(0);
}
",
        expect_lints: &["map_unwrap_or", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-deref_addrof_plus_return",
        provenance: "authored-composite:T3.5.5:deref_addrof_plus_return",
        code: r"pub fn read_health(health: &u32) -> u32 {
    return *&*health;
}
",
        expect_lints: &["deref_addrof", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-question_mark_plus_return",
        provenance: "authored-composite:T3.5.5:question_mark_plus_return",
        code: r"pub fn lookup(entry: Option<u32>) -> Option<u32> {
    if entry.is_none() {
        return None;
    }
    return Some(entry.unwrap() + 1);
}
",
        expect_lints: &["needless_return", "question_mark"],
        clean: false,
    },
    RawFixture {
        id: "composite-three_way_return_field_identity",
        provenance: "authored-composite:T3.5.5:three_way_return_field_identity",
        code: r"pub struct Shot {
    pub power: u32,
}

pub fn fire(power: u32) -> Shot {
    return Shot { power: power * 1 };
}
",
        expect_lints: &["identity_op", "needless_return"],
        clean: false,
    },
    RawFixture {
        id: "composite-field_names_plus_identity_op",
        provenance: "authored-composite:T3.5.5:field_names_plus_identity_op",
        code: r"pub struct Beam {
    pub power: u32,
    pub width: u32,
}

pub fn beam(power: u32, width: u32) -> Beam {
    Beam { power: power * 1, width: width }
}
",
        expect_lints: &["identity_op", "redundant_field_names"],
        clean: false,
    },
    RawFixture {
        id: "composite-range_loop_plus_double_parens",
        provenance: "authored-composite:T3.5.5:range_loop_plus_double_parens",
        code: r"pub fn offsets(values: &[u32]) -> Vec<u32> {
    let mut out = Vec::new();
    for i in 0..values.len() {
        out.push(((values[i] + 1)));
    }
    out
}
",
        expect_lints: &["double_parens", "needless_range_loop"],
        clean: false,
    },
    RawFixture {
        id: "composite-needless_bool_plus_identity_op",
        provenance: "authored-composite:T3.5.5:needless_bool_plus_identity_op",
        code: r"pub fn charged(power: u32) -> bool {
    if power * 1 > 10 { true } else { false }
}
",
        expect_lints: &["identity_op", "needless_bool"],
        clean: false,
    },
    RawFixture {
        id: "composite-collapsible_if_plus_identity_op",
        provenance: "authored-composite:T3.5.5:collapsible_if_plus_identity_op",
        code: r"pub fn tier(power: u32, active: bool) -> u32 {
    if active {
        if power * 1 > 5 {
            return 2;
        }
    }
    0
}
",
        expect_lints: &["collapsible_if", "identity_op"],
        clean: false,
    },
    RawFixture {
        id: "composite-map_unwrap_or_plus_useless_format",
        provenance: "authored-composite:T3.5.5:map_unwrap_or_plus_useless_format",
        code: r#"pub fn describe(entry: Option<u32>) -> String {
    entry.map(|_| format!("some")).unwrap_or(String::new())
}
"#,
        expect_lints: &["map_unwrap_or", "useless_format"],
        clean: false,
    },
    RawFixture {
        id: "composite-deref_addrof_plus_field_names",
        provenance: "authored-composite:T3.5.5:deref_addrof_plus_field_names",
        code: r"pub struct Slot {
    pub value: u32,
}

pub fn slot(value: &u32) -> Slot {
    Slot { value: *&*value }
}
",
        expect_lints: &["deref_addrof"],
        clean: false,
    },
    RawFixture {
        id: "composite-static_lifetime_plus_useless_format",
        provenance: "authored-composite:T3.5.5:static_lifetime_plus_useless_format",
        code: r#"pub const TAG: &'static str = "keep";

pub fn tagged() -> String {
    format!("static")
}
"#,
        expect_lints: &["redundant_static_lifetimes", "useless_format"],
        clean: false,
    },
    RawFixture {
        id: "composite-range_loop_plus_format_args",
        provenance: "authored-composite:T3.5.5:range_loop_plus_format_args",
        code: r#"pub fn describe_all(names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..names.len() {
        out.push(format!("{}", names[i]));
    }
    out
}
"#,
        expect_lints: &["needless_range_loop", "useless_format"],
        clean: false,
    },
    RawFixture {
        id: "katgpt-rs-quantile_balance_router-002-clean",
        provenance: "katgpt-rs@8da93f6c:quantile_balance_router.rs",
        code: r"/// Compute the post-balancing load-balance violation (MaxVio) for a given
/// bias vector `β`.
///
/// `MaxVio = max_j |count_j − m·k/n| / (m·k/n)`
///
/// where `count_j` is the number of calibration tokens that would pick expert
/// `j` under `top-k(s − β)`. Vanilla MoE has high MaxVio (skewed experts);
/// QB drives it → 0.
///
/// Public so Plan 455 Phase 3 head-to-head can measure MaxVio on MPI-
/// conditioned routers too (without QB).
pub fn compute_balance_violation(s: &[f32], m: usize, n: usize, k: usize, beta: &[f32]) -> f32 {
    debug_assert_eq!(s.len(), m * n);
    debug_assert_eq!(beta.len(), n);
    if m == 0 || n == 0 || k == 0 {
        return 0.0;
    }
    let ideal = (m as f32) * (k as f32) / (n as f32);
    if ideal == 0.0 {
        return 0.0;
    }
    let mut counts = vec![0usize; n];
    let mut row_buf: Vec<f32> = vec![0.0; n];
    for i in 0..m {
        let row = &s[i * n..(i + 1) * n];
        for j in 0..n {
            row_buf[j] = row[j] - beta[j];
        }
        // Find top-k indices in row_buf. For small n (typical game-scale
        // N ≤ 256), selection sort is cache-friendly.
        let kk = k.min(n);
        let mut idx: Vec<usize> = (0..n).collect();
        for r in 0..kk {
            let mut best = r;
            let mut best_score = row_buf[idx[r]];
            for c in (r + 1)..n {
                if row_buf[idx[c]] > best_score {
                    best = c;
                    best_score = row_buf[idx[c]];
                }
            }
            idx.swap(r, best);
            counts[idx[r]] += 1;
        }
    }
    let mut max_vio: f32 = 0.0;
    for &count in counts.iter() {
        let deviation = ((count as f32) - ideal).abs() / ideal;
        if deviation > max_vio {
            max_vio = deviation;
        }
    }
    max_vio
}",
        expect_lints: &[],
        clean: true,
    },
    RawFixture {
        id: "katgpt-rs-quantile_balance_router-003-clean",
        provenance: "katgpt-rs@8da93f6c:quantile_balance_router.rs",
        code: r"/// In-place quantile via sort + index.
///
/// Returns the value at probability `q ∈ [0, 1]` (so `q=0.5` is the median).
/// Linearly interpolates between adjacent order statistics (Type 7, same as
/// NumPy's default) to match the reference implementation.
///
/// The input slice is sorted in place — callers should treat it as mutably
/// borrowed. O(n log n) via `sort_by` (Rust's pdqsort).
///
/// # Perf note (Phase 2 follow-up)
///
/// Phase 1 uses sort (O(n log n)) for correctness simplicity. The reference
/// impl uses `np.quantile` which is also sort-based under the hood. For the
/// QB hot path at game scale (n ≤ 256), sort cost is ~2μs per quantile call,
/// well under the 1ms G4 target. If G4 ever fails, the obvious optimization
/// is `slice::select_nth_unstable` (O(n) average) — but the std API returns
/// a slice view, not a single element, in this toolchain, which makes the
/// code more awkward than the sort path. Premature optimization avoided.
fn quantile_in_place(data: &mut [f32], q: f32) -> f32 {
    let n = data.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return data[0];
    }
    // Sort ascending. f32 has no Ord (NaN), so use partial_cmp with a total
    // fallback. NaN (if present) sorts as less than everything — but the QB
    // caller contract says no NaN in input scores, so this is defensive only.
    data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Type 7 quantile (NumPy default): pos = q * (n - 1).
    let pos = q.clamp(0.0, 1.0) * (n as f32 - 1.0);
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = pos - lo as f32;
    data[lo] * (1.0 - frac) + data[hi] * frac
}",
        expect_lints: &[],
        clean: true,
    },
    RawFixture {
        id: "katgpt-rs-quantile_balance_router-005-clean",
        provenance: "katgpt-rs@8da93f6c:quantile_balance_router.rs",
        code: r#"/// Zero-alloc variant of [`route_with_bias`].
///
/// Identical math and ordering, but writes selected indices into caller-
/// owned `idx_buf` (length `>= n`) and returns the truncation length
/// `kk = min(k, n)`. Mirrors Plan 279 `gate_sigmoid_topk_into`.
pub fn route_with_bias_into(
    s_row: &[f32],
    beta: &[f32],
    k: usize,
    out_scores: &mut [f32],
    idx_buf: &mut [usize],
) -> usize {
    debug_assert_eq!(s_row.len(), beta.len(), "s_row / beta length mismatch");
    debug_assert_eq!(out_scores.len(), s_row.len(), "out_scores length mismatch");
    debug_assert_eq!(idx_buf.len(), s_row.len(), "idx_buf length mismatch");
    let n = s_row.len();
    let kk = k.min(n);

    for j in 0..n {
        out_scores[j] = s_row[j] - beta[j];
    }
    // Initialize idx_buf to [0, 1, ..., n-1].
    for (i, slot) in idx_buf.iter_mut().enumerate() {
        *slot = i;
    }
    for r in 0..kk {
        let mut best = r;
        let mut best_score = out_scores[idx_buf[r]];
        for c in (r + 1)..n {
            if out_scores[idx_buf[c]] > best_score {
                best = c;
                best_score = out_scores[idx_buf[c]];
            }
        }
        idx_buf.swap(r, best);
    }
    kk
}"#,
        expect_lints: &[],
        clean: true,
    },
    RawFixture {
        id: "katgpt-rs-ica_lens-004-clean",
        provenance: "katgpt-rs@8da93f6c:ica_lens.rs",
        code: r"/// Jacobi eigendecomposition of a symmetric `n × n` matrix `a` (f32).
///
/// Writes eigenvalues into `eigvals[..n]` and eigenvectors into
/// `eigvecs[..n*n]` (column `j` = eigenvector for `eigvals[j]`).
///
/// `scratch[..n*n]` is mutated (working copy of `a`).
///
/// Mirrors the f32 Jacobi pattern in `katgpt-spectral/src/river_valley.rs`
/// (eigenvalues-only variant) extended to track eigenvectors.
fn jacobi_eig_symmetric_into(
    a: &[f32],
    n: usize,
    max_sweeps: usize,
    eigvals: &mut [f32],
    eigvecs: &mut [f32],
    scratch: &mut [f32],
) {
    if n == 0 {
        return;
    }
    if n == 1 {
        eigvals[0] = a[0];
        eigvecs[0] = 1.0;
        return;
    }

    scratch[..n * n].copy_from_slice(&a[..n * n]);
    for i in 0..n {
        for j in 0..n {
            eigvecs[i * n + j] = if i == j { 1.0 } else { 0.0 };
        }
    }

    let sweeps = max_sweeps.max(20);
    for _ in 0..sweeps {
        let mut max_off = 0.0_f32;
        for p in 0..n {
            for q in (p + 1)..n {
                let val = scratch[p * n + q].abs();
                if val > max_off {
                    max_off = val;
                }
            }
        }
        if max_off < 1e-12 {
            break;
        }

        for p in 0..n {
            for q in (p + 1)..n {
                let apq = scratch[p * n + q];
                if apq.abs() < 1e-15 {
                    continue;
                }
                let app = scratch[p * n + p];
                let aqq = scratch[q * n + q];

                let tau = (aqq - app) / (2.0 * apq);
                let t = 1.0f32.copysign(tau) / (tau.abs() + (1.0 + tau * tau).sqrt());
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;

                scratch[p * n + p] = app - t * apq;
                scratch[q * n + q] = aqq + t * apq;
                scratch[p * n + q] = 0.0;
                scratch[q * n + p] = 0.0;

                for r in 0..n {
                    if r == p || r == q {
                        continue;
                    }
                    let arp = scratch[r * n + p];
                    let arq = scratch[r * n + q];
                    let new_rp = c * arp - s * arq;
                    let new_rq = s * arp + c * arq;
                    scratch[r * n + p] = new_rp;
                    scratch[p * n + r] = new_rp;
                    scratch[r * n + q] = new_rq;
                    scratch[q * n + r] = new_rq;
                }

                // Update eigenvectors: V → V · R(p, q, θ).
                for i in 0..n {
                    let vip = eigvecs[i * n + p];
                    let viq = eigvecs[i * n + q];
                    eigvecs[i * n + p] = c * vip - s * viq;
                    eigvecs[i * n + q] = s * vip + c * viq;
                }
            }
        }
    }

    for i in 0..n {
        eigvals[i] = scratch[i * n + i].max(0.0);
    }
}",
        expect_lints: &[],
        clean: true,
    },
    RawFixture {
        id: "katgpt-rs-ica_lens-006-clean",
        provenance: "katgpt-rs@8da93f6c:ica_lens.rs",
        code: r"/// Compute the pseudoinverse `D = R^†` (D × m, row-major) of the reading map
/// `R` (m × D, row-major).
///
/// For `m ≤ D` (undercomplete ICA), `R^† = R^T · (R · R^T)^{-1}`. We compute
/// the inverse of the `m × m` matrix `R · R^T` via Gauss-Jordan elimination
/// on an augmented `[R R^T | I]` system.
///
/// Column `j` (stride `m`) of `D` is the activation-space direction vector
/// for component `j`.
#[allow(clippy::too_many_arguments)]
fn pseudoinverse_into(
    reading_map: &[f32],
    out_writing_map: &mut [f32],
    rrt: &mut [f32],
    aug: &mut [f32],
    m: usize,
    d: usize,
) {
    // R · R^T (m × m).
    for i in 0..m {
        for j in 0..m {
            let mut acc = 0.0_f32;
            for k in 0..d {
                acc += reading_map[i * d + k] * reading_map[j * d + k];
            }
            rrt[i * m + j] = acc;
        }
    }

    // Augmented [R R^T | I] (m × 2m).
    for i in 0..m {
        for j in 0..m {
            aug[i * (2 * m) + j] = rrt[i * m + j];
            aug[i * (2 * m) + m + j] = if i == j { 1.0 } else { 0.0 };
        }
    }

    // Gauss-Jordan with partial pivoting.
    for col in 0..m {
        let mut pivot_row = col;
        let mut max_abs = aug[col * (2 * m) + col].abs();
        for row in (col + 1)..m {
            let val = aug[row * (2 * m) + col].abs();
            if val > max_abs {
                max_abs = val;
                pivot_row = row;
            }
        }
        if max_abs < 1e-12 {
            continue;
        }
        if pivot_row != col {
            for j in 0..(2 * m) {
                aug.swap(col * (2 * m) + j, pivot_row * (2 * m) + j);
            }
        }
        let pivot = aug[col * (2 * m) + col];
        let inv_pivot = 1.0 / pivot;
        for j in 0..(2 * m) {
            aug[col * (2 * m) + j] *= inv_pivot;
        }
        for row in 0..m {
            if row == col {
                continue;
            }
            let factor = aug[row * (2 * m) + col];
            if factor == 0.0 {
                continue;
            }
            for j in 0..(2 * m) {
                aug[row * (2 * m) + j] -= factor * aug[col * (2 * m) + j];
            }
        }
    }

    // D = R^T · (R R^T)^{-1}.
    // D[i, j] = Σ_k R[k, i] · (R R^T)^{-1}[k, j]
    //        = Σ_k reading_map[k*d + i] · aug[k*(2m) + m + j]
    for i in 0..d {
        for j in 0..m {
            let mut acc = 0.0_f32;
            for k in 0..m {
                acc += reading_map[k * d + i] * aug[k * (2 * m) + m + j];
            }
            out_writing_map[i * m + j] = acc;
        }
    }
}",
        expect_lints: &[],
        clean: true,
    },
    RawFixture {
        id: "katgpt-rs-ica_lens-013-clean",
        provenance: "katgpt-rs@8da93f6c:ica_lens.rs",
        code: r"/// Compute the Effective Receptive Field of a single component at a target
/// position.
///
/// Given the component's signed score at the target under (a) full context
/// and (b) suffixes of increasing length, returns the minimum suffix length
/// `k` such that the component is in the top-N by absolute score AND
/// preserves its sign.
///
/// Returns the last schedule entry if no suffix recovers within the window.
///
/// # Arguments
///
/// * `scores_full` — the component's signed score at each position under full
///   context.
/// * `target_idx` — index of the target position.
/// * `suffix_scores` — the component's signed score at the target under each
///   suffix length in `schedule`. Length must equal `schedule.len()`.
/// * `schedule` — the suffix lengths to try (e.g. [`DEFAULT_ERF_SCHEDULE`]).
/// * `top_n` — the top-N threshold (paper uses 15). Component must be in the
///   top-N by absolute score under the suffix.
pub fn effective_receptive_field(
    scores_full: &[f32],
    target_idx: usize,
    suffix_scores: &[f32],
    schedule: &[usize],
    top_n: usize,
) -> usize {
    let full_score = scores_full[target_idx];
    let full_sign = full_score.signum();
    if full_sign == 0.0 {
        return schedule.first().copied().unwrap_or(1);
    }

    let mut abs_scores: Vec<f32> = scores_full.iter().map(|x| x.abs()).collect();
    abs_scores.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let top_n_threshold = if top_n >= abs_scores.len() {
        0.0
    } else {
        abs_scores[top_n - 1]
    };

    for (k_idx, &k) in schedule.iter().enumerate() {
        if k_idx >= suffix_scores.len() {
            break;
        }
        let suffix_score = suffix_scores[k_idx];
        let suffix_sign = suffix_score.signum();
        if suffix_sign != full_sign && suffix_sign != 0.0 {
            continue;
        }
        if suffix_score.abs() >= top_n_threshold {
            return k;
        }
    }

    *schedule.last().unwrap_or(&1)
}",
        expect_lints: &[],
        clean: true,
    },
    // v2 (P9, 2026-08-25): the riir-ai ruliology/player.rs span whose heal
    // E0308'd in production (the verify gate's first multi-edit whole-file
    // revert). Real clippy fires ONLY map_unwrap_or here (probe-verified at
    // freeze: single_match_else does NOT fire on the default_score match —
    // both-arm-blocks + None-second is not its firing shape on 1.97). The
    // healer rewrites the two matches anyway (its own detection is wider);
    // the load-bearing measurement is parse_safety: post-guard the healed
    // span compiles (2 sound if-let rewrites, map_or declined); a guard
    // regression re-breaks it and drops parse_safety below 1.0.
    RawFixture {
        id: "riir-ai-ruliology-player-001",
        provenance: "riir-ai@849ce8330:crates/riir-games/src/ruliology/player.rs (P9; trait scaffolding added, lint spans verbatim)",
        code: r"pub trait StrategyPlayer {
    fn name(&self) -> &str;
    fn paradigm(&self) -> Paradigm;
    fn select_action(&mut self, state: &[f32], opponent_history: &[usize]) -> usize;
}
pub enum Paradigm {
    HeuristicLearning,
}

pub type ScoreFn = dyn Fn(usize, &[f32], &[usize]) -> f64 + Send + Sync;

pub struct HlPlayer {
    action_count: usize,
    score_fn: Option<Box<ScoreFn>>,
    name: String,
}

impl HlPlayer {
    fn default_score(action: usize, state: &[f32], opponent_history: &[usize]) -> f64 {
        let base = match opponent_history.last() {
            Some(&last) => {
                // Cooperative: high score for matching opponent's last action
                if action == last { 1.0 } else { 0.3 }
            }
            None => {
                // No history: prefer action 0 (cooperative default)
                if action == 0 { 1.0 } else { 0.5 }
            }
        };

        // State modulation: use state observation to adjust scores
        let state_bonus = match state.first() {
            Some(&s) if s > 0.5 => 0.1,
            _ => 0.0,
        };

        base + state_bonus
    }
}

impl StrategyPlayer for HlPlayer {
    fn name(&self) -> &str {
        &self.name
    }

    fn paradigm(&self) -> Paradigm {
        Paradigm::HeuristicLearning
    }

    fn select_action(&mut self, state: &[f32], opponent_history: &[usize]) -> usize {
        let scorer = self
            .score_fn
            .as_ref()
            .map(|f| f as &dyn Fn(usize, &[f32], &[usize]) -> f64)
            .unwrap_or(&(Self::default_score as fn(usize, &[f32], &[usize]) -> f64));

        let mut best_action = 0;
        let mut best_score = f64::NEG_INFINITY;
        for a in 0..self.action_count {
            let s = scorer(a, state, opponent_history);
            if s > best_score {
                best_score = s;
                best_action = a;
            }
        }
        best_action
    }
}
",
        expect_lints: &["map_unwrap_or"],
        clean: false,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_ids_are_unique() {
        let all = fixtures();
        let mut ids: Vec<&str> = all.iter().map(|f| f.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate fixture ids");
    }

    #[test]
    fn every_fixture_parses_and_has_provenance() {
        for f in fixtures() {
            assert!(!f.provenance.is_empty(), "{}: no provenance", f.id);
            assert!(!f.code.trim().is_empty(), "{}: empty code", f.id);
            assert!(syn::parse_file(f.code).is_ok(), "{}: unparseable", f.id);
            if !f.clean {
                assert!(!f.expect_lints.is_empty(), "{}: no expected lints", f.id);
            }
        }
    }

    #[test]
    fn at_least_one_clean_fixture() {
        assert!(
            fixtures().iter().any(|f| f.clean),
            "decline axis needs >=1 clean fixture"
        );
    }

    #[test]
    fn provenance_split_is_reported() {
        let all = fixtures();
        let real = all
            .iter()
            .filter(|f| f.provenance.starts_with("katgpt-rs@"))
            .count();
        let authored = all
            .iter()
            .filter(|f| f.provenance.starts_with("authored-"))
            .count();
        assert!(real >= 5, "v1 needs >=5 real-code fixtures (have {real})");
        assert!(
            authored >= 20,
            "composite set expected >=20 (have {authored})"
        );
    }
}
