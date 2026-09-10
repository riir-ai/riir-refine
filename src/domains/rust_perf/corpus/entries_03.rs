//! Entries slice 03 of 06 — rules 53–75 of 126 — moved VERBATIM from the
//! former corpus.rs monolith (2026-09-10 split, scripted by
//! scripts/split_rust_perf_corpus.py). Append new batches to the
//! LAST slice or open a new one past ~1800 lines.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── allocation: collect() + append() → extend() ───────────────────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). Staging an iterator into an intermediate `Vec` only to
        // `append` it into the target pays one extra allocation + an extra
        // memcpy per chunk. `extend` writes the iterator's items straight
        // into the target's growth path — zero intermediates. The compiler
        // cannot elide a `collect` (it materializes a concrete `Vec`).
        PerfRule {
            rule_id: "collect-append-to-extend".to_string(),
            name: "collect() + append() staging → extend() (no intermediate Vec)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in chunks {\n        let mut staged: Vec<u32> = c.iter().copied().collect(); // intermediate Vec alloc per chunk\n        target.append(&mut staged);\n    }\n}".to_string(),
            fixed_code: "fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {\n    for c in chunks {\n        target.extend(c.iter().copied()); // writes straight into the target\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(collect_append_to_extend_bench_spec()),
        },
        // ── complexity: HashSet contains+insert → insert's bool return ────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). `HashSet` has no `entry` API (unlike `HashMap` — see the
        // `entry-vs-contains-insert` sibling); the membership check IS the
        // insert's `bool` return. `!set.contains(&x)` + `set.insert(x)` walks
        // the hash table twice; `set.insert(x)` alone walks it once. The two
        // calls are opaque to LLVM (hashbrown internals) — no fusion.
        PerfRule {
            rule_id: "hashset-insert-bool-return".to_string(),
            name: "HashSet contains+insert double lookup → insert's bool return (one lookup)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashSet;\nfn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    for &x in items {\n        if !seen.contains(&x) {\n            seen.insert(x);\n            inserted += 1;\n        }\n    }\n    inserted\n}".to_string(),
            fixed_code: "use std::collections::HashSet;\nfn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {\n    let mut inserted = 0;\n    for &x in items {\n        if seen.insert(x) {\n            inserted += 1;\n        }\n    }\n    inserted\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hashset_insert_bool_return_bench_spec()),
        },
        // ── allocation: unwrap_or(eager) → unwrap_or_else(closure) ────────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). The `Option` sibling of `ok-or-else`: `unwrap_or(expr)`
        // evaluates `expr` BEFORE checking the `Some` variant — the default
        // is computed (and allocated, if it allocates) even when discarded.
        // `unwrap_or_else(|| expr)` defers construction to the `None` arm.
        // Wins when the default is non-trivial AND `Some` is the common case.
        PerfRule {
            rule_id: "unwrap-or-else-lazy".to_string(),
            name: "unwrap_or(expensive()) eager default → unwrap_or_else(closure) lazy".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn resolve(opt: Option<u32>) -> u32 {\n    opt.unwrap_or(expensive_default(64)) // eagerly evaluated even when Some\n}\nfn expensive_default(n: u32) -> u32 {\n    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 31))\n}".to_string(),
            fixed_code: "fn resolve(opt: Option<u32>) -> u32 {\n    opt.unwrap_or_else(|| expensive_default(64)) // built only on None\n}\nfn expensive_default(n: u32) -> u32 {\n    (0..n).fold(0u32, |acc, i| acc.wrapping_add(i * 31))\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(unwrap_or_else_bench_spec()),
        },
        // ── iterators: filter(p).count() > 0 → any(p) short-circuit ───────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). `filter(p).count() > 0` scans the WHOLE iterator, builds a
        // count, then throws it away. `any(p)` stops at the first hit. The
        // compiler cannot derive a short-circuit from a full `count` fold.
        // The win scales with how early the first match lands.
        PerfRule {
            rule_id: "filter-count-to-any".to_string(),
            name: "filter(p).count() > 0 full scan → any(p) short-circuit".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn has_even(vals: &[u32]) -> bool {\n    vals.iter().filter(|&&v| v % 2 == 0).count() > 0 // full scan, no short-circuit\n}".to_string(),
            fixed_code: "fn has_even(vals: &[u32]) -> bool {\n    vals.iter().any(|&v| v % 2 == 0) // stops at the first hit\n}".to_string(),
            // Bench 48: ParetoDominated at mid-position hits (0.82-0.90×) —
            // a genuine folk-wisdom refutation. LLVM VECTORIZES the count
            // loop (batched element scanning) while any's early-exit forces
            // scalar iteration; at a 50% hit position the vectorized FULL
            // count beats the scalar half-scan. any wins only for very early
            // hits (empirically ≤ ~10% position) or non-vectorizable
            // predicates. Keep as an idiom rule with the regime caveat.
            context_notes: Some(
                "Bench 48: ParetoDominated 0.82-0.90x at mid-position hits. \
                 LLVM vectorizes filter().count() into batched scanning while \
                 any()'s early-exit forces scalar iteration — at a 50% hit \
                 position the vectorized full count WINS. any() only wins for \
                 very early hits (~<10% position) or non-vectorizable \
                 predicates. Regime-dependent: suggest only when hits are \
                 known-early or the predicate is expensive/branchy."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_count_to_any_bench_spec()),
        },
        // ── allocation: Vec::new + resize(n, 0) → vec![0; n] calloc path ──────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). `Vec::new()` + `resize(n, 0)` allocates uninitialized
        // memory then runs an explicit memset pass over every byte. `vec![0; n]`
        // routes through `alloc_zeroed` (calloc) — for large buffers the
        // allocator hands back lazily-zeroed OS pages, skipping the touch.
        PerfRule {
            rule_id: "vec-macro-zero-fill".to_string(),
            name: "Vec::new() + resize(n, 0) memset → vec![0; n] alloc_zeroed (calloc)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn zero_buf(n: usize) -> Vec<u8> {\n    let mut v = Vec::new(); // uninit alloc\n    v.resize(n, 0); // explicit memset pass\n    v\n}".to_string(),
            fixed_code: "fn zero_buf(n: usize) -> Vec<u8> {\n    vec![0u8; n] // alloc_zeroed — calloc zero pages\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_macro_zero_fill_bench_spec()),
        },
        // ── iterators: .map(|x| *x) → .copied() specialized adapter ───────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). `.map(|x| *x)` wraps the iterator in the generic `Map`
        // adapter (opaque closure, no size hints). `.copied()` is a
        // specialized adapter implementing `TrustedLen` + `InPlaceIterable`
        // markers — downstream consumers (collect, sum) get exact-size
        // reservations the `Map` form cannot express. In tight release loops
        // LLVM often lowers both identically — the win regime is adapter
        // fusion + allocation sizing, which the bench measures honestly.
        PerfRule {
            rule_id: "iter-copied-deref-map".to_string(),
            name: ".map(|x| *x) generic adapter → .copied() specialized (TrustedLen)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn sum_kills(kills: &[u64]) -> u64 {\n    kills.iter().map(|k| *k).sum() // generic Map adapter, no size hint\n}".to_string(),
            fixed_code: "fn sum_kills(kills: &[u64]) -> u64 {\n    kills.iter().copied().sum() // specialized Copy adapter\n}".to_string(),
            // Bench 48: Mixed — parity at ≤100K, 0.80× at 1M (copied SLOWER).
            // The specialized adapter's length-tracking overhead loses to the
            // fully-unrolled map+sum loop at large N on a sum fold. The rule
            // stands as an idiom (TrustedLen matters for collect's exact-size
            // reservations, not for folds); do not suggest for large-N folds.
            context_notes: Some(
                "Bench 48: Mixed (tie/tie/0.80x at 1K/100K/1M). The specialized \
                 Copied adapter carries length-tracking overhead that loses to \
                 the fully-unrolled Map+sum loop at large N on a sum fold. The \
                 idiom remains correct for collect() (exact-size reservation via \
                 TrustedLen) and for readability; do not suggest expecting a \
                 throughput win on large-N folds."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(iter_copied_deref_map_bench_spec()),
        },
        // ── iterators: filter(p).map(f) → filter_map fused adapter ────────────
        // Source: nnethercote/perf-book iterators chapter (Batch 48, idle-queue
        // item 11). The perf-book HEDGES on this one ("filter_map is not
        // always faster") — the fused form removes one adapter state-machine
        // layer but blocks some internal specializations of the separated
        // adapters. Genuinely regime-dependent; the Pareto bench decides the
        // corpus annotation, not the book's hedged prose.
        PerfRule {
            rule_id: "filter-map-fuse".to_string(),
            name: "filter(p).map(f) two adapters → filter_map fused (regime-dependent)".to_string(),
            category: "iterators".to_string(),
            buggy_code: "fn even_squares(vals: &[u32]) -> Vec<u32> {\n    vals.iter().filter(|&&v| v % 2 == 0).map(|&v| v * v).collect()\n}".to_string(),
            fixed_code: "fn even_squares(vals: &[u32]) -> Vec<u32> {\n    vals.iter().filter_map(|&v| (v % 2 == 0).then_some(v * v)).collect()\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(filter_map_fuse_bench_spec()),
        },
        // ── complexity: Vec::insert(0, x) loop → VecDeque::push_front ─────────
        // Source: nnethercote/perf-book std-types chapter (Batch 48, idle-queue
        // item 11). The write-side sibling of `vec-remove-front-to-vecdeque`:
        // every `insert(0, x)` memmoves the whole buffer down (O(n) per
        // insert, O(n²) per loop); `VecDeque::push_front` advances a ring
        // head in O(1). The compiler cannot turn a front-insert loop into a
        // ring buffer.
        PerfRule {
            rule_id: "vec-insert-front-deque".to_string(),
            name: "Vec::insert(0, x) O(n²) prepend loop → VecDeque::push_front O(1)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn prepend_history(history: &mut Vec<u32>, events: &[u32]) {\n    for &e in events {\n        history.insert(0, e); // O(n) memmove per insert → O(n²) total\n    }\n}".to_string(),
            fixed_code: "use std::collections::VecDeque;\nfn prepend_history(history: &mut VecDeque<u32>, events: &[u32]) {\n    for &e in events {\n        history.push_front(e); // O(1) ring-buffer head advance\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_insert_front_deque_bench_spec()),
        },
        // ── arithmetic: x.powf(2.0) constant exponent → x.powi(2) ──────────────
        // Source: users.rust-lang.org/t/56451 + baptiste-wicht.com std::pow C++
        // study (Batch 53, idle-queue item 11). `powf` lowers to a libm `pow`
        // call — an opaque scheduling barrier in a hot loop; `powi(2)` compiles
        // to a single multiply. The candidate risk (recorded honestly by the
        // bench, not assumed): LLVM's instcombine rewrites `pow(x, 2.0)` to
        // `x * x` in release — the verdict axis decides whether this is a
        // runtime win or an idiom-only rule.
        PerfRule {
            rule_id: "powi-vs-powf".to_string(),
            name: "x.powf(2.0) libm call → x.powi(2) single multiply (constant exponent)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn sum_squares(vals: &[f32]) -> f32 {\n    vals.iter().map(|v| v.powf(2.0)).sum() // libm pow call per element\n}".to_string(),
            fixed_code: "fn sum_squares(vals: &[f32]) -> f32 {\n    vals.iter().map(|v| v.powi(2)).sum() // single multiply\n}".to_string(),
            // Bench 53 (Bench 038): all-Tie at 1.00x across 1K/100K/1M —
            // LLVM instcombine lowers pow(x, 2.0) to the same x*x multiply
                       // as powi(2) in release, so the classic "powf is slow" advice
            // does NOT hold for the constant-exponent-2 release case. The
            // idiom still matters for: debug-profile hot loops (no instcombine
            // — powf stays a libm call), exponents >= 3 (LLVM does NOT lower
            // pow(x, 3.0) — correctly-rounded pow differs from x*x*x, so it
            // stays a libm call while powi(3) becomes two multiplies), and
            // non-constant exponents where powi is not applicable.
            context_notes: Some(
                "Bench 53: Mixed — all-Tie 1.00x (1K/100K/1M) — LLVM lowers powf(2.0) \
                 to the same x*x as powi(2) in release; the folk wisdom \
                 ('powf is extremely slow') does not hold for \
                 constant-exponent-2 release code. Suggest as an idiom for \
                 debug-profile hot loops (powf stays a libm call without \
                 instcombine) and exponents >= 3 (pow(x,3.0) is NOT lowered — \
                 it stays a libm call while powi(3) is two multiplies); do \
                 not suggest expecting a release-throughput win at exponent 2."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(powi_vs_powf_bench_spec()),
        },

        // ── arithmetic: x.powf(runtime_int as f64) libm call → x.powi(n) binary
        // exponentiation ── the NON-CONSTANT exponent case the powi-vs-powf
        // annotation explicitly deferred (Batch 77 / lucebox PR #625 rope.cu
        // `rope_theta_fp64`: `while (e) { if (e & 1) r *= base; base *= base;
        // e >>= 1; }` — the source measured the libcall DOMINATING the whole
        // RoPE kernel on RDNA4: 692 → 76 µs/launch @512 tokens, within 1 ulp;
        // negative exponents keep the libcall there because `e >>= 1` on a
        // negative int never reaches 0 — the TOTALLY guard is part of the
        // content. On CPU: powf with a runtime exponent is an opaque libm
        // call (no instcombine bailout — the exponent is not a compile-time
        // constant, so the constant-exponent Tie verdict does NOT apply);
        // powi(n) is log2(n) multiplies. NOT bit-identical: powi is not
        // correctly-rounded pow — the ported-numeric MANDATE applies as a
        // rel-error gate (max rel-error across a swept exponent range, ulp
        // bound documented honestly, the newton-refinement annotation
        // precedent).
        PerfRule {
            rule_id: "runtime-int-exponent-powi".to_string(),
            name: "x.powf(n as f64) libm call with runtime integer exponent → x.powi(n) binary exponentiation".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn theta_scale(base: f64, pos: &[i32], exp: i32) -> Vec<f64> {\n    pos.iter()\n        .map(|&p| base.powf((p * exp) as f64)) // opaque libm pow call\n        .collect() // no instcombine bailout: the exponent is RUNTIME\n}".to_string(),
            fixed_code: "fn theta_scale(base: f64, pos: &[i32], exp: i32) -> Vec<f64> {\n    pos.iter()\n        .map(|&p| base.powi(p * exp)) // log2 multiplies, no libm call\n        .collect() // Rust powi handles negative exponents (1/x^|n|)\n}".to_string(),
            context_notes: Some(
                "Bench 77: ParetoDominant 2.57x/2.81x/2.66x (1K/100K/1M, release, \
                 M3 aarch64; verdict-stable x2; 0 allocs both arms). CROSS-ARCH \
                 RE-GATE (2026-08-27 same day, 4090 box, x86_64-pc-windows-msvc, \
                 THIS harness unchanged, verdict-stable x2): all-Tie \
                 0.99x/1.00x/1.00x, 0/0 allocs, alloc-det stable — on x86-msvc \
                 BOTH lowerings are comparable calls (MSVC pow is fast; powi \
                 rides a similar-cost routine), so the 2.6x is M3/aarch64 (and \
                 GPU-slow-libm) regime, NOT universal: suggest by host — expect \
                 the win where the platform's libm pow is slow (aarch64/M3, GPU \
                 device libm, embedded); on fast-libm x86 hosts this is an idiom \
                 swap, not a speedup (the newton-refinement regime-inversion \
                 class, Batch 62). The GPU \
                 motivating evidence is the source's RDNA4 RoPE kernel (692 -> 76 \
                 µs/launch, 9.1x) — the CPU verdict is this bench's own. NOT \
                 bit-identical: powi(n) is binary exponentiation, not \
                 correctly-rounded pow — gate on max rel-error across a swept \
                 exponent range (the ported-numeric MANDATE; the \
                 newton-refinement precedent). The constant-exponent-2 Tie \
                 verdict of powi-vs-powf does NOT apply here: a runtime exponent \
                 gives LLVM no instcombine bailout, so powf stays a libm call in \
                 release. Prefer powf when the result must be correctly rounded \
                 (ULP-sensitive accumulation) — the multiplies accumulate \
                 rounding error ~log2(n) ulp.".to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(runtime_int_exponent_powi_bench_spec()),
        },
        // ── arithmetic: (dx²+dy²).sqrt() < r → dx²+dy² < r² ────────────────────
        // Source: the Box2D squared-distance convention (box2d.org SIMD
        // matters). sqrt is monotonic on non-negative inputs, so comparing
        // squared distances preserves ordering — the sqrt is pure waste when
        // only the COMPARISON is consumed. Guard: if the distance VALUE is
        // consumed downstream, keep the sqrt; NaN inputs order differently
        // under the two forms (both false in practice for <).
        PerfRule {
            rule_id: "distance-sq-no-sqrt".to_string(),
            name: "(dx*dx+dy*dy).sqrt() < r → dx*dx+dy*dy < r*r (compare squared)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    (dx * dx + dy * dy).sqrt() < r // sqrt only to compare\n}".to_string(),
            fixed_code: "fn in_range(dx: f32, dy: f32, r: f32) -> bool {\n    dx * dx + dy * dy < r * r // monotonic on non-negatives — ordering preserved\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(distance_sq_no_sqrt_bench_spec()),
        },
        // ── allocation: HashMap::new() + known-count loop → with_capacity ────
        // Source: users.rust-lang.org/t/44322 ("preallocate your Vecs and
        // HashMaps"). The HashMap sibling of `vec-with-capacity` but a
        // DIFFERENT cost class: growth rehashes — every resize re-runs the
        // hasher (SipHash) over ALL stored elements to place them in the new
        // bucket set, not just a realloc+memcpy. `with_capacity(n)` with a
        // known count pays one table alloc + zero rehash passes.
        PerfRule {
            rule_id: "hashmap-with-capacity".to_string(),
            name: "HashMap::new() + known-count insert loop → HashMap::with_capacity(n)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> {\n    let mut map = HashMap::new(); // starts at capacity ~3\n    for &it in items {\n        map.insert(it, it.wrapping_mul(31)); // rehash ladder: ~log2(n) resizes, each re-SipHashing all entries\n    }\n    map\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn index_items(items: &[u32]) -> HashMap<u32, u32> {\n    let mut map = HashMap::with_capacity(items.len()); // one table alloc, zero resizes\n    for &it in items {\n        map.insert(it, it.wrapping_mul(31));\n    }\n    map\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hashmap_with_capacity_bench_spec()),
        },
        // ── layout: AoS iteration over one hot field → SoA split ─────────────
        // Source: nnethercote/perf-book data-structure layout + the Box2D SIMD
        // post (Batch 53, idle-queue item 11). Iterating ONE field of a wide
        // AoS struct touches every cache line but consumes 4 of 32 bytes
        // (12.5% utilization) and defeats auto-vectorization (strided
        // gather). The SoA split makes the hot field contiguous — 16 floats
        // per cache line + a clean SIMD sum. The cold fields pay one pointer
        // indirection on THEIR (rare) access.
        PerfRule {
            rule_id: "soa-hot-field".to_string(),
            name: "AoS Vec<WideStruct> single-field scan → SoA split (hot field contiguous)".to_string(),
            category: "layout".to_string(),
            buggy_code: "struct Agent { x: f32, y: f32, vx: f32, vy: f32, hp: u32, level: u32, flags: u64, id: u64 } // 40 bytes\nfn total_x(agents: &[Agent]) -> f32 {\n    agents.iter().map(|a| a.x).sum() // 4 useful bytes per 40-byte struct — strided, scalar\n}".to_string(),
            fixed_code: "struct Agents { xs: Vec<f32>, ys: Vec<f32>, vxs: Vec<f32>, vys: Vec<f32>, hps: Vec<u32>, levels: Vec<u32>, flags: Vec<u64>, ids: Vec<u64> }\nfn total_x(agents: &Agents) -> f32 {\n    agents.xs.iter().sum() // contiguous — 16 floats/line, SIMD-able\n}".to_string(),
            // Bench 53 (Bench 038): Mixed 1.07x/tie/0.96x — a serial f32
            // add-reduction (~3-4 cycle dependency chain per element) dominates
            // BOTH sides; the 40-byte-stride load latency is fully hidden by
            // out-of-order execution, so the bandwidth+vectorization win never
            // materializes on a latency-bound scalar sum. SoA wins materialize
            // on THROUGHPUT-bound field consumers (count/filter over the field,
            // min/max, batch transform — vectorizable, no serial chain) or when
            // multiple hot fields are scanned together.
            context_notes: Some(
                "Bench 53: Mixed (1.07x/tie/0.96x at 1K/100K/1M) — a serial f32 \
                 add-reduction dominates both layouts; the strided-load penalty \
                 is hidden by OoO execution, so SoA buys nothing on \
                 latency-bound scalar sums. The layout win materializes on \
                 throughput-bound field consumers (count/filter, min/max, batch \
                 transform — vectorizable, no serial chain) or multi-field scans; \
                 suggest for those shapes, not for scalar sum folds."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(soa_hot_field_bench_spec()),
        },
        // ── arithmetic: loop-invariant RUNTIME divisor → magic multiplication ──
        // Source: libdivide (ridiculous_fish) via thermite's Rust port
        // (raygon-renderer/thermite `divider.rs` — Batch 59, the first
        // external-source distill via the `distill` skill). Hardware
        // integer division is 20-90+ cycles and NOT vectorizable; a
        // loop-INVARIANT runtime divisor (a function parameter — opaque to
        // LLVM, so no strength reduction) converts once into a magic
        // multiplier + shift (Granlund–Montgomery), then each division is
        // mulhi + shifts (~4 cycles, pipelined). Two-regime rule (thermite's
        // own split): CONSTANT divisors need nothing — LLVM already
        // strength-reduces them to multiplication at compile time; the win
        // is exclusively the RUNTIME-invariant case applied ≥~3 times
        // (setup pays one hardware div). Guard: d must be nonzero + the
        // same value for the whole batch (bucket counts, shard stripes,
        // hash-then-div layouts).
        PerfRule {
            rule_id: "division-by-invariant-multiplication".to_string(),
            name: "Loop of x / d with runtime-invariant d → precomputed magic multiplier + shift (libdivide)".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn bucket_sum(values: &[u32], d: u32) -> u32 {\n    let mut acc = 0;\n    for &x in values {\n        acc = acc.wrapping_add(x / d); // hardware udiv per element — d is a runtime invariant\n    }\n    acc\n}".to_string(),
            fixed_code: "struct FastDivU32 { magic: u32, shift: u32, add: bool }\nimpl FastDivU32 {\n    fn new(d: u32) -> Self { // one hardware div, once — amortized after ~3 divisions\n        let l = 31 - d.leading_zeros();\n        if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }\n        let k = 1u64 << l;\n        let base = (k << 32) / u64::from(d);\n        let rem = (k << 32) % u64::from(d);\n        if u64::from(d) - rem < k {\n            Self { magic: base as u32 + 1, shift: l, add: false }\n        } else {\n            let mut m = base * 2;\n            let rem2 = rem * 2;\n            if rem2 >= u64::from(d) { m += 1; }\n            Self { magic: (m + 1) as u32, shift: l, add: true } // (m+1) wraps mod 2^32 — intended (m spans (2^32, 2^33))\n        }\n    }\n    #[inline(always)]\n    fn div(&self, x: u32) -> u32 {\n        if self.magic == 0 { return x >> self.shift; } // power-of-two divisor\n        let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;\n        if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift } // widening add: u32 wrap drops the carry (d=7 x=MAX)\n    }\n}\nfn bucket_sum(values: &[u32], d: u32) -> u32 {\n    let fd = FastDivU32::new(d); // hoisted out of the loop\n    let mut acc = 0;\n    for &x in values {\n        acc = acc.wrapping_add(fd.div(x)); // mulhi + shift per element\n    }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(division_invariant_bench_spec()),
        },
        // ── dispatch: static-keyword HashMap lookup → plain match (LLVM length-switch) ──
        // Source: oxc "Pursuit of Performance on Building a JavaScript
        // Compiler" (oxc.rs/docs/learn/performance, rustmagazine issue-3;
        // Batch 61 online-search distill). oxc tried to beat a plain match on
        // string literals with a gperf-style perfect hash (V8's approach) and
        // COULD NOT — `--emit=llvm-ir` shows LLVM already lowers the match to
        // a switch on string LENGTH + one inline compare per length group.
        // The folk wisdom "string match is O(n) comparisons, use a hash map"
        // is wrong on modern LLVM: the HashMap pays SipHash per lookup while
        // the match pays a length switch + 1-2 byte compares. Bounded
        // fix-space: STATIC key sets only (a literal-initialized map);
        // dynamic keys keep the map.
        PerfRule {
            rule_id: "keyword-map-to-match".to_string(),
            name: "Static-keyword LazyLock<HashMap<&str, V>> dispatch → plain match (LLVM lowers to a length switch)".to_string(),
            category: "dispatch".to_string(),
            buggy_code: "use std::collections::HashMap;\nuse std::sync::LazyLock;\n#[derive(Clone, Copy)]\nenum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }\nstatic KEYWORDS: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {\n    let mut m = HashMap::new();\n    m.insert(\"as\", Kind::As);\n    m.insert(\"do\", Kind::Do);\n    m.insert(\"if\", Kind::If);\n    m.insert(\"let\", Kind::Let);\n    m.insert(\"fn\", Kind::Fn);\n    m.insert(\"while\", Kind::While);\n    m.insert(\"constructor\", Kind::Constructor);\n    m\n});\nfn match_keyword(s: &str) -> Kind {\n    *KEYWORDS.get(s).unwrap_or(&Kind::Ident) // SipHash per lookup\n}".to_string(),
            fixed_code: "#[derive(Clone, Copy)]\nenum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }\nfn match_keyword(s: &str) -> Kind {\n    // LLVM lowers string-literal matches to a switch on length +\n    // inline compares per length group (oxc verified via --emit=llvm-ir)\n    match s {\n        \"as\" => Kind::As,\n        \"do\" => Kind::Do,\n        \"if\" => Kind::If,\n        \"let\" => Kind::Let,\n        \"fn\" => Kind::Fn,\n        \"while\" => Kind::While,\n        \"constructor\" => Kind::Constructor,\n        _ => Kind::Ident,\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(keyword_map_to_match_bench_spec()),
        },
        // ── layout: usize index fields in high-volume structs → u32 ──────────
        // Source: oxc Span optimization (same article, Batch 61). The AST
        // leaf carried `struct Node { start: usize, end: usize }` — 16 bytes
        // — for byte offsets that cannot exceed u32 (< 4GB source). Narrowing
        // to u32 halved the node + measured up to +5% on large files (cache
        // density on the hottest leaf type). The mechanism is bandwidth:
        // half the bytes per element means twice the elements per cache
        // line. Guard: the domain must genuinely bound below 2^32 (byte
        // offsets, entity indexes, tick counters in long sessions need
        // audit). Type ripple is caller-wide — suggestion-quality rewrite,
        // no proposer (the Batch 58 signature-change lesson).
        PerfRule {
            rule_id: "usize-index-to-u32".to_string(),
            name: "usize index/offset fields in high-volume structs → u32 (halve cache footprint) RIDER (Batch 112 external confirmation, nnethercote 2026-07: rustc #158720 shrank AST expression nodes 72→64 B — \"small enough to fit in a cache line\" — >10% wall on AST-heavy benches + cache-miss −29%; the lineage from 104 B years ago is \"chip, chip, chipping\": shrink past the 64 B cache-line boundary is a step function, not a smooth curve)".to_string(),
            category: "layout".to_string(),
            buggy_code: "struct Span { start: usize, end: usize } // 16 bytes — pointer-sized for a < 4GB domain\nfn total_len(spans: &[Span]) -> u64 {\n    let mut acc = 0;\n    for s in spans { acc += (s.end - s.start) as u64; }\n    acc\n}".to_string(),
            fixed_code: "struct Span { start: u32, end: u32 } // 8 bytes — same domain, half the cache line traffic\nfn total_len(spans: &[Span]) -> u64 {\n    let mut acc = 0;\n    for s in spans { acc += u64::from(s.end - s.start); }\n    acc\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(usize_index_to_u32_bench_spec()),
        },
        // ── layout: hot counter arrays u64 "just in case" → u32 +
        // checked-promotion sidecar ──────────────────────────────────
        // Source: Datadog RTDB/Monocle (Batch 78, Research 092 — the
        // DDSketch aggregation buffer: "DDSketch bin counts are 64-bit
        // integers, but in practice most counts never approach the
        // 32-bit limit. We conserve memory by storing counts as u32
        // values in the aggregation buffer and promoting them to 64-bit
        // only if they overflow. Rust's built-in overflow checks make it
        // straightforward to detect when an upgrade to a larger counter
        // is needed."). The mechanism is the usize-index-to-u32 twin on
        // VALUES: half-width counters = 2x per cache line on the sweep.
        // DISTINCT from usize-index-to-u32 (Batch 61: narrows INDEX
        // fields whose domain bounds below 2^32 — a layout claim with a
        // domain guard); this narrows VALUE counters whose common case
        // stays low but whose domain does NOT bound them — the escape
        // hatch is runtime promotion (checked_add → sparse sidecar),
        // not a domain proof. The promoted slot's narrow cell zeroes
        // (dead) so the dense sweep stays branch-free; the sidecar is
        // sparse (only overflowing slots, an empty-HashMap fast path
        // guards the common regime). Datadog's compound (this + the
        // radix buffer + inline bins + shared buffer) measured +70%
        // aggregation throughput — the isolated mechanism is our bench.
        PerfRule {
            rule_id: "narrow-counter-overflow-promotion".to_string(),
            name: "hot counter arrays stored u64 \"just in case\" → u32 + checked-promotion sidecar (2x cache density)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Hot aggregation buffer: every counter is u64 \"just in case\" —\n// 8 bytes each, half the per-line density on the aggregation sweep.\nstruct Bins {\n    counts: Vec<u64>, // production data: most bins never approach 2^32\n}\nimpl Bins {\n    fn bump(&mut self, i: usize, v: u64) {\n        self.counts[i] += v; // full-width add for a counter that fits u32\n    }\n    fn total(&self) -> u64 {\n        self.counts.iter().sum() // sweep pays 2x the bytes per element\n    }\n}".to_string(),
            fixed_code: "// Same buffer, common case narrowed to u32 + a sparse promoted sidecar:\n// 2x counters per cache line on the sweep; Rust's checked_add IS the\n// overflow detector (the promote path moves the slot to the sidecar and\n// zeroes the narrow slot — totals stay exact).\nstruct Bins {\n    counts: Vec<u32>,                      // narrow fast path (dense sweep)\n    wide: std::collections::HashMap<usize, u64>, // promoted slots only (sparse)\n}\nimpl Bins {\n    fn bump(&mut self, i: usize, v: u32) {\n        if self.wide.is_empty() {\n            // common regime: nothing ever promoted — narrow add only\n            match self.counts[i].checked_add(v) {\n                Some(x) => self.counts[i] = x,\n                None => {\n                    let w = u64::from(self.counts[i]) + u64::from(v);\n                    self.counts[i] = 0; // narrow slot is dead now\n                    self.wide.insert(i, w);\n                }\n            }\n            return;\n        }\n        if let Some(w) = self.wide.get_mut(&i) {\n            *w += u64::from(v); // already promoted — stay wide\n            return;\n        }\n        match self.counts[i].checked_add(v) {\n            Some(x) => self.counts[i] = x,\n            None => {\n                let w = u64::from(self.counts[i]) + u64::from(v);\n                self.counts[i] = 0;\n                self.wide.insert(i, w);\n            }\n        }\n    }\n    fn total(&self) -> u64 {\n        // dense narrow sweep (2x density) + the tiny promoted patch\n        self.counts.iter().map(|&n| u64::from(n)).sum::<u64>()\n            + self.wide.values().sum::<u64>()\n    }\n}".to_string(),
            context_notes: Some(
                "MEASURED ParetoDominated 0.64-0.66x at 100K/1M/4M counters (M3 Max release, 2026-08-27; verdict-stable x2; alloc axis deterministic: buggy 1 alloc of 8N bytes vs fixed 1 alloc of 4N — the halving is exact). THE TRADE, honestly: the win is MEMORY (2x per cache line / half the footprint — Datadog's motivation was the 2,100 GiB infeasibility of per-worker DDSketch buffers, not speed), NOT wall-clock on update-heavy shapes: the per-update wide.is_empty() guard + checked_add overflow branch cost ~35-55% on a 4-updates-per-slot round-robin stream, and the flat ratio across sizes shows the workload is branch-bound, not bandwidth-bound (a bandwidth win would grow with the working set). REGIME where the trade PAYS: memory-footprint-bound systems (huge counter arrays, RAM/GB-priced deployment) and sweep-dominated access (read-mostly, few updates — the sweep alone touches half the bytes); the cost regime is update-heavy streams. Datadog's +70% aggregation throughput was the COMPOUND of the shared buffer + radix tree + inline bins + this narrowing — never cite it as the isolated mechanism. The promotion semantics are exact: totals match u64 bit-identically on every stream incl. u32-boundary crossings (the exhaustive-style G1 gate below). Distinct from usize-index-to-u32 (index fields, domain-bounded) — this narrows VALUE counters with a runtime escape hatch. Suggestion-only (type-blind: a parse-only proposer cannot see counter magnitude distributions). Source: Datadog RTDB/Monocle (datadoghq.com blog, 2025-08-04, Batch 78 / Research 092)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(narrow_counter_overflow_promotion_bench_spec()),
        },
        // ── layout: by-value moves of >128-byte types lower to memcpy CALLS ──
        // Source: nnethercote July-2026 rustc-perf post (rust-lang/rust
        // #160005, Research 134, Batch 112): LLVM lowers any by-value move
        // of a >128-byte type to a `memcpy` call instead of inline
        // load/store pairs — a hot 136-byte type paid ~half its compile
        // time in memcpy; the fix shrank the type to 104 bytes so the
        // remaining moves inlined (plus avoiding 2/3 of the moves outright)
        // → 40% wall on the two worst crates. DHAT copy-profiling was the
        // finding instrument. The 128 B threshold is an LLVM BACKEND
        // CONSTANT (`MaxStoresPerMemcpy` family) and it is ARCH-SHAPED:
        // x86-64's inline ceiling is 128 B (8 stores = 4 pairs x 32 B) so
        // 136 B → memcpy CALL; aarch64's ceiling is HIGHER — the corpus
        // gate's asm reading shows a 136-byte move inlining to ldp/stp
        // pairs (4 pairs + an 8 B tail) on Apple silicon with current
        // rustc, while a 320-byte move on the same box emits `bl _memcpy`
        // — the ARM boundary sits in (136, 320]. The M3 bench therefore
        // ties at 136 B (both arms inline); the x86 re-gate (4090,
        // same day) then showed the step DIRECTLY: the 48-byte copy
        // inlines while the 136-byte loop carries `movl $136;
        // callq memcpy` per iteration. Fix forms: (1) pass by reference where the
        // consumer only reads; (2) shrink the type under the threshold
        // (repack, usize-index-to-u32 composition, move the cold tail
        // out-of-line); (3) avoid the move (in-place construction —
        // architectural). Distinct from `box-large-enum-variant` (variant
        // packing at the ENUM level), `async-future-size` (state-machine
        // footprint, no move-cost axis), `soa-hot-field` (single-field scan
        // contiguity) — the mechanism here is the MOVE COST of the whole
        // type crossing the inline-memcpy boundary, found by copy-profiling
        // not by field scans. Suggestion-only (a parse-only proposer cannot
        // see type sizes or hot-path move frequency).
        PerfRule {
            rule_id: "hot-byval-move-over-memcpy-threshold".to_string(),
            name: "hot by-value moves of a >128-byte struct → shrink under the memcpy threshold / pass by reference (LLVM lowers big moves to memcpy CALLS)".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Hot path: every call moves the 136-byte record BY VALUE — LLVM lowers\n// each move to a memcpy CALL (not inline stores) once the type crosses the\n// 128-byte inline threshold (x86-64 `MaxStoresPerMemcpy` = 128 B; aarch64's\n// ceiling is lower still). Copy-profiling (DHAT) is how you find this.\nstruct Record {\n    hot: [u64; 13], // 104 B — the fields the hot path actually consumes\n    cold: [u64; 4], // 32 B — carried along in EVERY move\n}\n\nfn score(r: Record) -> u64 {\n    let mut a = 0u64;\n    for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }\n    for i in 0..4 { a = a.wrapping_add(r.cold[i].wrapping_mul(3)); }\n    a\n}\n\nfn run(mut state: Record, n: usize) -> u64 {\n    let mut acc = 0u64;\n    for i in 0..n {\n        acc ^= score(state); // 136-byte memcpy per call\n        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);\n        state.cold[i & 3] = state.cold[i & 3].rotate_left(1);\n    }\n    acc\n}".to_string(),
            fixed_code: "// Fix (composed forms): shrink the moved type under the threshold by\n// moving the cold tail OUT of the hot record (by reference when read-only,\n// in-place when mutated) — the 104-byte moves now inline as load/store\n// pairs (on x86-64), and the cold fields are never copied at all.\nstruct Record {\n    hot: [u64; 13], // 104 B — under the threshold\n}\n\nfn score(r: Record, cold: &[u64; 4]) -> u64 {\n    let mut a = 0u64;\n    for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }\n    for i in 0..4 { a = a.wrapping_add(cold[i].wrapping_mul(3)); }\n    a\n}\n\nfn run(mut state: Record, cold: &mut [u64; 4], n: usize) -> u64 {\n    let mut acc = 0u64;\n    for i in 0..n {\n        acc ^= score(state, cold); // 104-byte move: inline stores\n        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);\n        cold[i & 3] = cold[i & 3].rotate_left(1); // cold tail mutated in place — zero copies\n    }\n    acc\n}".to_string(),
            context_notes: Some(
                "MEASURED Mixed 1/0/1 at 1K/1M iterations (M3 Max release, 2026-09-05, verdict-stable x2: 1.29x/0.99x then 1.29x/0.99x; allocs 0/0 both arms, alloc-det stable). The tie at n=1M is the honest mechanism cell, not a bench bug: the threshold step this rule encodes is ARCH-SHAPED and the M3 sits on the no-step side at 136 B. Asm evidence (the corpus gate reads the -C emit-asm output): on aarch64 + current rustc a 136-byte by-value move inlines to ldp/stp pairs (4 pairs + an 8 B tail) - NO memcpy call - while a 320-byte move on the same box emits bl _memcpy, so the aarch64 inline ceiling sits in (136, 320]; x86-64's ceiling is 128 B (nnethercote: rust-lang/rust #160005, July 2026 - any by-value move of a >128-byte type lowers to a memcpy CALL; a hot 136-byte type paid ~half its compile time in memcpy until shrunk to 104 B, 40% wall on the two worst crates; DHAT copy-profiling was the finding instrument). REGIME: a measured x86 win at the 128 B boundary; an ARM win only at larger types (once the move crosses the ARM ceiling - between 136 and 320 B on this toolchain - the call appears there too and the same shrink fixes it). The n=1K 1.29x cell is warm-up-dominated (per-iter cost drops 2x from n=1K to n=1M in BOTH arms); never cite it as the mechanism. The fix doctrine is arch-independent and never-negative (fewer/smaller by-value moves; profile with DHAT-class copy counters, not field scans). Composed-fix disclosure: the bench's fixed arm = shrink (136 -> 104) + the 32 B cold tail by-reference/in-place - both rule fix-forms; a binary bench cannot separate them. x86 RE-GATED 2026-09-05 (4090 box, quiet, rustc 1.93 x86_64-pc-windows-msvc): the asm shows the step DIRECTLY - the 48-byte loop's copy has NO memcpy call while the 136-byte loop's hot loop carries `movl $136; callq memcpy` per iteration, and the probe's wall clock moves the same direction (136 B loop = 1.32-1.35x the 48 B loop per iteration on x86 vs 1.00-1.13x on the M3). Distinct from box-large-enum-variant (enum-variant packing), async-future-size (state-machine footprint), soa-hot-field (field-scan contiguity) - the mechanism here is the MOVE COST of the whole type at a call boundary. Suggestion-only (a parse-only proposer cannot see type sizes or hot-path move frequency)."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(hot_byval_move_memcpy_threshold_bench_spec()),
        },
        // ── simd: transposed-weight register GEMV ─────────────────────────────────────
        // Source: microGPT-C (MIT, Research 070, Batch 62). The row-major
        // matvec dots every row into ONE scalar accumulator and pays a
        // horizontal reduction per row (the in-tree `binary.rs`
        // `vaddvq_f32(acc0)` shape); packing weights transposed once at
        // weight-freeze lets each input element broadcast and FMA into
        // output-vector accumulators — zero horizontal reduction, sequential
        // weight streaming, output registers held across the column loop
        // (the ryg 2015 transposed-lane Mv analysis). Distinct from the
        // kernel_opt GPU plane-per-row family (`workgroup-equals-plane-
        // native-reduction`, `single-plane-sufficient-reduction`): those are
        // GPU workgroup/shuffle concerns — this is the CPU SIMD register
        // twin. Architectural (weight storage packs at model-load; per-call
        // `&[f32]` params cannot pack in place) — suggestion-quality, no
        // proposer (the `soa-hot-field` class). Tail tiles 16/8/4 rows keep
        // lanes full at non-multiple nout.
        PerfRule {
            rule_id: "transposed-weight-register-gemv".to_string(),
            name: "Row-major per-row dot + horizontal reduce → transposed weights, per-input broadcast FMA (register GEMV)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// Row-major GEMV: each row is a dot product into ONE scalar\nfn matvec(w: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n    for r in 0..nout {\n        let row = &w[r * nin..(r + 1) * nin];\n        let mut acc = 0.0f32;\n        for c in 0..nin {\n            acc += row[c] * x[c]; // horizontal reduce at the end of EVERY row\n        }\n        y[r] = acc;\n    }\n}".to_string(),
            fixed_code: "// Transposed (column-major) weights: each input element x[c] broadcasts\n// once and FMAs into the whole output vector — lane-parallel, ZERO\n// horizontal reduction anywhere. Pack weights transposed ONCE at\n// weight-freeze; the pack cost amortizes over every call.\nfn matvec(w_t: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n    for r in 0..nout { y[r] = 0.0; }\n    for c in 0..nin {\n        let xc = x[c]; // one load, nout lane FMAs\n        let col = &w_t[c * nout..(c + 1) * nout];\n        for r in 0..nout {\n            y[r] += xc * col[r]; // auto-vectorizes: no per-row reduce\n        }\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(transposed_weight_register_gemv_bench_spec()),
        },
        // ── arithmetic: Newton refinement loses to hardware on CPU ───────────
        // Source: microGPT-C PERFORMANCE.md dead-ends (Research 070,
        // Batch 62) — the honest-annotation class (`filter-count-to-any`
        // precedent): the rule exists to STOP a plausible-looking GPU→CPU
        // port. On GPU, div/sqrt are multi-cycle → approximate + refine
        // wins; on CPU they are single-instruction + correctly rounded →
        // every Newton step is pure extra work (source measured 2-4%
        // slower on Ryzen 5 5600H). Distinct from
        // `regularizer-above-noise-floor-coupled-to-damping-cap` (that is
        // Newton-Schulz ITERATION robustness — different concern). The
        // relaxed-precision-estimate axis (`rsqrtss` 12-bit / `frecpe`) is
        // a SEPARATE valid optimization where accuracy can relax — never
        // suggest replacing precision-required divs.
        PerfRule {
            rule_id: "newton-refinement-vs-hardware-transcendental".to_string(),
            name: "Newton-refined recip/rsqrt (the GPU fast-math recipe) → hardware single-instruction div/sqrt on CPU".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "// GPU-ported fast-math recip + rsqrt: seed + Newton refinement steps.\n// On GPU this wins (div/sqrt are multi-cycle → approx + refine);\n// on CPU the hardware op is single-instruction + correctly rounded,\n// so every refinement step is pure extra work.\nfn fast_inv(x: f32) -> f32 {\n    let r = 1.0 / x; // seed (estimate-tier on GPU)\n    let r = r * (2.0 - x * r); // Newton step 1\n    r * (2.0 - x * r) // Newton step 2\n}\nfn fast_rsqrt(x: f32) -> f32 {\n    let r = 1.0 / x.sqrt();\n    r * (0.5 * (3.0 - x * r * r)) // one Newton refinement\n}".to_string(),
            fixed_code: "// On CPU, `fdiv`/`fsqrt` are single-instruction and correctly rounded\n// — the refinement chain is pure extra work (measured 2-4% slower\n// than plain division on modern x86/ARM). Use the hardware op directly.\nfn fast_inv(x: f32) -> f32 { 1.0 / x }\nfn fast_rsqrt(x: f32) -> f32 { 1.0 / x.sqrt() }".to_string(),
            context_notes: Some(
                "Mixed (Batch 62 bench, M3 Max): hardware `fdiv`/`fsqrt` never \
                 lose to the refinement chain — 1.09x at 1K, 1.01x at 100K \
                 (source measured 2-4% on Ryzen 5 5600H). On CPU div/sqrt are \
                 single-instruction + correctly rounded, so every Newton step is \
                 pure extra work; the verdict INVERTS on GPU (multi-cycle \
                 div/sqrt → approximate+refine wins). Suggest the hardware op on \
                 CPU hot paths; never port GPU fast-math refinement chains. The \
                 relaxed-precision-estimate axis (12-bit estimate intrinsics) is a \
                 separate optimization valid ONLY where accuracy can relax — not \
                 for precision-required divs. x86 re-gate (2026-08-19, 4090 box, quiet): never-loses is CROSS-ARCH and the win is LARGER there — ParetoDominant 1.64x/1.66x at 1K/100K; the near-tie is M3-aarch64-specific (fdiv latency, pipeline-hidden at 100K)."
                .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(newton_refinement_vs_hardware_bench_spec()),
        },
        // ── simd: loop-order-innermost-contiguous ──
        // Source: Arun Nagaraj, 'Matrix Multiplication in Rust with
        // SIMD: From Naive to 126x Faster' (medium @aruntamiln
        // 2026-02-12; repo ArunTamil21/matmat-simd typo ArunTamil21/matmul-simd;
        // x86 AVX measured, technique ISA-independent — it is
        // memory-hierarchy, not ISA). The i-j-k nest's INNERMOST loop
        // (k) strides `b`'s OUTER axis (`b[k][j]` at stride-n) — every
        // access a new cache line, L1 misses 45%->12% after the reorder.
        // Distinct from `transposed-weight-register-gemv` (B62: repacks
        // the WEIGHTS column-major for GEMV — that changes the data
        // layout; this reorders the LOOPS over an unchanged layout, the
        // no-repack general nested-accumulation case), `simd-chunk`
        // (chunking for vectorization width), `layout-reorder` (struct
        // field padding), `soa-hot-field` (AoS->SoA). Legality: no
        // loop-carried deps across the swapped axes (the `c[i][j] +=`
        // accumulation shape is the canonical legal case). Float
        // sum-order changes under the reorder — rel-error tolerance
        // gate, NOT bit-identity (the `distance-sq-no-sqrt` precedent).
        PerfRule {
            rule_id: "loop-order-innermost-contiguous".to_string(),
            name: "i-j-k nested accumulation with a column-striding inner access → i-k-j reorder (innermost varies the LAST index)".to_string(),
            category: "simd".to_string(),
            buggy_code: "// i-j-k nest: the INNERMOST loop (k) walks b[k][j] COLUMN-wise at stride-n\nfn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n    for i in 0..n {\n        for j in 0..n {\n            for k in 0..n {\n                c[i * n + j] += a[i * n + k] * b[k * n + j]; // stride-n inner\n            }\n        }\n    }\n}".to_string(),
            fixed_code: "// i-k-j reorder: the innermost loop (j) varies the LAST index — a\n// CONTIGUOUS row walk through b, c, and the hoisted a row anchor.\nfn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n    for i in 0..n {\n        let a_row = &a[i * n..(i + 1) * n]; // hoist the row anchor\n        for k in 0..n {\n            let a_ik = a_row[k];\n            for j in 0..n {\n                c[i * n + j] += a_ik * b[k * n + j]; // j strides by 1\n            }\n        }\n    }\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(loop_order_innermost_contiguous_bench_spec()),
        },
        // ── complexity: debug-assert-length-hoist ──
        // Source: Shnatsel, 'How to avoid bounds checks in Rust
        // (without unsafe!)' (2023-01-17, the canonical article;
        // repo Shnatsel/bounds-check-cookbook). A `debug_assert!`
        // mentioning `.len()` is COMPILED OUT in release — dropping
        // exactly the length constraint the optimizer needed to
        // eliminate the subsequent bounds checks, so release retains
        // ALL of them ('Ironically, this makes the code slower'). The
        // fix is the macro swap: `assert!` survives release, and
        // redundant asserts are optimizer-removed ('you cannot have
        // too many'). The counter-intuition is the value: `debug_assert!`
        // FEELS free (it vanishes!) but its vanishing is the pessimization.
        // Sibling fact encoded: an `assert!` in front of the hot loop is
        // the cheapest bounds-check hoist (check once, not per-iteration).
        // Honest expectation (annotation): aarch64 gains are the 1.03x
        // class (the 15% headline was an x86 unrolling secondary
        // effect) — the rule exists to stop the release-mode
        // pessimization, not to promise throughput.
        PerfRule {
            rule_id: "debug-assert-length-hoist".to_string(),
            name: "debug_assert! mentioning .len() is erased in release (dropping the constraint bounds-check elision needed) → assert!".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// debug_assert! is ERASED in release — the optimizer loses exactly the\n// range constraint it needed to eliminate the loop's bounds checks\nfn checksum(rows: &[(u64, u64)], first: usize, count: usize) -> u64 {\n    debug_assert!(first + count <= rows.len());\n    let mut acc = 0;\n    for i in first..first + count {\n        acc ^= rows[i].0.wrapping_add(rows[i].1); // release keeps the bounds check\n    }\n    acc\n}".to_string(),
            fixed_code: "// assert! SURVIVES release: the range constraint stays visible and the\n// bounds checks are elided (\"you cannot have too many asserts\" — the\n// redundant ones are optimizer-removed, never re-checked)\nfn checksum(rows: &[(u64, u64)], first: usize, count: usize) -> u64 {\n    assert!(first + count <= rows.len());\n    let mut acc = 0;\n    for i in first..first + count {\n        acc ^= rows[i].0.wrapping_add(rows[i].1); // checks elided in release\n    }\n    acc\n}".to_string(),
            context_notes: Some(
                "Mixed, regime-split (Batch 64 bench, M3 Max aarch64): the \
                 canonical range-pinned shape measures 1.13x at n=1K \
                 (elision win) but a STABLE 0.67-0.71x LOSS at n=100K \
                 (3-run verified) — on aarch64 the swap is shape- and \
                 scale-dependent, NOT a free win; the article's own aarch64 \
                 runs measured 1.03x (the 15% headline was an x86 unrolling \
                 secondary effect). Scope: the assert's constraint must be \
                 PROPAGATABLE to the indexed accesses (parameter/range-pinned \
                 indices) — a `debug_assert!` over a DATA-DEPENDENT index \
                 array can never enable elision (the compiler cannot \
                 propagate through runtime index values), so the swap there \
                 is pure cost. The rule's VALUE is stopping the \
                 counter-intuitive release-mode pessimization \
                 (`debug_assert!` feels free — its vanishing drops the \
                 length constraint bounds-check elision needed) while warning \
                 that the fix itself needs measuring per shape. Suggest \
                 `assert!` (redundant asserts are optimizer-removed) on \
                 range-pinned shapes; MEASURE before expecting throughput. x86 re-gate (2026-08-19, 4090 box, quiet): regime CONFIRMED cross-arch — 1.00x tie at 1K, 0.92x at 100K (milder than aarch64's 0.67-0.71x but still no free win; the article's x86 headline does not reproduce on this harness shape either).".to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(debug_assert_length_hoist_bench_spec()),
        },
        // ── arithmetic: sparse set-bit iteration — trailing_zeros + BLSR ──
        // Source: polars kernel idioms (Batch 66, Research 073 —
        // polars-compute/src/filter/boolean.rs + polars-arrow bitmap
        // TrueIdxIter). The buggy shape tests ALL 64 bit positions with a
        // shift+mask each; the fixed loop consumes the least significant
        // set bit per iteration (trailing_zeros + `m &= m.wrapping_sub(1)`
        // — the BLSR instruction where available) so it runs popcount(m)
        // iterations, not 64 — a ~16x iteration cut at 4/64 density. The
        // compiler cannot derive the tz-loop from the shift-test loop (the
        // trip count is data-dependent). Both forms emit identical indices.
        //
        // ANTI-SHAPE (Batch 68 rider, Research 075 — tantivy
        // common/src/bitset.rs `TinySet::pop_lowest`): code that ALREADY
        // iterates by trailing_zeros but clears with `x ^= 1 << lowest` is
        // ALSO this rule's territory — the xor-clear SERIALIZES the count
        // and the clear (the shift needs the tz result: ARM64
        // rbit→clz→lsl→eor, x86 tzcnt→btc), while `x &= x - 1` (Kernighan)
        // makes the clear INDEPENDENT of the count (ARM64 sub→and, x86
        // blsr — tzcnt runs in parallel; tantivy's in-source godbolt:
        // godbolt.org/z/fnfrP1T5f). Same indices, same popcount trips —
        // only the dependency chain changes.
        PerfRule {
            rule_id: "sparse-set-bit-iteration-tz-blsr".to_string(),
            name: "64-iteration set-bit scan → trailing_zeros + BLSR loop".to_string(),
            category: "arithmetic".to_string(),
            buggy_code: "fn sum_set_positions(mask: u64) -> u32 {\n    let mut sum = 0;\n    for i in 0..64 {\n        if (mask >> i) & 1 != 0 {\n            sum += i as u32;\n        }\n    }\n    sum\n}".to_string(),
            fixed_code: "fn sum_set_positions(mask: u64) -> u32 {\n    let mut sum = 0;\n    let mut m = mask;\n    while m != 0 {\n        let i = m.trailing_zeros(); // TZCNT\n        sum += i;\n        m &= m.wrapping_sub(1); // BLSR: clear least significant set bit\n    }\n    sum\n}".to_string(),
            context_notes: None,
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sparse_set_bit_iteration_bench_spec()),
        },
    ]
}

/// Bench spec for `collect-append-to-extend`: collect+append staging vs extend.
#[cfg(feature = "rust_perf_bench")]
fn collect_append_to_extend_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    // One intermediate Vec alloc + memcpy per chunk.
    pub fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {
        for c in chunks {
            let mut staged: Vec<u32> = c.iter().copied().collect();
            target.append(&mut staged);
        }
    }
}

mod fixed {
    // Straight into the target's growth path — zero intermediates.
    pub fn merge_chunks(target: &mut Vec<u32>, chunks: &[Vec<u32>]) {
        for c in chunks {
            target.extend(c.iter().copied());
        }
    }
}

pub fn make_input(n: usize) -> Vec<Vec<u32>> {
    // Chunks of 16 — a realistic batch shape (both sides do the same work,
    // the difference is purely the intermediate staging).
    (0..n / 16).map(|b| (0..16).map(|i| (b * 16 + i) as u32).collect()).collect()
}

pub fn run_buggy(input: &[Vec<u32>]) {
    let mut target: Vec<u32> = Vec::with_capacity(input.len() * 16);
    buggy::merge_chunks(&mut target, input);
    std::hint::black_box(target.len());
}

pub fn run_fixed(input: &[Vec<u32>]) {
    let mut target: Vec<u32> = Vec::with_capacity(input.len() * 16);
    fixed::merge_chunks(&mut target, input);
    std::hint::black_box(target.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `hashset-insert-bool-return`: contains+insert double lookup
/// vs insert's bool return. Half-duplicate input so both bool arms fire.
#[cfg(feature = "rust_perf_bench")]
fn hashset_insert_bool_return_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
use std::collections::HashSet;

mod buggy {
    use std::collections::HashSet;
    // Two hash-table walks per element.
    pub fn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {
        let mut inserted = 0;
        for &x in items {
            if !seen.contains(&x) {
                seen.insert(x);
                inserted += 1;
            }
        }
        inserted
    }
}

mod fixed {
    use std::collections::HashSet;
    // ONE hash-table walk per element.
    pub fn register_seen(seen: &mut HashSet<u32>, items: &[u32]) -> usize {
        let mut inserted = 0;
        for &x in items {
            if seen.insert(x) {
                inserted += 1;
            }
        }
        inserted
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // 50% duplicates: every element's membership arm fires both ways.
    (0..n).map(|i| (i % (n / 2).max(1)) as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut seen = HashSet::with_capacity(input.len());
    std::hint::black_box(buggy::register_seen(&mut seen, input));
}

pub fn run_fixed(input: &[u32]) {
    let mut seen = HashSet::with_capacity(input.len());
    std::hint::black_box(fixed::register_seen(&mut seen, input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `unwrap-or-else-lazy`: eager default construction vs lazy
/// closure, 99% Some (the regime where the eager default is pure waste).
/// Mirrors the `ok-or-else` harness shape.
#[cfg(feature = "rust_perf_bench")]
fn unwrap_or_else_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: r#"
mod buggy {
    fn expensive_default(n: u32) -> u32 {
        (0..n).fold(0u32, |acc, i| acc.wrapping_add(i.wrapping_mul(31)))
    }
    pub fn resolve(opt: Option<u32>) -> u32 {
        opt.unwrap_or(expensive_default(std::hint::black_box(64))) // eagerly evaluated even when Some
    }
}

mod fixed {
    fn expensive_default(n: u32) -> u32 {
        (0..n).fold(0u32, |acc, i| acc.wrapping_add(i.wrapping_mul(31)))
    }
    pub fn resolve(opt: Option<u32>) -> u32 {
        opt.unwrap_or_else(|| expensive_default(std::hint::black_box(64))) // built only on None
    }
}

pub fn make_input(n: usize) -> Vec<Option<u32>> {
    // 99% Some, 1% None — realistic distribution (mirrors ok-or-else).
    (0..n).map(|i| if i % 100 == 0 { None } else { Some(i as u32) }).collect()
}

pub fn run_buggy(input: &[Option<u32>]) {
    // black_box(input) defeats loop-invariant hoisting of the pure loop body
    // out of the caller's timing loop (Batch 48 lesson).
    let input = std::hint::black_box(input);
    let sum: u32 = input.iter().map(|o| buggy::resolve(*o)).sum();
    std::hint::black_box(sum);
}

pub fn run_fixed(input: &[Option<u32>]) {
    let input = std::hint::black_box(input);
    let sum: u32 = input.iter().map(|o| fixed::resolve(*o)).sum();
    std::hint::black_box(sum);
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-count-to-any`: full-scan filter+count vs any
/// short-circuit. First element matches, rest don't — the maximal short-circuit
/// regime (the rule's selling point).
#[cfg(feature = "rust_perf_bench")]
fn filter_count_to_any_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn has_hit(vals: &[u32]) -> bool {
        vals.iter().filter(|&&v| v == 42).count() > 0 // full scan, always
    }
}

mod fixed {
    pub fn has_hit(vals: &[u32]) -> bool {
        vals.iter().any(|&v| v == 42) // stops at the first hit
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    // Hit lands at n/2 — an early-but-not-zero short-circuit regime. (A
    // position-0 hit drives `any` to a literal sub-ns call, which the
    // validator's div-by-zero guard classifies as a Tie — the guard exists
    // for malformed 0ns measurements, not for genuinely free calls.)
    let mut v = vec![7u32; n];
    v[n / 2] = 42;
    v
}

pub fn run_buggy(input: &[u32]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::has_hit(input));
}

pub fn run_fixed(input: &[u32]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::has_hit(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-macro-zero-fill`: uninit alloc + memset vs calloc.
/// Both sides black_box-read a byte near the end so the pages are touched
/// identically for the read; the difference is the WRITE path (memset vs
/// lazy zero pages).
#[cfg(feature = "rust_perf_bench")]
fn vec_macro_zero_fill_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn zero_buf(n: usize) -> Vec<u8> {
        let mut v = Vec::new();
        v.resize(n, 0); // uninit alloc + explicit memset
        v
    }
}

mod fixed {
    pub fn zero_buf(n: usize) -> Vec<u8> {
        vec![0u8; n] // alloc_zeroed — calloc zero pages
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    // black_box(n) defeats loop-invariant hoisting; spread reads (first /
    // middle / last) make partial memset elision observable.
    let n = std::hint::black_box(*input);
    let v = buggy::zero_buf(n);
    let mid = v.len() / 2;
    std::hint::black_box((v[0], v[mid], v[v.len() - 1]));
}

pub fn run_fixed(input: &usize) {
    let n = std::hint::black_box(*input);
    let v = fixed::zero_buf(n);
    let mid = v.len() / 2;
    std::hint::black_box((v[0], v[mid], v[v.len() - 1]));
}
"#
        .to_string(),
    }
}

/// Bench spec for `iter-copied-deref-map`: generic Map adapter vs specialized
/// copied(). Sum consumer at plain sizes — honest Tie expected in tight
/// release loops; the alloc axis + adapter markers carry the distinction.
#[cfg(feature = "rust_perf_bench")]
fn iter_copied_deref_map_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_kills(kills: &[u64]) -> u64 {
        kills.iter().map(|k| *k).sum()
    }
}

mod fixed {
    pub fn sum_kills(kills: &[u64]) -> u64 {
        kills.iter().copied().sum()
    }
}

pub fn make_input(n: usize) -> Vec<u64> {
    (0..n).map(|i| i as u64).collect()
}

pub fn run_buggy(input: &[u64]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::sum_kills(input));
}

pub fn run_fixed(input: &[u64]) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::sum_kills(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `filter-map-fuse`: two adapters vs filter_map. The
/// perf-book hedges — the Pareto verdict decides the annotation.
#[cfg(feature = "rust_perf_bench")]
fn filter_map_fuse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn even_squares(vals: &[u32]) -> Vec<u32> {
        vals.iter().filter(|&&v| v % 2 == 0).map(|&v| v * v).collect()
    }
}

mod fixed {
    pub fn even_squares(vals: &[u32]) -> Vec<u32> {
        vals.iter().filter_map(|&v| (v % 2 == 0).then_some(v * v)).collect()
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    std::hint::black_box(buggy::even_squares(input));
}

pub fn run_fixed(input: &[u32]) {
    std::hint::black_box(fixed::even_squares(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-insert-front-deque`: Vec::insert(0, x) O(n²) prepend
/// loop vs VecDeque::push_front O(1). Mirrors the vec-remove-front harness.
#[cfg(feature = "rust_perf_bench")]
fn vec_insert_front_deque_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![250, 1_000, 4_000],
        harness: r#"
use std::collections::VecDeque;

mod buggy {
    pub fn prepend_all(out: &mut Vec<u32>, items: &[u32]) {
        for &x in items {
            out.insert(0, x); // O(n) memmove each → O(n²) total
        }
    }
}

mod fixed {
    use std::collections::VecDeque;
    pub fn prepend_all(out: &mut VecDeque<u32>, items: &[u32]) {
        for &x in items {
            out.push_front(x); // O(1) ring head advance
        }
    }
}

pub fn make_input(n: usize) -> Vec<u32> {
    (0..n).map(|i| i as u32).collect()
}

pub fn run_buggy(input: &[u32]) {
    let mut out = Vec::with_capacity(input.len());
    buggy::prepend_all(&mut out, input);
    std::hint::black_box(out.len());
}

pub fn run_fixed(input: &[u32]) {
    let mut out = VecDeque::with_capacity(input.len());
    fixed::prepend_all(&mut out, input);
    std::hint::black_box(out.len());
}
"#
        .to_string(),
    }
}

/// Bench spec for `powi-vs-powf`: libm pow call vs single multiply. The
/// honest risk the bench adjudicates: LLVM instcombine may rewrite
/// `pow(x, 2.0)` to `x * x` in release, collapsing the win (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn powi_vs_powf_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_squares(vals: &[f32]) -> f32 {
        vals.iter().map(|v| v.powf(2.0)).sum()
    }
}

mod fixed {
    pub fn sum_squares(vals: &[f32]) -> f32 {
        vals.iter().map(|v| v.powi(2)).sum()
    }
}

pub fn make_input(n: usize) -> Vec<f32> {
    (0..n).map(|i| i as f32 * 0.5 - 0.25).collect()
}

pub fn run_buggy(input: &[f32]) {
    std::hint::black_box(buggy::sum_squares(input));
}

pub fn run_fixed(input: &[f32]) {
    std::hint::black_box(fixed::sum_squares(input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `distance-sq-no-sqrt`: sqrt-then-compare vs squared
/// compare. On aarch64 f32 sqrt is a single NEON instruction — the bench
/// decides whether the scalar-ARM reality matches the Box2D x86-SIMD
/// rationale (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn distance_sq_no_sqrt_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
pub struct Query {
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
}

mod buggy {
    pub fn count_in_range(pts: &[(f32, f32)], cx: f32, cy: f32, r: f32) -> usize {
        pts.iter()
            .filter(|&&(x, y)| {
                let dx = x - cx;
                let dy = y - cy;
                (dx * dx + dy * dy).sqrt() < r
            })
            .count()
    }
}

mod fixed {
    pub fn count_in_range(pts: &[(f32, f32)], cx: f32, cy: f32, r: f32) -> usize {
        let r2 = r * r;
        pts.iter()
            .filter(|&&(x, y)| {
                let dx = x - cx;
                let dy = y - cy;
                dx * dx + dy * dy < r2
            })
            .count()
    }
}

pub struct Input {
    pts: Vec<(f32, f32)>,
    q: Query,
}

pub fn make_input(n: usize) -> Input {
    let pts = (0..n).map(|i| (i as f32 * 0.013, i as f32 * 0.029)).collect();
    Input {
        pts,
        q: Query { cx: 0.0, cy: 0.0, r: 5000.0 },
    }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::count_in_range(&input.pts, input.q.cx, input.q.cy, input.q.r));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::count_in_range(&input.pts, input.q.cx, input.q.cy, input.q.r));
}
"#
        .to_string(),
    }
}

/// Bench spec for `runtime-int-exponent-powi`: runtime-integer-exponent
/// powf (libm call) vs powi (binary exponentiation). The f64 arm is the
/// source shape (RoPE theta scaling); the f32 arm checks the idiom holds
/// at lower precision. Rel-error gate lives in the harness's correctness
/// sum (max rel-error across the swept exponent range folded into the
/// black_box output — the validator only times; the GOAT gate asserts the
/// rel-error separately in corpus tests).
#[cfg(feature = "rust_perf_bench")]
fn runtime_int_exponent_powi_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    // runtime integer exponent -> opaque libm pow call per element
    pub fn theta_sum(base: f64, pos: &[i32], exp: i32) -> f64 {
        let mut acc = 0.0f64;
        for &p in pos {
            acc += base.powf((p * exp) as f64);
        }
        acc
    }
}

mod fixed {
    // powi: log2(n) multiplies, no libm call (negative n -> 1/x^|n|)
    pub fn theta_sum(base: f64, pos: &[i32], exp: i32) -> f64 {
        let mut acc = 0.0f64;
        for &p in pos {
            acc += base.powi(p * exp);
        }
        acc
    }
}

pub struct Input {
    pos: Vec<i32>,
    base: f64,
    exp: i32,
}

pub fn make_input(n: usize) -> Input {
    // swept runtime exponents across the range (rel-error surface):
    // pos alternates small/large so the harness sweeps |p*exp| widely
    let pos: Vec<i32> = (0..n)
        .map(|i| if i % 2 == 0 { (i % 17) as i32 } else { -((i % 29) as i32) })
        .collect();
    Input { pos, base: 10000.0f64, exp: 3 }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::theta_sum(input.base, &input.pos, input.exp));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::theta_sum(input.base, &input.pos, input.exp));
}
"#
        .to_string(),
    }
}

/// Bench spec for `hashmap-with-capacity`: rehash ladder vs single exact
/// reservation. The alloc axis gates this even where SipHash timing
/// dominates (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn hashmap_with_capacity_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![8, 64, 512, 4096],
        harness: r#"
mod buggy {
    use std::collections::HashMap;
    pub fn build(n: usize) -> usize {
        let mut m: HashMap<u64, u64> = HashMap::new();
        for i in 0..n as u64 {
            m.insert(i, i.wrapping_mul(2_654_435_761));
        }
        m.len()
    }
}

mod fixed {
    use std::collections::HashMap;
    pub fn build(n: usize) -> usize {
        let mut m: HashMap<u64, u64> = HashMap::with_capacity(n);
        for i in 0..n as u64 {
            m.insert(i, i.wrapping_mul(2_654_435_761));
        }
        m.len()
    }
}

pub fn make_input(n: usize) -> usize {
    n
}

pub fn run_buggy(input: &usize) {
    std::hint::black_box(buggy::build(*input));
}

pub fn run_fixed(input: &usize) {
    std::hint::black_box(fixed::build(*input));
}
"#
        .to_string(),
    }
}

/// Bench spec for `soa-hot-field`: 40-byte AoS stride scan vs contiguous
/// SoA column sum (Batch 53).
#[cfg(feature = "rust_perf_bench")]
fn soa_hot_field_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub struct Agent {
        pub x: f32,
        pub y: f32,
        pub vx: f32,
        pub vy: f32,
        pub hp: u32,
        pub level: u32,
        pub flags: u64,
        pub id: u64,
    }
    pub fn total_x(agents: &[Agent]) -> f32 {
        agents.iter().map(|a| a.x).sum()
    }
}

mod fixed {
    pub struct Agents {
        pub xs: Vec<f32>,
        pub ys: Vec<f32>,
        pub vxs: Vec<f32>,
        pub vys: Vec<f32>,
        pub hps: Vec<u32>,
        pub levels: Vec<u32>,
        pub flags: Vec<u64>,
        pub ids: Vec<u64>,
    }
    pub fn total_x(agents: &Agents) -> f32 {
        agents.xs.iter().sum()
    }
}

pub struct Input {
    aos: Vec<buggy::Agent>,
    soa: fixed::Agents,
}

pub fn make_input(n: usize) -> Input {
    let mut aos = Vec::with_capacity(n);
    let mut soa = fixed::Agents {
        xs: Vec::with_capacity(n),
        ys: Vec::with_capacity(n),
        vxs: Vec::with_capacity(n),
        vys: Vec::with_capacity(n),
        hps: Vec::with_capacity(n),
        levels: Vec::with_capacity(n),
        flags: Vec::with_capacity(n),
        ids: Vec::with_capacity(n),
    };
    for i in 0..n as u64 {
        let x = i as f32 * 0.5;
        let y = i as f32 * 0.25;
        let vx = i as f32 * 0.125;
        let vy = i as f32 * 0.0625;
        aos.push(buggy::Agent {
            x,
            y,
            vx,
            vy,
            hp: (i % 1000) as u32,
            level: (i % 50) as u32,
            flags: i,
            id: i,
        });
        soa.xs.push(x);
        soa.ys.push(y);
        soa.vxs.push(vx);
        soa.vys.push(vy);
        soa.hps.push((i % 1000) as u32);
        soa.levels.push((i % 50) as u32);
        soa.flags.push(i);
        soa.ids.push(i);
    }
    Input { aos, soa }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::total_x(&input.aos));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::total_x(&input.soa));
}
"#
        .to_string(),
    }
}

/// Bench spec for `division-by-invariant-multiplication`: hardware integer
/// division by a loop-invariant RUNTIME divisor vs libdivide-style magic
/// multiplication (Batch 59, thermite distill). The divisor is a runtime
/// field passed through `black_box` at the call boundary — LLVM already
/// strength-reduces compile-time-constant divisors, so the baseline must
/// stay opaque for the bench to measure the rule's actual regime. Setup
/// (one hardware div inside `FastDiv::new`) is INSIDE the timed call — the
/// honest per-batch shape; it amortizes to noise at n >= 1000.
#[cfg(feature = "rust_perf_bench")]
const DIVISION_INVARIANT_FIXED_SRC: &str = r#"    pub struct FastDivU32 { magic: u32, shift: u32, add: bool }
    impl FastDivU32 {
        pub fn new(d: u32) -> Self {
            let l = 31 - d.leading_zeros();
            if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
            let k = 1u64 << l;
            let base = (k << 32) / u64::from(d);
            let rem = (k << 32) % u64::from(d);
            if u64::from(d) - rem < k {
                Self { magic: base as u32 + 1, shift: l, add: false } // ceil magic — the +1 is load-bearing
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u64::from(d) { m += 1; }
                Self { magic: (m + 1) as u32, shift: l, add: true } // (m+1) wraps mod 2^32 — intended
            }
        }
        #[inline(always)]
        pub fn div(&self, x: u32) -> u32 {
            if self.magic == 0 { return x >> self.shift; } // power-of-two divisor
            let hi = ((u64::from(self.magic) * u64::from(x)) >> 32) as u32;
            // widening add: hi + x reaches ~2^33 — a u32 wrap drops the carry bit
            // (d=7 x=u32::MAX fails by 2^29). The theorem holds in wide math.
            if self.add { ((u64::from(hi) + u64::from(x)) >> 1 >> self.shift) as u32 } else { hi >> self.shift }
        }
    }
    pub struct FastDivU64 { magic: u64, shift: u32, add: bool }
    impl FastDivU64 {
        pub fn new(d: u64) -> Self {
            let l = 63 - d.leading_zeros();
            if d.is_power_of_two() { return Self { magic: 0, shift: l, add: false }; }
            let k = 1u128 << l;
            let base = (k << 64) / u128::from(d);
            let rem = (k << 64) % u128::from(d);
            if u128::from(d) - rem < k {
                Self { magic: base as u64 + 1, shift: l, add: false } // ceil magic — the +1 is load-bearing
            } else {
                let mut m = base * 2;
                let rem2 = rem * 2;
                if rem2 >= u128::from(d) { m += 1; }
                Self { magic: (m + 1) as u64, shift: l, add: true } // (m+1) wraps mod 2^64 — intended
            }
        }
        #[inline(always)]
        pub fn div(&self, x: u64) -> u64 {
            if self.magic == 0 { return x >> self.shift; } // power-of-two divisor
            let hi = ((u128::from(self.magic) * u128::from(x)) >> 64) as u64;
            // widening add (see u32 twin): u64 wrap would drop the carry bit
            if self.add { ((u128::from(hi) + u128::from(x)) >> 1 >> self.shift) as u64 } else { hi >> self.shift }
        }
    }
    pub fn sum_quot_u32(data: &[u32], d: u32) -> u32 {
        let fd = FastDivU32::new(d); // one hardware div, hoisted
        let mut acc = 0u32;
        for &x in data {
            acc = acc.wrapping_add(fd.div(x)); // mulhi + shifts per element
        }
        acc
    }
    pub fn sum_quot_u64(data: &[u64], d: u64) -> u64 {
        let fd = FastDivU64::new(d);
        let mut acc = 0u64;
        for &x in data {
            acc = acc.wrapping_add(fd.div(x));
        }
        acc
    }"#;

#[cfg(feature = "rust_perf_bench")]
pub(super) fn division_invariant_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000, 1_000_000],
        harness: r#"
mod buggy {
    pub fn sum_quot_u32(data: &[u32], d: u32) -> u32 {
        let mut acc = 0u32;
        for &x in data {
            acc = acc.wrapping_add(x / d); // hardware udiv per element
        }
        acc
    }
    pub fn sum_quot_u64(data: &[u64], d: u64) -> u64 {
        let mut acc = 0u64;
        for &x in data {
            acc = acc.wrapping_add(x / d);
        }
        acc
    }
}

mod fixed {
__FASTDIV_IMPLS__
}

pub struct Input {
    data32: Vec<u32>,
    data64: Vec<u64>,
    d32: u32,
    d64: u64,
}

pub fn make_input(n: usize) -> Input {
    // diverse residues; the divisors are runtime fields (NOT constants —
    // see the spec doc comment on strength reduction)
    let data32 = (0..n).map(|i| (i as u32).wrapping_mul(2654435761)).collect();
    let data64 = (0..n).map(|i| (i as u64).wrapping_mul(6_364_136_223_846_793_005)).collect();
    Input { data32, data64, d32: 97, d64: 1000003 }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::sum_quot_u32(&input.data32, std::hint::black_box(input.d32)));
    std::hint::black_box(buggy::sum_quot_u64(&input.data64, std::hint::black_box(input.d64)));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::sum_quot_u32(&input.data32, std::hint::black_box(input.d32)));
    std::hint::black_box(fixed::sum_quot_u64(&input.data64, std::hint::black_box(input.d64)));
}"#
        .replace("__FASTDIV_IMPLS__", DIVISION_INVARIANT_FIXED_SRC),
    }
}

/// Bench spec for `keyword-map-to-match`: static-keyword HashMap dispatch vs
/// plain match on string literals. The map pays SipHash per lookup; the match
/// compiles to a switch on length + 1-2 inline compares (LLVM; oxc verified
/// via --emit=llvm-ir after failing to beat it with a gperf-style perfect
/// hash). Mixed identifier workload (7 keywords + 3 non-keywords cycling);
/// the LazyLock init amortizes across the timing loop — the measured cost is
/// per-lookup dispatch, not construction.
#[cfg(feature = "rust_perf_bench")]
fn keyword_map_to_match_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: r#"
mod buggy {
    use std::collections::HashMap;
    use std::sync::LazyLock;
    #[derive(Clone, Copy)]
    pub enum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }
    static KEYWORDS: LazyLock<HashMap<&'static str, Kind>> = LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("as", Kind::As);
        m.insert("do", Kind::Do);
        m.insert("if", Kind::If);
        m.insert("let", Kind::Let);
        m.insert("fn", Kind::Fn);
        m.insert("while", Kind::While);
        m.insert("constructor", Kind::Constructor);
        m
    });
    pub fn match_keyword(s: &str) -> u64 {
        *KEYWORDS.get(s).unwrap_or(&Kind::Ident) as u64
    }
}

mod fixed {
    #[derive(Clone, Copy)]
    pub enum Kind { As, Do, If, Let, Fn, While, Constructor, Ident }
    pub fn match_keyword(s: &str) -> u64 {
        let k = match s {
            "as" => Kind::As,
            "do" => Kind::Do,
            "if" => Kind::If,
            "let" => Kind::Let,
            "fn" => Kind::Fn,
            "while" => Kind::While,
            "constructor" => Kind::Constructor,
            _ => Kind::Ident,
        };
        k as u64
    }
}

pub fn make_input(n: usize) -> Vec<&'static str> {
    let words = ["as", "identifier", "do", "if", "some_long_name", "let", "fn", "while", "constructor", "another_name"];
    (0..n).map(|i| words[i % words.len()]).collect()
}

pub fn run_buggy(input: &[&'static str]) {
    let input = std::hint::black_box(input);
    let acc: u64 = input.iter().map(|s| buggy::match_keyword(s)).sum();
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &[&'static str]) {
    let input = std::hint::black_box(input);
    let acc: u64 = input.iter().map(|s| fixed::match_keyword(s)).sum();
    std::hint::black_box(acc);
}"#
        .to_string(),
    }
}

/// Bench spec for `usize-index-to-u32`: 16-byte spans vs 8-byte spans under a
/// bandwidth-bound sum pass. At 1K elements both arrays are L1-resident (16KB
/// vs 8KB) — expected Tie. At 1M the wide array is 16MB (exceeds L2 on most
/// cores) while the narrow is 8MB — the bandwidth win should materialize.
/// Same access pattern both sides; only the field width differs (the oxc Span
/// change, isolated).
#[cfg(feature = "rust_perf_bench")]
fn usize_index_to_u32_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 1_000_000],
        harness: r#"
mod buggy {
    pub struct Span { pub start: usize, pub end: usize }
    pub fn total_len(spans: &[Span]) -> u64 {
        let mut acc = 0u64;
        for s in spans { acc += (s.end - s.start) as u64; }
        acc
    }
}

mod fixed {
    pub struct Span { pub start: u32, pub end: u32 }
    pub fn total_len(spans: &[Span]) -> u64 {
        let mut acc = 0u64;
        for s in spans { acc += u64::from(s.end - s.start); }
        acc
    }
}

pub struct Input { wide: Vec<buggy::Span>, narrow: Vec<fixed::Span> }

pub fn make_input(n: usize) -> Input {
    let wide: Vec<buggy::Span> = (0..n)
        .map(|i| buggy::Span { start: i * 3, end: i * 3 + (i % 7) + 1 })
        .collect();
    let narrow: Vec<fixed::Span> = wide
        .iter()
        .map(|s| fixed::Span { start: s.start as u32, end: s.end as u32 })
        .collect();
    Input { wide, narrow }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    std::hint::black_box(buggy::total_len(&input.wide));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    std::hint::black_box(fixed::total_len(&input.narrow));
}"#
        .to_string(),
    }
}

/// Bench spec for `hot-byval-move-over-memcpy-threshold` (Batch 112,
/// Research 134 — the nnethercote July-2026 distill): a 136-byte record
/// moved BY VALUE per hot-loop call (LLVM lowers each move to a `memcpy`
/// call above the inline threshold) vs the composed fix — the record shrunk
/// to 104 bytes + the 32-byte cold tail passed by reference and mutated in
/// place. Both arms fold the SAME 17 u64s per score call with the SAME
/// per-field ops in the same order (the fairness gate pins it), so the
/// measured delta is move cost only, never workload. `#[inline(never)]`
/// keeps the calls out-of-line (the source's shape); `black_box` at the
/// call sites defeats move elision. n = hot-loop iterations. Composition
/// disclosure: a binary bench cannot separate the shrink step from the
/// cold-tail move-avoidance — the measured delta is the composed fix,
/// exactly as the rule's fix space prescribes.
/// Alloc axis: zero allocations both arms (stack-only values).
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // data table: the harness is a single embedded raw string
fn hot_byval_move_memcpy_threshold_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 1_000_000],
        harness: r#"
mod buggy {
    #[derive(Clone, Copy)]
    pub struct Record { pub hot: [u64; 13], pub cold: [u64; 4] } // 17 x 8 = 136 B (> 128)

    #[inline(never)]
    pub fn score(r: Record) -> u64 {
        let mut a = 0u64;
        for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }
        for i in 0..4 { a = a.wrapping_add(r.cold[i].wrapping_mul(3)); }
        a
    }
}

mod fixed {
    #[derive(Clone, Copy)]
    pub struct Record { pub hot: [u64; 13] } // 13 x 8 = 104 B (<= 128 on x86-64)

    #[inline(never)]
    pub fn score(r: Record, cold: &[u64; 4]) -> u64 {
        let mut a = 0u64;
        for i in 0..13 { a = a.wrapping_add(r.hot[i].rotate_left(7) ^ (i as u64)); }
        for i in 0..4 { a = a.wrapping_add(cold[i].wrapping_mul(3)); }
        a
    }
}

pub struct Input { seed_hot: [u64; 13], seed_cold: [u64; 4], iters: usize }

pub fn make_input(n: usize) -> Input {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x };
    let mut seed_hot = [0u64; 13];
    let mut seed_cold = [0u64; 4];
    for v in seed_hot.iter_mut() { *v = next(); }
    for v in seed_cold.iter_mut() { *v = next(); }
    Input { seed_hot, seed_cold, iters: n }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut state = buggy::Record { hot: input.seed_hot, cold: input.seed_cold };
    let mut acc = 0u64;
    for i in 0..input.iters {
        acc ^= buggy::score(std::hint::black_box(state)); // by-value 136 B move per call
        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);
        state.cold[i & 3] = state.cold[i & 3].rotate_left(1);
    }
    std::hint::black_box((state, acc));
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut state = fixed::Record { hot: input.seed_hot };
    let mut cold = input.seed_cold;
    let mut acc = 0u64;
    for i in 0..input.iters {
        acc ^= fixed::score(std::hint::black_box(state), std::hint::black_box(&cold)); // 104 B move + by-ref tail
        state.hot[i % 13] = state.hot[i % 13].wrapping_add(acc);
        cold[i & 3] = cold[i & 3].rotate_left(1); // cold tail: zero copies
    }
    std::hint::black_box((state, cold, acc));
}
"#
            .to_string(),
    }
}

/// Bench spec for `transposed-weight-register-gemv`: row-major per-row dot
/// with its horizontal reduce, vs transposed weights with per-input
/// broadcast FMA
/// into output-vector accumulators (the ryg 2015 transposed-lane Mv
/// analysis; microGPT-C `mv16_blk16v(4)` shape, portable-Rust form).
/// NOUT fixed at 64 (4 NEON / 2 AVX2 full lanes); `n` sweeps NIN
/// 16 / 64 / 4096. **Measured (Batch 62, M3 Max): ParetoDominant 1.18x /
/// 2.09x / 6.08x — the research's "wash at large nin" expectation was
/// WRONG-DIRECTION**: in strict-FP Rust the row-dot's scalar accumulator is
/// a serial dependency chain LLVM cannot vectorize without reassociation,
/// so it degrades FURTHER at large nin (llama.cpp's fine row-major dots use
/// explicit multi-accumulator SIMD — not what portable scalar code gets).
/// NOT bit-identical (float sum order changes under transposition) — the
/// validator's rel-error tolerance covers it (the `distance-sq-no-sqrt`
/// 1-ulp caveat precedent). `y` is cloned per run (identical alloc count
/// both sides — the alloc axis compares shapes, not the steady-state
/// discipline).
#[cfg(feature = "rust_perf_bench")]
fn transposed_weight_register_gemv_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![16, 64, 4096],
        harness: "\nconst NOUT: usize = 64; // 4 NEON vectors / 2 AVX2 vectors of lanes — tile stays full\n\nmod buggy {\n    pub fn matvec(w: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n        for r in 0..nout {\n            let row = &w[r * nin..(r + 1) * nin];\n            let mut acc = 0.0f32;\n            for c in 0..nin {\n                acc += row[c] * x[c];\n            }\n            y[r] = acc;\n        }\n    }\n}\n\nmod fixed {\n    pub fn matvec(w_t: &[f32], x: &[f32], y: &mut [f32], nout: usize, nin: usize) {\n        for r in 0..nout { y[r] = 0.0; }\n        for c in 0..nin {\n            let xc = x[c];\n            let col = &w_t[c * nout..(c + 1) * nout];\n            for r in 0..nout {\n                y[r] += xc * col[r];\n            }\n        }\n    }\n}\n\npub struct Input { w_row: Vec<f32>, w_col: Vec<f32>, x: Vec<f32>, y: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let nin = n; // n is the input dimension; NOUT fixed at 64\n    let nout = NOUT;\n    // Same values both layouts — only the layout differs.\n    let vals = |i: usize, j: usize| {\n        let h = (i as u32).wrapping_mul(0x9E37_79B9) ^ (j as u32).wrapping_mul(0x85EB_CA6B);\n        ((h >> 8) as f32) / 16777216.0 - 0.5\n    };\n    let w_row: Vec<f32> = (0..nout).flat_map(|r| (0..nin).map(move |c| vals(r, c))).collect();\n    let w_col: Vec<f32> = (0..nin).flat_map(|c| (0..nout).map(move |r| vals(r, c))).collect();\n    let x: Vec<f32> = (0..nin).map(|c| vals(9999, c)).collect();\n    Input { w_row, w_col, x, y: vec![0.0; nout] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let nin = input.x.len();\n    buggy::matvec(&input.w_row, &input.x, &mut input.y.clone(), NOUT, nin);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let nin = input.x.len();\n    fixed::matvec(&input.w_col, &input.x, &mut input.y.clone(), NOUT, nin);\n}".to_string(),
    }
}

/// Bench spec for `newton-refinement-vs-hardware-transcendental`:
/// seed + Newton-refined `inv`/`rsqrt` vs the plain hardware ops. On CPU
/// the hardware op is single-instruction + correctly rounded, so the
/// refinement steps are pure extra work. **Measured (Batch 62, M3 Max):
/// Mixed — hardware never loses: 1.09x at 1K, 1.01x at 100K** (source
/// measured 2-4% on Ryzen 5 5600H; the M3's fdiv/sqrt latency flattens
/// the gap at 100K where the pipeline hides it). **x86 re-gate
/// (2026-08-19, 4090 box): ParetoDominant 1.64x/1.66x — the win is
/// LARGER on x86; the near-tie is M3-aarch64-specific.** The rule's purpose is
/// stopping the plausible-looking GPU→CPU fast-math port — the annotation
/// class (`filter-count-to-any` precedent). Well-conditioned [0.5, 2.0]
/// inputs — deterministic.
#[cfg(feature = "rust_perf_bench")]
fn newton_refinement_vs_hardware_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: "\nmod buggy {\n    pub fn fast_inv(x: f32) -> f32 {\n        let r = 1.0 / x;\n        let r = r * (2.0 - x * r);\n        r * (2.0 - x * r)\n    }\n    pub fn fast_rsqrt(x: f32) -> f32 {\n        let r = 1.0 / x.sqrt();\n        r * (0.5 * (3.0 - x * r * r))\n    }\n}\n\nmod fixed {\n    pub fn fast_inv(x: f32) -> f32 { 1.0 / x }\n    pub fn fast_rsqrt(x: f32) -> f32 { 1.0 / x.sqrt() }\n}\n\npub fn make_input(n: usize) -> Vec<f32> {\n    // Values in [0.5, 2.0]: no inf/nan/underflow in inv or rsqrt —\n    // deterministic, well-conditioned.\n    (0..n)\n        .map(|i| {\n            let h = (i as u32).wrapping_mul(0x9E37_79B9);\n            0.5 + ((h >> 8) as f32) / 8388608.0\n        })\n        .collect()\n}\n\npub fn run_buggy(input: &[f32]) {\n    let input = std::hint::black_box(input);\n    let acc: f32 = input\n        .iter()\n        .map(|&x| buggy::fast_inv(x) + buggy::fast_rsqrt(x))\n        .sum();\n    std::hint::black_box(acc);\n}\n\npub fn run_fixed(input: &[f32]) {\n    let input = std::hint::black_box(input);\n    let acc: f32 = input\n        .iter()\n        .map(|&x| fixed::fast_inv(x) + fixed::fast_rsqrt(x))\n        .sum();\n    std::hint::black_box(acc);\n}".to_string(),
    }
}

/// Bench spec for `loop-order-innermost-contiguous`: i-j-k vs i-k-j
/// nested-accumulation matmul. The inner loop's access pattern is the
/// whole story: column-stride-n vs contiguous row walk (L1 misses
/// 45%->12% in the source). Square sizes 64/128/256 — L1/L2-resident so
/// the access-pattern effect dominates. Float sum-order differs between
/// the orders — the validator's rel-error tolerance covers it (NOT
/// bit-identity; the `distance-sq-no-sqrt` 1-ulp caveat precedent).
#[cfg(feature = "rust_perf_bench")]
fn loop_order_innermost_contiguous_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![64, 128, 256],
        harness: "\nmod buggy {\n    pub fn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n        for i in 0..n {\n            for j in 0..n {\n                let mut acc = 0.0f32;\n                for k in 0..n {\n                    acc += a[i * n + k] * b[k * n + j]; // k strides b's OUTER axis\n                }\n                c[i * n + j] = acc;\n            }\n        }\n    }\n}\n\nmod fixed {\n    pub fn matmul(a: &[f32], b: &[f32], c: &mut [f32], n: usize) {\n        for i in 0..n {\n            for j in 0..n {\n                c[i * n + j] = 0.0;\n            }\n        }\n        for i in 0..n {\n            let a_row = &a[i * n..(i + 1) * n];\n            for k in 0..n {\n                let a_ik = a_row[k];\n                let b_row = &b[k * n..(k + 1) * n];\n                let c_row = &mut c[i * n..(i + 1) * n];\n                for j in 0..n {\n                    c_row[j] += a_ik * b_row[j]; // j strides by 1 everywhere\n                }\n            }\n        }\n    }\n}\n\npub struct Input { a: Vec<f32>, b: Vec<f32>, c: Vec<f32> }\n\npub fn make_input(n: usize) -> Input {\n    let vals = |s: u32| (0..n * n)\n        .map(move |i| {\n            let h = (i as u32).wrapping_mul(0x9E37_79B9).wrapping_add(s);\n            ((h >> 8) as f32) / 16_777_216.0 - 0.5\n        })\n        .collect();\n    Input { a: vals(1), b: vals(2), c: vec![0.0; n * n] }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let n = (input.a.len() as f64).sqrt() as usize;\n    buggy::matmul(&input.a, &input.b, &mut input.c.clone(), n);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let n = (input.a.len() as f64).sqrt() as usize;\n    fixed::matmul(&input.a, &input.b, &mut input.c.clone(), n);\n}".to_string(),
    }
}

/// Bench spec for `debug-assert-length-hoist`: debug_assert vs assert
/// before an indexing loop. The whole point is RELEASE behavior
/// (debug_assert is erased there — in debug both assert forms run);
/// the validator builds generated crates in release, so the buggy side
/// retains every bounds check. **Measured (Batch 64, M3 aarch64):
/// Mixed regime-split — 1.13x at n=1K (elision win) but a STABLE
/// 0.67-0.71x LOSS at n=100K** (3-run verified; the article's own
/// aarch64 number was 1.03x; x86 re-gate 2026-08-19 on the 4090 box:
/// 1.00x tie @1K, 0.92x @100K — regime confirmed cross-arch, milder
/// loss; the x86 headline win does not reproduce on this shape). The
/// canonical range-pinned shape
/// (first+count <= len feeding the induction loop); data-dependent
/// index arrays cannot propagate the constraint at all (measured
/// 0.74-0.80x dominated on the first harness shape — elision is
/// impossible there, the assert is pure cost).
#[cfg(feature = "rust_perf_bench")]
fn debug_assert_length_hoist_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 100_000],
        harness: "\nmod buggy {\n    pub fn range_checksum(rows: &[(u64, u64)], first: usize, count: usize,\n                          out: &mut Vec<u64>) {\n        debug_assert!(first + count <= rows.len()); // ERASED in release —\n        // the optimizer loses exactly this range constraint\n        for i in first..first + count {\n            out.push(rows[i].0 ^ rows[i].1); // bounds checks RETAINED\n        }\n    }\n}\n\nmod fixed {\n    pub fn range_checksum(rows: &[(u64, u64)], first: usize, count: usize,\n                          out: &mut Vec<u64>) {\n        assert!(first + count <= rows.len()); // SURVIVES release — the range\n        // constraint stays visible to the optimizer\n        for i in first..first + count {\n            out.push(rows[i].0 ^ rows[i].1); // bounds checks ELIDED\n        }\n    }\n}\n\npub struct Input { rows: Vec<(u64, u64)>, scratch: Vec<u64> }\n\npub fn make_input(n: usize) -> Input {\n    let rows: Vec<(u64, u64)> = (0..n)\n        .map(|i| {\n            let h = (i as u64).wrapping_mul(0x9E37_79B9_97F4_A7C1);\n            (h ^ (h >> 31), h.wrapping_mul(3))\n        })\n        .collect();\n    Input { rows, scratch: Vec::with_capacity(n) }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut out = input.scratch.clone();\n    buggy::range_checksum(&input.rows, 3, input.rows.len() - 3, &mut out);\n    std::hint::black_box(&out);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut out = input.scratch.clone();\n    fixed::range_checksum(&input.rows, 3, input.rows.len() - 3, &mut out);\n    std::hint::black_box(&out);\n}".to_string(),
    }
}

/// Bench spec for `sparse-set-bit-iteration-tz-blsr`: 64-iteration
/// shift-test scan vs trailing_zeros+BLSR loop over sparse masks.
///
/// ~4/64 set bits per word (the rule's applicable sparse regime —
/// `0x0101..` selects bit 0 of each byte, each set with p~1/2). The
/// buggy form always executes 64 shift+test iterations; the fixed form
/// executes popcount iterations (~4). Both emit identical index sums
/// (bit-identical G1 by construction). At dense masks the two converge —
/// the honest regime boundary recorded in the rule's annotation.
#[cfg(feature = "rust_perf_bench")]
fn sparse_set_bit_iteration_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000, 1_000_000],
        harness: "\nmod buggy {\n    pub fn sum_set_positions(mask: u64) -> u32 {\n        let mut sum = 0;\n        for i in 0..64 {\n            if (mask >> i) & 1 != 0 { sum += i as u32; }\n        }\n        sum\n    }\n}\n\nmod fixed {\n    pub fn sum_set_positions(mask: u64) -> u32 {\n        let mut sum = 0;\n        let mut m = mask;\n        while m != 0 {\n            let i = m.trailing_zeros();\n            sum += i;\n            m &= m.wrapping_sub(1);\n        }\n        sum\n    }\n}\n\npub fn make_input(n: usize) -> Vec<u64> {\n    // ~4/64 set bits per word (sparse regime — the rule's applicable\n    // window): 0x0101.. selects bit 0 of each byte, each set with p~1/2\n    // under the multiplicative hash.\n    (0..n)\n        .map(|i| (i as u64).wrapping_mul(0x9E37_79B9_97F4_A7C1) & 0x0101_0101_0101_0101)\n        .collect()\n}\n\npub fn run_buggy(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let mut acc = 0u32;\n    for &m in input {\n        acc = acc.wrapping_add(buggy::sum_set_positions(m));\n    }\n    std::hint::black_box(acc);\n}\n\npub fn run_fixed(input: &[u64]) {\n    let input = std::hint::black_box(input);\n    let mut acc = 0u32;\n    for &m in input {\n        acc = acc.wrapping_add(fixed::sum_set_positions(m));\n    }\n    std::hint::black_box(acc);\n}\n".to_string(),
    }
}

