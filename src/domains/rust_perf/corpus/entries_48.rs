//! Entries slice - Batch 282 (2026-10-25): the `PerfR` C-to-Rust translation
//! perf-issue classes - arXiv 2610.23044v1 ("Where Does the Performance Go?
//! Understanding and Recovering Performance Regressions in C-to-Rust
//! Translation", Yuan/Wang/Zhou/Liu/Peng - Fudan + NTU, submitted 2026-10-16).
//! Three own `rust_perf` rules from the paper's nine recurring classes (Table IV,
//! 269 instances over 20 real C projects + their c2rust translations): class 2
//! `Redundant Conversion Saturation` (arithmetic), class 5 `Repeated Invariant
//! Dispatch` (dispatch), class 9 `Obscured Memory Access Properties`
//! (iterators - the paper's MOST WIDESPREAD rule, matching 89% of hot
//! functions across all 12 held-out evaluation projects). The other six
//! classes are adjacent-covered or c2rust-specific at this batch: class 1
//! (residual bounds checks) rides the face on `chunks-exact` (slice/iterator
//! traversal exposes index-length relationships - the corpus + upstream
//! `needless_range_loop` own the HOW), class 3 (missed loop vectorization)
//! rides the face on `simd-chunk`, class 4 (missed hot-callee inlining) rides
//! the face on `inline-hint-not-through-unmarked-wrapper` (the
//! translation-DROPS-attributes face), classes 6/7/8 (buffer management,
//! byte-oriented ops, element-wise processing) are corpus-covered
//! (`launch-cost-scales-with-allocation-count-pack-one-arena` /
//! `bounded-batch-buffer-pool`, `copy-from-slice` /
//! `unaligned-word-load-from-bytes` / `hot-byval-move-over-memcpy-threshold`,
//! `chunks-exact` / `dual-mask-fused-block-scan` /
//! `dense-table-bulk-update-via-contiguous-iter`) and dropped honestly.
//!
//! All three rules are corpus-only `NotBenchable` (the source figures are
//! paper-reported on THEIR benchmark: i9-10980XE, clang-17 vs
//! rustc 1.77.0-nightly, both LLVM 17.0.6 - provenance, never bars; the
//! decision content is the law + the regime) - no proposer files, so the
//! healer never auto-applies these rewrites; the UNSAFE-adjacent fix
//! direction (rules 1 and 3) is disclosed in `context_notes` as a
//! verify-gated MANUAL rewrite with its proof obligations named, exactly the
//! paper's own optimization-card discipline ("Cards with unmet or unresolved
//! preconditions are excluded"). Fixtures are ORIGINAL machinery (never the
//! studied projects' code verbatim - the bzip2/heman/xxHash/libzahl/miniz
//! instances are cited as shapes, not pasted), each compiled and executed
//! standalone (rustc --edition 2021 --test) before this table was assembled;
//! the buggy/fixed arm pair shares the surrounding harness and differs only
//! on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── arithmetic: a range-PROVEN float→int cast still pays the
        // saturating-conversion machinery -------------------------------
        // Batch 282 / arXiv 2610.23044v1 class 2 (`Redundant Conversion
        // Saturation`, the heman fastFloor instance: seven saturating
        // conversions survive inlining into the island-noise inner loop
        // because `as` lowers to llvm.fptosi.sat even when the operand's
        // range rules saturation out; the C build's plain fptosi never
        // pays it). Rust-native exposure: every `as` cast from a float
        // type carries the same saturation semantics.
        PerfRule {
            rule_id: "saturating-f2i-cast-proven-range-to-int-unchecked".to_string(),
            name: "a hot-loop float→int `as` cast whose operand range is PROVEN in-range (pre-clamped input, bounded index math, a preceding assert) still pays the saturating-conversion machinery (llvm.fptosi.sat / .fptoui.sat) on every iteration - when the truncated value provably fits the target integer type, `to_int_unchecked` drops the saturation dead work (the C-build parity shape: plain fptosi), and the proof obligations (finite operand, in-range truncation) are the caller's, carried as a SAFETY comment at the site - an unproven range stays on `as` forever"
                .to_string(),
            category: "arithmetic".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `saturating-f2i-cast-proven-range-to-int-unchecked`
// (riir-refine rust_perf corpus, Batch 282) -- the ANTI-PATTERN arm.
//
// Re-authored shape from arXiv 2610.23044v1 §IV-C (the heman
// `fastFloor` island-noise instance, cited as a shape, not pasted):
// a hot noise-generation loop converts float samples to integers with
// `as`. The input is PRE-CLAMPED to [0.0, 1.0) and the multiplier is
// the fixed grid width, so the truncated value provably fits -- yet
// every iteration still executes the full saturating-conversion
// sequence (NaN/overflow checks + the clamp select), which the C
// build's plain cvttss2si never does.

const GRID: f32 = 512.0;

/// strength is pre-clamped by the caller to [0.0, 1.0).
fn island_noise(strength: f32, samples: &[f32], out: &mut [i32]) {
    assert!(samples.len() == out.len());
    for (i, (s, slot)) in samples.iter().zip(out.iter_mut()).enumerate() {
        // strength < 1.0 and s in [0.0, GRID) => (s * strength) < GRID
        // => the truncated value fits i32 by construction.
        let v = s * strength.clamp(0.0, 1.0) + (i as f32) / GRID;
        *slot = v as i32; // saturating cast: dead saturation machinery per iteration
    }
}

fn main() {
    let samples = [0.0f32, 128.0, 256.0, 511.0];
    let mut out = [0i32; 4];
    island_noise(0.5, &samples, &mut out);
    assert_eq!(out, [0, 64, 128, 255]);
}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `saturating-f2i-cast-proven-range-to-int-unchecked`
// (riir-refine rust_perf corpus, Batch 282) -- the LAW arm.
//
// The SAME loop with the range proof made EXPLICIT at the site: the
// pre-clamp is named, the bound arithmetic is named, and the cast
// becomes `to_int_unchecked` -- one conversion instruction, no
// saturation machinery. The proof obligations are the caller's and
// ride the SAFETY comment: a future edit that breaks the range breaks
// the CONTRACT, not just a bound -- which is why an unproven range
// stays on `as` and why this rule is suggest-only (the healer never
// auto-applies an unsafe rewrite; the paper's own two-gate validation
// = the verify discipline).

const GRID: f32 = 512.0;

/// strength is pre-clamped by the caller to [0.0, 1.0).
fn island_noise(strength: f32, samples: &[f32], out: &mut [i32]) {
    assert!(samples.len() == out.len());
    let strength = strength.clamp(0.0, 1.0); // the clamp IS the range proof
    for (i, (s, slot)) in samples.iter().zip(out.iter_mut()).enumerate() {
        // INVARIANT: 0.0 <= s < GRID, 0.0 <= strength <= 1.0
        // => 0.0 <= v < GRID + 1.0 < i32::MAX (truncation fits).
        let v = s * strength + (i as f32) / GRID;
        debug_assert!((0.0..GRID + 1.0).contains(&v));
        // SAFETY: v is finite and its truncated value is in [0, 513),
        // proven by the invariant above + the debug_assert.
        *slot = unsafe { v.to_int_unchecked::<i32>() };
    }
}

fn main() {
    let samples = [0.0f32, 128.0, 256.0, 511.0];
    let mut out = [0i32; 4];
    island_noise(0.5, &samples, &mut out);
    assert_eq!(out, [0, 64, 128, 255]);
}
"#
            .to_string(),
            context_notes: Some("Grade B - NotBenchable on this box (every figure is PAPER-reported on their benchmark - i9-10980XE, clang-17 vs rustc 1.77.0-nightly, both LLVM 17.0.6, paired interleaved runs CV<=3% - provenance, never bars; the decision content is the SATURATION-AS-DEAD-WORK law + the proof-obligation discipline). THE CLASS, paper verbatim (Table IV, class 2): 'Saturating float-to-integer conversions remain even when range invariants prove them unnecessary. [...] Optimized Rust IR uses llvm.fptosi.sat or llvm.fptoui.sat for range-bounded operands, whereas C IR uses fptosi or fptoui. | Avoid unnecessary saturation with to_int_unchecked for finite operands whose values, truncated toward zero, fit the target integer type.' THE INSTANCE (paper §IV-C): 'In heman, seven such conversions originate from fastFloor after inlining into heman_internal_generate_island_noise. [...] The operands are finite grid coordinates whose truncated values remain within the target integer range. Once these conditions are established, to_int_unchecked eliminates the redundant saturation.' THE LAW: Rust's `as` float-to-int cast ALWAYS saturates (since 1.45: NaN -> 0, out-of-range -> clamp to MIN/MAX), and that semantic is compiled as real machinery - a compare/select chain or the fptosi.sat intrinsic - executed on EVERY evaluation; when the call site PROVES the range (a preceding clamp, bounded index arithmetic, an assert), the machinery is dead work the equivalent C never paid. THE PROOF OBLIGATIONS (the optimization card, paper §V-D): (1) the operand is FINITE (no NaN/inf - a clamp to a finite range proves it), (2) the value truncated toward zero fits the target integer type. UNSAFE-DISCLOSURE (healer divergence guard): to_int_unchecked is `unsafe` and violating the contract is UB - this rule is SUGGEST-ONLY BY CONSTRUCTION (no proposer exists; the healer never auto-applies it), and any manual application carries the SAFETY comment with the named invariant + a debug_assert at the site, the paper's own precondition-exclusion discipline ('Cards with unmet or unresolved preconditions are excluded'). REGIME: hot loops converting float to int where the range proof exists at the site (grid/coordinate math with clamps, noise generation, quantize ladders); per-op the win is the saturation compare/select chain (paper: the heman noise loop carried SEVEN such conversions); cold paths and unproven ranges stay on `as` (the branch predictor hides a cold saturating cast; hunting proofs for one-shot casts is churn). ANTI-REGIME: the operand range is NOT provable at the site (user input, network data) - saturation is then the CORRECT semantic, not dead work; the cast is `as usize` for INDEXING (different hazard: usize-index-to-u32 owns index width; a wrapping/unchecked index conversion is a memory-safety question, not this law); f64→smaller-int where the range genuinely exceeds the target (banker-scale math). DISTINCT-FROM (grep-verified in this corpus): f32-counter-to-int (entries_01) owns the ACCUMULATOR-TYPE axis (f32 sequential dependency vs u64 associativity - a different mechanism entirely, the fix there changes the accumulator, not the cast); usize-index-to-u32 owns index WIDTH; the clippy_lints unnecessary_cast proposer owns redundant cast REMOVAL (never introduces unsafe); upstream clippy has NO lint suggesting to_int_unchecked (cast_possible_truncation warns but its fix is never unsafe). Fixtures ORIGINAL (the heman island-noise shape re-authored as a plain-Rust stand-in; no project code pasted). CITE: arXiv 2610.23044v1 (Yuan, Wang, Zhou, Liu, Peng - Fudan University + Nanyang Technological University, submitted 2026-10-16) - Table IV class 2 + §IV-C the heman fastFloor instance + §V-D the optimization-card precondition discipline + §VI-A the two-gate validation (incremental 1.0% hot-function-weighted + cumulative 0.3% aggregate + severe-regression guard, paired interleaved runs CV<=3%, Tukey 1.5x IQR) - the same two-gate shape as the corpus rust_perf_bench Pareto + the Issue-940 determinism fold-order pins. Batch 282 / Plan 286 / Research 319.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── dispatch: a loop re-resolves an INVARIANT handler/mode every
        // iteration ------------------------------------------------------
        // Batch 282 / arXiv 2610.23044v1 class 5 (`Repeated Invariant
        // Dispatch`: 'A loop repeatedly invokes an invariant callback,
        // e.g., cb.expect(...)(args), or branches on an invariant mode' -
        // the c2rust callback-pointer shape; Rust-native exposure is the
        // Option-handler / mode-cell re-read the optimizer must treat as
        // volatile across the opaque indirect call).
        PerfRule {
            rule_id: "loop-invariant-dispatch-resolution-hoist".to_string(),
            name: "a hot loop re-resolves a loop-INVARIANT dispatch on every iteration - an Option handler re-unwrapped (expect/unwrap per iteration), a handler re-read out of a mutable cell or &mut field, or a mode flag re-branched - because the resolution sits on the same critical path as the opaque indirect call the optimizer must assume clobbers it; resolve ONCE before the loop (the checked unwrap with its panic path happens once, the cell read happens once) and the loop body is the bare indirect call - the invariant-mode SPECIALIZED-LOOP face (one hoisted branch, two loop bodies) is the same law where the branch is the loop's shape, disclosed manual"
                .to_string(),
            category: "dispatch".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `loop-invariant-dispatch-resolution-hoist`
// (riir-refine rust_perf corpus, Batch 282) -- the ANTI-PATTERN arm.
//
// Re-authored shape from arXiv 2610.23044v1 Table IV class 5 (the
// c2rust `cb.expect(...)(args)` callback shape, cited as a shape, not
// pasted): a streaming pipeline resolves its handler INSIDE the loop.
// The handler is loop-invariant -- nothing in the body reassigns it --
// but the resolver sits behind `&mut self`, and the opaque indirect
// call forces the optimizer to re-load and re-check it every iteration
// (the expect panic path + the field re-read ride the hot path).

struct Pipeline {
    handler: Option<fn(u64) -> u64>,
}

impl Pipeline {
    fn run(&mut self, xs: &[u64]) -> u64 {
        let mut acc = 0u64;
        for &x in xs {
            // re-resolved EVERY iteration: the checked unwrap + the
            // &mut-field re-read both survive into optimized IR because
            // the indirect call may clobber anything address-taken.
            let f = self.handler.expect("handler required");
            acc = f(acc ^ x);
        }
        acc
    }
}

fn main() {
    let mut p = Pipeline { handler: Some(|x| x.wrapping_mul(31)) };
    let xs = [1u64, 2, 3];
    let _ = p.run(&xs); // shape only
}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `loop-invariant-dispatch-resolution-hoist`
// (riir-refine rust_perf corpus, Batch 282) -- the LAW arm.
//
// The resolution happens ONCE before the loop: the expect panic path
// runs once (or never), the field is read once, and the loop body is
// the bare indirect call over a register-resident handler. Behavior is
// identical EXCEPT for the abort-on-None timing (the panic fires
// before any work instead of after the first element's handler call
// -- for an invariant handler this is strictly more honest: the paper's
// own shape aborts the batch, not the element).

struct Pipeline {
    handler: Option<fn(u64) -> u64>,
}

impl Pipeline {
    fn run(&mut self, xs: &[u64]) -> u64 {
        // resolve ONCE: the check + the read leave the hot loop.
        let f = self.handler.expect("handler required");
        let mut acc = 0u64;
        for &x in xs {
            acc = f(acc ^ x);
        }
        acc
    }
}

fn main() {
    let mut p = Pipeline { handler: Some(|x| x.wrapping_mul(31)) };
    let xs = [1u64, 2, 3];
    let _ = p.run(&xs); // shape only
}
"#
            .to_string(),
            context_notes: Some("Grade B - NotBenchable on this box (the source figures are paper-reported cross-project; the per-iteration cost is the checked-unwrap branch + the address-taken field re-load that the opaque indirect call's clobber assumption re-executes - provenance, never bars; the decision content is the INVARIANT-RESOLUTION law). THE CLASS, paper verbatim (Table IV, class 5): 'Loops dispatch on callbacks or mode values that remain invariant across iterations. | A loop repeatedly invokes an invariant callback, e.g., cb.expect(...)(args), or branches on an invariant mode. | Resolve loop-invariant callback or mode dispatch once before the loop, then execute a specialized loop for the selected target or mode.' WHY THE COMPILER DOES NOT DO IT HERE: the resolution reads through `&mut self` (or a cell) and the loop body contains an OPAQUE INDIRECT CALL - LLVM must assume the call clobbers any address-taken memory, so the handler re-load and the expect check re-execute every iteration; hoisting the resolution MOVES the load and the panic path out of the loop, and the paper's cross-project data shows the pattern recurring across projects (every one of the nine rules contributed accepted rewrites on the held-out set). THE MECHANICAL FACE (this fixture): Option-handler resolution - `let f = self.handler.expect(...)` hoisted above the loop; behavior delta disclosed: the panic timing moves BEFORE any element work (strictly more honest for an invariant handler). THE MANUAL FACE (disclosed, not fixable by a parse-only proposer): the invariant-MODE branch - `for x in xs { if self.mode == Mode::A { a_path(x) } else { b_path(x) } }` specializes into a hoisted mode test + TWO loop bodies (the specialized-loop direction); that duplicates the loop and is a semantic restructure - the divergence-prone class, manual with the paper's validation gate. REGIME: hot loops whose per-iteration dispatch resolution is invariant and reads through mutable state (config-selected handlers, codec pipelines, per-batch mode selection resolved per-item by habit); composes with the dispatch family - after hoisting, a REMAINING data-dependent dispatch may still benefit from the family's other members. ANTI-REGIME: the dispatch is genuinely per-iteration (the handler changes mid-loop - then the re-resolution IS the semantic); the resolution is a static table already hoisted (dispatch-handler-pair-hoist's territory); the loop is cold. DISTINCT-FROM (grep-verified in this corpus): dispatch-handler-pair-hoist (entries_05) owns the RUNTIME-TABLE face (two static-table entries re-indexed per iteration, hoisted to register-resident selects - the items are compile-time known, the SELECT is the fix) - THIS rule owns the RESOLUTION face (the Option check + the mutable-cell read, the check itself is the per-iteration cost); keyword-map-to-match (entries_03) owns map-lookup-to-match SPECIALIZATION at build time; dyn-to-enum (entries_01) owns the trait-object eligibility axis; division-by-invariant-multiplication owns the arithmetic-cleanup cousin (an invariant DIVISION is strength-reduced - this rule's invariant DISPATCH is resolution-hoisted; same 'invariant on the hot path' genus, different operator). Fixtures ORIGINAL (the c2rust callback shape re-authored as a plain-Rust stand-in; no project code pasted). KINSHIP (Batch 305, XiangpengHao/bbtree design/03-decisions.md D28, MIT OR Apache-2.0, quote verified at the pin 8adae97a): 'Branch early, loop late: a node's kind is decided once per step.' - the ordered-structure cousin of this resolution-hoist law: the leaf/internal kind decision is made ONCE where the page is read (the edge tag decides), never re-derived inside the search loop; the source measured a descent deciding it three to six times per level and a steal or merge five to eight times when the decision lived in each method instead. Same genus (an invariant resolution hoisted off the hot path), different payload (a kind dispatch vs this rule's Option-handler/mode resolution). CITE: arXiv 2610.23044v1 (Yuan, Wang, Zhou, Liu, Peng - Fudan University + Nanyang Technological University, submitted 2026-10-16) - Table IV class 5 + §V-D the optimization-card precondition discipline + §VII-B (every rule contributed accepted rewrites across the 12 held-out projects). Batch 282 / Plan 286 / Research 319.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── iterators: raw-pointer traversal hides bounds + aliasing from
        // the optimizer ---------------------------------------------------
        // Batch 282 / arXiv 2610.23044v1 class 9 (`Obscured Memory Access
        // Properties` - the paper's MOST WIDESPREAD rule: 89% of hot
        // functions across all 12 held-out projects, contributing to
        // accepted rewrites in 11; the libzahl zrsh instance carried 73 vs
        // 24 static loads and missing TBAA, and RQ5 found 46 of 62 accepted
        // rewrites were exactly this Memory Property Recovery class).
        PerfRule {
            rule_id: "raw-pointer-traversal-slice-view-recovery-load-elimination".to_string(),
            name: "a hot loop traverses buffers through raw-pointer arithmetic (ptr.add(i) / ptr.offset(i) derefs) with a separately tracked length - bounds and aliasing relationships stay IMPLICIT, so LLVM's GVN/LICM cannot prove loads independent or hoist them (redundant loads survive, invariant loads re-load, TBAA-style facts are absent) - recover a &[T]/&mut [T] slice view over the traversed extent and iterate it, exposing extent + noalias so load elimination and hoisting fire; where the exclusivity/extent proof cannot be established the raw-pointer form stays, and the SAFE fallback half of the law is hoisting the loop-invariant loads by hand"
                .to_string(),
            category: "iterators".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `raw-pointer-traversal-slice-view-recovery-load-elimination`
// (riir-refine rust_perf corpus, Batch 282) -- the ANTI-PATTERN arm.
//
// Re-authored shape from arXiv 2610.23044v1 §IV-C (the libzahl `zrsh`
// limb-shift instance, cited as a shape, not pasted): a big-integer
// shift routine walks its limb arrays through raw-pointer offsets with
// the length tracked separately. Bounds are implicit (the caller's
// problem), aliasing is implicit (two loose pointers), and the
// optimizer's GVN/LICM cannot tie the loads together - the paper
// measured the class at 73 static load instructions in optimized Rust
// IR vs 24 in C, with the !tbaa metadata missing and 46 GVN
// missed-optimization remarks citing potentially-aliasing stores.

/// dst and src must not alias; dst.len() >= src.len() - off.
///
/// # Safety
/// Both pointers must be valid for reads/writes of `n` u64s.
unsafe fn shift_limbs_raw(dst: *mut u64, src: *const u64, n: usize, off: usize) {
    for i in 0..n {
        // raw-pointer traversal: bounds + aliasing implicit to the optimizer
        *dst.add(i) = *src.add(i + off);
    }
}

fn main() {
    let src = [1u64, 2, 3, 4];
    let mut dst = [0u64; 3];
    // SAFETY: disjoint stack arrays, lengths respected (n=3, off=1).
    unsafe { shift_limbs_raw(dst.as_mut_ptr(), src.as_ptr(), 3, 1) };
    assert_eq!(dst, [2, 3, 4]);
}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `raw-pointer-traversal-slice-view-recovery-load-elimination`
// (riir-refine rust_perf corpus, Batch 282) -- the LAW arm.
//
// The SAME traversal over recovered slice views: the function
// signature now CARRIES the extent (the slices' lengths) and Rust's
// reference aliasing rules carry the no-alias fact, so GVN can
// eliminate redundant loads and LICM can hoist invariants. The unsafe
// surface shrinks to the boundary where the views are CONSTRUCTED (or
// vanishes entirely when the caller already holds slices) - the
// paper's safety finding: slice recovery reduced pointer-arithmetic
// operations 18% and raw-pointer dereferences 10% while widening the
// safe-callable interface.

/// dst and src must not alias; the lengths ARE the contract now
/// (zip stops at the shorter side - the extent is structural, not a
/// caller promise).
fn shift_limbs_slices(dst: &mut [u64], src: &[u64], off: usize) {
    // the zip is structural: the extent is the TYPES, the bounds checks
    // vanish, and Rust's reference rules hand LLVM the no-alias fact.
    for (d, &s) in dst.iter_mut().zip(src[off..].iter()) {
        *d = s;
    }
}

fn main() {
    let src = [1u64, 2, 3, 4];
    let mut dst = [0u64; 3];
    shift_limbs_slices(&mut dst, &src, 1);
    assert_eq!(dst, [2, 3, 4]);
}
"#
            .to_string(),
            context_notes: Some("Grade B+ - NotBenchable on this box (every figure is PAPER-reported: the 73-vs-24 static-load IR comparison, the 46-vs-16 GVN remarks, the 89%-of-hot-functions cross-project prevalence, RQ5's 46-of-62 accepted rewrites in this class, and the 18%/10% pointer-op reductions - provenance, never bars; the decision content is the ACCESS-PROPERTY-EXPOSURE law, the corpus's strongest-evidence external validation of the iterator/slice family). THE CLASS, paper verbatim (Table IV, class 9): 'Insufficiently exposed access relationships and invariants inhibit load elimination or hoisting. | A loop accesses buffers through raw pointers (e.g., ptr.add(i)), with separately tracked bounds and implicit aliasing relationships. | Recover valid &[T]/&mut [T] views to expose access properties to LLVM, or hoist loop-invariant loads.' THE INSTANCE (paper §IV-C): 'In libzahl, zrsh traverses limb arrays through raw-pointer accesses such as *ptr.offset(i). The optimized Rust IR contains 73 static load instructions, compared with 24 in C, and lacks the !tbaa metadata present in the C IR. Consistently, GVN emits 46 missed-optimization remarks for Rust, versus 16 for C, frequently citing potentially aliasing writes as barriers to load elimination.' THE LAW: raw-pointer derefs carry NO extent and NO aliasing promise - every load is a fresh, potentially-clobbering read the optimizer must keep; a slice/reference view carries the extent structurally (the length IS the type) and the uniqueness legally (&mut T excludes conflicting accesses for its lifetime), so GVN eliminates the redundant loads and LICM hoists the invariants. THE PROOF OBLIGATIONS (the paper's optimization card for this class, §V-D): establish the buffer EXTENT, keep every access within it, and exclude CONFLICTING ACCESS while the mutable view is live - exactly the three obligations a recovered &mut [T] view makes structural. SAFETY-DISCLOSURE (healer divergence guard): constructing the view from a raw pointer is the unsafe step (slice::from_raw_parts / from_raw_parts_mut) and a WRONG extent or a violated exclusivity claim is immediate UB - this rule is SUGGEST-ONLY BY CONSTRUCTION (no proposer exists); manual applications prefer recovering the view at the SIGNATURE boundary (take &[T] parameters, as this fixture does - the unsafe surface vanishes) over from_raw_parts construction inside the body, and the SAFE fallback half of the law is hoisting the loop-invariant loads by hand when the view cannot be recovered. REGIME: hot loops over buffers behind raw-pointer parameters (FFI boundaries, allocator internals, big-number limb math, device staging) where an extent is already tracked somewhere; Rust-native code reaching for ptr.add where a slice would do. ANTI-REGIME: the traversal genuinely interleaves UNRELATED pointers the type system cannot relate (alias the slices would forbid - then the raw form is the CORRECT representation and the hoist-by-hand half applies); the loop is cold; the pointer arithmetic is the FFI wire contract itself. DISTINCT-FROM (grep-verified in this corpus): encoded-layout-borrowed-view owns the ENCODING axis (a borrowed view over an encoded buffer's layout - not access-property exposure); partitioned-mut-slices-zero-sync owns the multi-thread PARTITION axis; eager-pointer-collect-lazy-fetch owns a fetch-strategy axis; chunks-exact + the iterator family own the SLICE-ITERATION HOW - this rule owns the WHY (the optimizer-visible access properties the view restores), and the Batch-282 face added to chunks-exact carries the bounds-check-elimination connection. Fixtures ORIGINAL (the libzahl zrsh shape re-authored as a plain-Rust stand-in; no project code pasted). CITE: arXiv 2610.23044v1 (Yuan, Wang, Zhou, Liu, Peng - Fudan University + Nanyang Technological University, submitted 2026-10-16) - Table IV class 9 + §IV-C the libzahl zrsh instance + §V-B2 the Local Pointer-View Recovery refinement + §V-D the optimization card (extent/within/conflict-free) + §VII-C the safety findings (unsafe-fn share 99.8% -> 60.5%, pointer arithmetic -18%, derefs -10%) + §VII-B the 89% prevalence. Batch 282 / Plan 286 / Research 319.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