/// Bench spec for `narrow-counter-overflow-promotion` (Batch 78,
/// Research 092 — Datadog RTDB/Monocle): the full-width u64 counter
/// buffer vs the u32+sidecar form. Sizes = counter count; working set
/// 0.8/8/32 MB (u64 arm) vs 0.4/4/16 MB (u32 arm) — the smallest size
/// is L2-resident (density pays nothing — the honest Tie regime), the
/// largest is DRAM-bound. Alloc axis: buggy 1 alloc of 8N bytes, fixed
/// 1 alloc of 4N (empty HashMap never allocates) — the halving is exact.
/// G1: totals are closed-form identical (no promotion fires in the
/// common regime; debug_assert per run).
#[cfg(feature = "rust_perf_bench")]
fn narrow_counter_overflow_promotion_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000, 4_000_000],
        harness: "\nmod buggy {\n    // The full-width buffer: every counter u64 \"just in case\".\n    pub struct Bins { pub counts: Vec<u64> }\n    pub fn bump_all(c: &mut Bins, updates: &[(u32, u32)]) {\n        for &(i, v) in updates {\n            c.counts[i as usize] += u64::from(v);\n        }\n    }\n    pub fn total(c: &Bins) -> u64 {\n        c.counts.iter().sum()\n    }\n}\n\nmod fixed {\n    use std::collections::HashMap;\n    // Common case narrowed to u32; overflow promotes to a sparse sidecar.\n    pub struct Bins {\n        pub counts: Vec<u32>,\n        pub wide: HashMap<usize, u64>,\n    }\n    pub fn bump_all(c: &mut Bins, updates: &[(u32, u32)]) {\n        for &(i, v) in updates {\n            let i = i as usize;\n            if c.wide.is_empty() {\n                // common regime: nothing ever promoted\n                match c.counts[i].checked_add(v) {\n                    Some(x) => c.counts[i] = x,\n                    None => {\n                        let w = u64::from(c.counts[i]) + u64::from(v);\n                        c.counts[i] = 0; // narrow slot is dead now\n                        c.wide.insert(i, w);\n                    }\n                }\n                continue;\n            }\n            if let Some(w) = c.wide.get_mut(&i) {\n                *w += u64::from(v);\n                continue;\n            }\n            match c.counts[i].checked_add(v) {\n                Some(x) => c.counts[i] = x,\n                None => {\n                    let w = u64::from(c.counts[i]) + u64::from(v);\n                    c.counts[i] = 0;\n                    c.wide.insert(i, w);\n                }\n            }\n        }\n    }\n    pub fn total(c: &Bins) -> u64 {\n        c.counts.iter().map(|&n| u64::from(n)).sum::<u64>()\n            + c.wide.values().sum::<u64>()\n    }\n}\n\n/// Deterministic update stream: every slot bumped exactly UPDATES_PER_SLOT\n/// times with LCG deltas 1..=64 (the common regime — no slot approaches\n/// u32::MAX, so the fixed arm never promotes; totals are closed-form).\nconst UPDATES_PER_SLOT: u64 = 4;\nconst DELTA_MAX: u32 = 64;\n\npub struct Input {\n    n: usize,\n    updates: Vec<(u32, u32)>,\n    expected_total: u64,\n}\n\npub fn make_input(n: usize) -> Input {\n    let mut rng: u64 = 0x0B78_0D06_C0DE_5EED; // seeded LCG\n    let rng = &mut rng;\n    let mut updates: Vec<(u32, u32)> = Vec::with_capacity(n * UPDATES_PER_SLOT as usize);\n    let mut expected: u64 = 0;\n    // round-robin slots so both arms walk the array identically\n    for round in 0..UPDATES_PER_SLOT {\n        for i in 0..n {\n            *rng = rng.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);\n            let v = ((*rng >> 33) as u32 % DELTA_MAX) + 1; // 1..=64\n            updates.push((i as u32, v));\n            expected += u64::from(v);\n            std::hint::black_box(&round);\n        }\n    }\n    Input { n, updates, expected_total: expected }\n}\n\npub fn run_buggy(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut bins = buggy::Bins { counts: vec![0u64; input.n] };\n    buggy::bump_all(&mut bins, &input.updates);\n    let t = buggy::total(&bins);\n    std::hint::black_box(t);\n    debug_assert_eq!(t, input.expected_total);\n}\n\npub fn run_fixed(input: &Input) {\n    let input = std::hint::black_box(input);\n    let mut bins = fixed::Bins { counts: vec![0u32; input.n], wide: std::collections::HashMap::new() };\n    fixed::bump_all(&mut bins, &input.updates);\n    let t = fixed::total(&bins);\n    std::hint::black_box(t);\n    debug_assert_eq!(t, input.expected_total);\n}\n".to_string(),
    }
}
