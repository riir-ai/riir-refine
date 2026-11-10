//! Bench 006 — the M3-Max GOAT for the rust_perf rule
//! `repr-align-register-partition-store-forwarding` (Batch 211, the
//! bevyengine/bevy #26042 EntityLocation distill + the rust-lang/rust
//! #163916 alignment-drop rider).
//!
//! The rule is NotBenchable ON THIS BOX as a corpus rule (every figure in
//! its context_notes is a FOREIGN source-reported bench — provenance,
//! never bars), and the corpus cannot price it with a generated bench_spec:
//! the claim is about LLVM's register PARTITION of an aggregate load/store
//! pair, not about any local algorithm. The staged local evidence (Batch
//! 211's queue note) is THIS file: the Plan 204 T11 pattern — a
//! hand-written A/B of the rule's own fixture shapes, align-4 vs
//! align(8), everything else byte-identical code.
//!
//! What is measured, per the rule's own law:
//!
//! - **Shape F (the rule's fixture, verbatim transcription)** —
//!   `store_then_key`: push the 4x u32 location into a Vec (the 8+4+4
//!   store partition) then compute `cell_key` (the joined (row,
//!   table_row) read). One function, both struct forms.
//! - **Shape C (the bevy construction loop)** — locations loaded from a
//!   table, stored into a destination aggregate (a distinct type, same
//!   4-field shape — the UnsafeEntityCell stand-in), keys computed from
//!   the loaded location. This is the PR's "hit on nearly every entity
//!   access" path: aggregate store construction + partial-field join.
//!
//! The A/B: the SAME bodies generated twice, the ONLY difference the
//! `#[repr(align(8))]` attribute (the rule's one-line fix). No arch
//! intrinsics — portable Rust — but the claim is LLVM+arm64 codegen, so
//! the citable run is THIS box (M3 Max, the Plan-204 T11 lane).
//!
//! Gates:
//!
//! 1. **G1 (bit-identity)** — both forms produce IDENTICAL key vectors
//!    over the same fixture table (the attribute must be a pure codegen
//!    change; any output difference is a bench defect, panic).
//! 2. **G4 (zero-alloc)** — the counting allocator armed around both
//!    measured loops: zero allocations (the vectors are pre-allocated to
//!    the exact length; a push realloc would be a bench bug).
//! 3. **G2 (interleaved best-of-N, provenance-gated)** — min-of-5
//!    interleaved passes per shape per arm (the sequential-A/B box-noise
//!    lesson), correctness-coupled sinks (each arm's key sum must equal
//!    the G1 reference, so a deleted loop FAILS instead of reporting a
//!    fast zero). Every citable number quotes the `bench_preflight.sh`
//!    PROVENANCE line; `REPR_ALIGN_BENCH_REQUIRE_PROVENANCE=1` is the
//!    exit-2 gate posture for the citable run.
//!
//! Verdict vocabulary (honest to the rule's status — it is LANDED and
//! source-reported; this bench does not gate a landing, it gathers the
//! LOCAL evidence the corpus annotation says does not exist yet):
//!
//! - gates pass + the align arm strictly faster at >= 1 shape with NO
//!   shape slower beyond noise (>= 1% behind its pair) = `REPRODUCED`.
//! - gates pass otherwise = `NOT-REPRODUCED-LOCALLY` — a real measured
//!   outcome: the local fixtures do not exhibit the partition stall on
//!   this LLVM/toolchain, and the rule stays source-reported (the
//!   anti-regime clause already says it: an attribute without a failing
//!   benchmark is speculative layout churn — measure first). Exit 0 BOTH
//!   ways; the record carries the numbers either way.
//!
//! ```bash
//! # the whole gate (G1 + G4 + provisional-labeled G2):
//! cargo bench --bench bench_006_repr_align_m3_goat
//! # the GATE posture (exit 2 unless preflight PASSED — the citable run):
//! REPR_ALIGN_BENCH_REQUIRE_PROVENANCE=1 cargo bench --bench bench_006_repr_align_m3_goat
//! ```

#![allow(clippy::pedantic)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

// ── the two struct forms (the A/B is the ATTRIBUTE and nothing else) ─────

/// The rule's buggy_code form: natural align 4.
#[derive(Clone, Copy)]
struct LocAlign4 {
    row: u32,
    table_row: u32,
    archetype: u32,
    slot: u32,
}

/// The rule's fixed_code form: the ONE attribute.
#[derive(Clone, Copy)]
#[repr(align(8))]
struct LocAlign8 {
    row: u32,
    table_row: u32,
    archetype: u32,
    slot: u32,
}

const _: () = {
    assert!(std::mem::size_of::<LocAlign4>() == 16);
    assert!(std::mem::size_of::<LocAlign8>() == 16);
    assert!(std::mem::align_of::<LocAlign4>() == 4);
    assert!(std::mem::align_of::<LocAlign8>() == 8);
};

// ── the rule's fixture functions, generated per form (identical bodies) ──
// Hand-written TWICE, not macro-generated: the textual duplication IS the
// A/B (a reviewer sees the bodies are identical and the ONLY difference is
// the struct's attribute). The B202 macro-parse-trap precedent says the
// hour is better spent elsewhere.

mod a4 {
    use super::LocAlign4;

    pub fn cell_key(loc: LocAlign4) -> u64 {
        ((loc.row as u64) << 32) | loc.table_row as u64
    }

    /// The rule's `store_then_key`: the partitioned store, then the
    /// joined partial-field read.
    pub fn store_then_key(locs: &mut Vec<LocAlign4>, loc: LocAlign4) -> u64 {
        locs.push(loc); // the aggregate store
        cell_key(loc) // the joined read
    }

    /// Shape F driver: the fixture driven N times over a sink Vec
    /// sized to N (push never reallocs — G4's precondition).
    pub fn shape_f(table: &[LocAlign4], dst: &mut Vec<LocAlign4>) -> u64 {
        dst.clear();
        let mut acc = 0u64;
        for (i, &loc) in table.iter().enumerate() {
            let k = store_then_key(dst, loc);
            acc ^= k.rotate_left((i & 31) as u32);
        }
        acc
    }

    /// Shape C driver: the bevy construction loop — the aggregate
    /// store built FROM A TABLE LOAD (the 4+8+4 vs 4+4+8 partition
    /// question), then the joined read. The destination type is a
    /// DISTINCT aggregate with the same 4-field shape (the
    /// UnsafeEntityCell stand-in) so the store cannot collapse into
    /// the load's registers.
    #[derive(Clone, Copy)]
    pub struct Cell {
        pub row: u32,
        pub table_row: u32,
        pub archetype: u32,
        pub slot: u32,
    }

    pub fn shape_c(table: &[LocAlign4], dst: &mut Vec<Cell>) -> u64 {
        dst.clear();
        let mut acc = 0u64;
        for (i, &loc) in table.iter().enumerate() {
            dst.push(Cell {
                row: loc.row,
                table_row: loc.table_row,
                archetype: loc.archetype,
                slot: loc.slot,
            });
            acc ^= cell_key(loc).rotate_left((i & 31) as u32);
        }
        acc
    }
}

mod a8 {
    use super::LocAlign8;

    pub fn cell_key(loc: LocAlign8) -> u64 {
        ((loc.row as u64) << 32) | loc.table_row as u64
    }

    pub fn store_then_key(locs: &mut Vec<LocAlign8>, loc: LocAlign8) -> u64 {
        locs.push(loc); // the aggregate store
        cell_key(loc) // the joined read
    }

    pub fn shape_f(table: &[LocAlign8], dst: &mut Vec<LocAlign8>) -> u64 {
        dst.clear();
        let mut acc = 0u64;
        for (i, &loc) in table.iter().enumerate() {
            let k = store_then_key(dst, loc);
            acc ^= k.rotate_left((i & 31) as u32);
        }
        acc
    }

    #[derive(Clone, Copy)]
    pub struct Cell {
        pub row: u32,
        pub table_row: u32,
        pub archetype: u32,
        pub slot: u32,
    }

    pub fn shape_c(table: &[LocAlign8], dst: &mut Vec<Cell>) -> u64 {
        dst.clear();
        let mut acc = 0u64;
        for (i, &loc) in table.iter().enumerate() {
            dst.push(Cell {
                row: loc.row,
                table_row: loc.table_row,
                archetype: loc.archetype,
                slot: loc.slot,
            });
            acc ^= cell_key(loc).rotate_left((i & 31) as u32);
        }
        acc
    }
}

// ── the counting allocator (G4) ──────────────────────────────────────────

static ALLOC_ARM: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ALLOC_ARM.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ALLOC_ARM.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL_ALLOC: Counting = Counting;

fn alloc_armed<T>(f: impl FnOnce() -> T) -> (T, usize) {
    ALLOC_COUNT.store(0, Ordering::Relaxed);
    ALLOC_ARM.store(true, Ordering::Relaxed);
    let out = f();
    ALLOC_ARM.store(false, Ordering::Relaxed);
    (out, ALLOC_COUNT.load(Ordering::Relaxed))
}

// ── the deterministic fixture generator (identical on every host) ────────

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }
    /// xorshift64* — deterministic, no deps.
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

const FIXTURE_SEED: u64 = 0x2110_0600_C0DE_0001;

/// The fixture table: 4x u32 locations, full-range fields (the partition
/// question is structural, not value-dependent, but full-range values keep
/// every field load live).
fn gen_table(n: usize, seed_offset: u64) -> Vec<LocAlign4> {
    let mut rng = Rng::new(FIXTURE_SEED ^ seed_offset);
    (0..n)
        .map(|_| LocAlign4 {
            row: rng.next() as u32,
            table_row: rng.next() as u32,
            archetype: rng.below(1 << 20) as u32,
            slot: rng.below(1 << 12) as u32,
        })
        .collect()
}

/// Transmute-free mirror of a LocAlign4 table into LocAlign8 (identical
/// field bytes — G1 requires both arms to see identical DATA).
fn mirror_table(table: &[LocAlign4]) -> Vec<LocAlign8> {
    table
        .iter()
        .map(|l| LocAlign8 {
            row: l.row,
            table_row: l.table_row,
            archetype: l.archetype,
            slot: l.slot,
        })
        .collect()
}

// ── provenance (the bench_005 pattern — the instrument is cross-repo) ────

fn preflight() -> (String, String) {
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../riir-reflex/scripts/bench_preflight.sh");
    if !script.exists() {
        return (
            "UNAVAILABLE".to_string(),
            format!(
                "preflight script absent at {} — provenance never skipped",
                script.display()
            ),
        );
    }
    match Command::new(&script).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let prov = stdout
                .lines()
                .find(|l| l.starts_with("PROVENANCE:"))
                .map(|l| l.to_string())
                .unwrap_or_else(|| {
                    stdout
                        .lines()
                        .find(|l| l.contains("REFUSED") || l.contains("FAIL"))
                        .unwrap_or("preflight refused (no reason line parsed)")
                        .to_string()
                });
            let verdict = if out.status.success() && prov.starts_with("PROVENANCE:") {
                "PASSED".to_string()
            } else {
                "REFUSED".to_string()
            };
            (verdict, prov)
        }
        Err(e) => (
            "UNAVAILABLE".to_string(),
            format!("preflight spawn failed: {e}"),
        ),
    }
}

fn say(msg: &str) {
    println!("{msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

/// Best-of-N total-wall passes, interleaved per shape (the sequential-A/B
/// box-noise lesson: contention adds time to SOME runs; the minimum of
/// interleaved pairs separates it).
const MEASURED_PASSES: usize = 5;

/// The sweep: table sizes around the bevy hot shapes (their world_entity
/// bench builds 50k entities; the mut-slice shapes touch 20-200). 4k keeps
/// each pass sub-millisecond-fast while still ~4096 store+read pairs; 64k
/// gives the loop enough body to steady-state.
const TABLE_GRID: [usize; 4] = [4_096, 16_384, 32_768, 65_536];
const WARMUP_PASSES: usize = 2;

fn main() {
    say("── bench_006: the repr(align(8)) store-forwarding M3-Max GOAT (Batch 211) ──");

    // ── provenance (before any timing) ───────────────────────────────────
    let (prov_verdict, prov_line) = preflight();
    say(&format!("provenance: {prov_verdict} — {prov_line}"));
    if prov_verdict != "PASSED"
        && std::env::var("REPR_ALIGN_BENCH_REQUIRE_PROVENANCE").is_ok_and(|v| v == "1")
    {
        say(
            "GATE POSTURE: provenance not PASSED and REPR_ALIGN_BENCH_REQUIRE_PROVENANCE=1 — exit 2",
        );
        std::process::exit(2);
    }
    let citable = prov_verdict == "PASSED";

    // ══ G1 — bit-identity: the attribute is a pure codegen change ══
    say("── G1: identical key vectors across the attribute ──");
    {
        let table4 = gen_table(4_096, 0xBEEF);
        let table8 = mirror_table(&table4);
        let mut dst4: Vec<LocAlign4> = Vec::with_capacity(table4.len());
        let mut dst8: Vec<LocAlign8> = Vec::with_capacity(table8.len());
        let mut cells4: Vec<a4::Cell> = Vec::with_capacity(table4.len());
        let mut cells8: Vec<a8::Cell> = Vec::with_capacity(table8.len());

        // direct cell_key agreement on every fixture element
        for (a, b) in table4.iter().zip(&table8) {
            let ka = a4::cell_key(*a);
            let kb = a8::cell_key(*b);
            assert_eq!(
                ka, kb,
                "G1 FAIL: cell_key diverges at row={} table_row={}",
                a.row, a.table_row
            );
        }
        // shape F driver agreement
        let s4 = a4::shape_f(&table4, &mut dst4);
        let s8 = a8::shape_f(&table8, &mut dst8);
        assert_eq!(s4, s8, "G1 FAIL: shape_f acc diverges ({s4} vs {s8})");
        assert_eq!(dst4.len(), dst8.len(), "G1 FAIL: dst lengths diverge");
        // shape C driver agreement (field-by-field on the stored cells)
        let c4 = a4::shape_c(&table4, &mut cells4);
        let c8 = a8::shape_c(&table8, &mut cells8);
        assert_eq!(c4, c8, "G1 FAIL: shape_c acc diverges ({c4} vs {c8})");
        for (a, b) in cells4.iter().zip(&cells8) {
            assert_eq!(
                (a.row, a.table_row, a.archetype, a.slot),
                (b.row, b.table_row, b.archetype, b.slot),
                "G1 FAIL: stored Cell fields diverge"
            );
        }
        say(
            "G1 PASS — cell_key + shape_f + shape_c bit-identical across the attribute (4096 fixtures, cells field-checked)",
        );
    }

    // ══ G4 — zero allocations in the measured loops ══
    say("── G4: zero allocations in the measured paths ──");
    {
        let table4 = gen_table(4_096, 0xCAFE);
        let table8 = mirror_table(&table4);
        let mut dst4: Vec<LocAlign4> = Vec::with_capacity(table4.len());
        let mut dst8: Vec<LocAlign8> = Vec::with_capacity(table8.len());
        let mut cells4: Vec<a4::Cell> = Vec::with_capacity(table4.len());
        let mut cells8: Vec<a8::Cell> = Vec::with_capacity(table8.len());
        let (_, n) = alloc_armed(|| a4::shape_f(&table4, &mut dst4));
        assert_eq!(n, 0, "G4 FAIL: shape_f/align4 allocates ({n})");
        let (_, n) = alloc_armed(|| a8::shape_f(&table8, &mut dst8));
        assert_eq!(n, 0, "G4 FAIL: shape_f/align8 allocates ({n})");
        let (_, n) = alloc_armed(|| a4::shape_c(&table4, &mut cells4));
        assert_eq!(n, 0, "G4 FAIL: shape_c/align4 allocates ({n})");
        let (_, n) = alloc_armed(|| a8::shape_c(&table8, &mut cells8));
        assert_eq!(n, 0, "G4 FAIL: shape_c/align8 allocates ({n})");
    }
    say("G4 PASS — zero allocations, both arms, both shapes (pre-sized sinks)");

    // ══ G2 — the A/B sweep, interleaved best-of-N ══
    say("── G2: align-4 vs align-8 sweep (table sizes), interleaved best-of-N ──");
    say(&format!(
        "    (interleaved best-of-{MEASURED_PASSES} per shape per arm, {WARMUP_PASSES} warmup passes; correctness-coupled sinks)"
    ));
    let mut shapes_strict_win = 0usize;
    let mut shapes_behind = 0usize;

    for &n in &TABLE_GRID {
        let table4 = gen_table(n, n as u64);
        let table8 = mirror_table(&table4);
        let mut dst4: Vec<LocAlign4> = Vec::with_capacity(n);
        let mut dst8: Vec<LocAlign8> = Vec::with_capacity(n);
        let mut cells4: Vec<a4::Cell> = Vec::with_capacity(n);
        let mut cells8: Vec<a8::Cell> = Vec::with_capacity(n);

        // correctness-coupled sinks: the G1 reference at this size; both
        // arms must reproduce it inside the timed loop or the bench FAILS
        // (a deleted loop reports a fast wrong number, never a pass).
        let ref_a4 = a4::shape_f(&table4, &mut dst4);
        let ref_a8 = a8::shape_f(&table8, &mut dst8);
        assert_eq!(ref_a4, ref_a8, "G2 sink mismatch (shape_f) at n={n}");
        let ref_c4 = a4::shape_c(&table4, &mut cells4);
        let ref_c8 = a8::shape_c(&table8, &mut cells8);
        assert_eq!(ref_c4, ref_c8, "G2 sink mismatch (shape_c) at n={n}");

        // warmup (caches, page faults, allocator steady state)
        for _ in 0..WARMUP_PASSES {
            std::hint::black_box(a4::shape_f(&table4, &mut dst4));
            std::hint::black_box(a8::shape_f(&table8, &mut dst8));
            std::hint::black_box(a4::shape_c(&table4, &mut cells4));
            std::hint::black_box(a8::shape_c(&table8, &mut cells8));
        }

        // interleaved best-of-N, shape F
        let mut best_a4 = f64::INFINITY;
        let mut best_a8 = f64::INFINITY;
        for _ in 0..MEASURED_PASSES {
            let t0 = Instant::now();
            let s = a4::shape_f(&table4, &mut dst4);
            assert_eq!(
                std::hint::black_box(s),
                ref_a4,
                "G2 sink drift (shape_f/align4) at n={n}"
            );
            best_a4 = best_a4.min(t0.elapsed().as_secs_f64());
            let t0 = Instant::now();
            let s = a8::shape_f(&table8, &mut dst8);
            assert_eq!(
                std::hint::black_box(s),
                ref_a8,
                "G2 sink drift (shape_f/align8) at n={n}"
            );
            best_a8 = best_a8.min(t0.elapsed().as_secs_f64());
        }

        // interleaved best-of-N, shape C
        let mut best_c4 = f64::INFINITY;
        let mut best_c8 = f64::INFINITY;
        for _ in 0..MEASURED_PASSES {
            let t0 = Instant::now();
            let s = a4::shape_c(&table4, &mut cells4);
            assert_eq!(
                std::hint::black_box(s),
                ref_c4,
                "G2 sink drift (shape_c/align4) at n={n}"
            );
            best_c4 = best_c4.min(t0.elapsed().as_secs_f64());
            let t0 = Instant::now();
            let s = a8::shape_c(&table8, &mut cells8);
            assert_eq!(
                std::hint::black_box(s),
                ref_c8,
                "G2 sink drift (shape_c/align8) at n={n}"
            );
            best_c8 = best_c8.min(t0.elapsed().as_secs_f64());
        }

        // per-shape verdicts: strict win = align8 faster by > 1%; behind =
        // align8 slower by > 1% (outside the noise floor this box shows
        // between same-code arms)
        for (label, a4, a8) in [("F", best_a4, best_a8), ("C", best_c4, best_c8)] {
            let ratio = a8 / a4;
            let tag = if ratio < 0.99 {
                shapes_strict_win += 1;
                "align8 WINS"
            } else if ratio > 1.01 {
                shapes_behind += 1;
                "align8 BEHIND"
            } else {
                "parity"
            };
            say(&format!(
                "  n={n:>6} shape={label}  align4={:>9.3} us  align8={:>9.3} us  ratio={ratio:>6.4}  {tag}{}",
                a4 * 1e6,
                a8 * 1e6,
                if citable { "" } else { "  (PROVISIONAL)" }
            ));
        }
    }

    // ══ the verdict ══
    say("── VERDICT ──");
    say(&format!(
        "shapes: {shapes_strict_win} align8-win, {shapes_behind} align8-behind, {} parity (of {})",
        TABLE_GRID.len() * 2 - shapes_strict_win - shapes_behind,
        TABLE_GRID.len() * 2
    ));
    if shapes_strict_win > 0 && shapes_behind == 0 {
        say(&format!(
            "REPRODUCED — G1 exact, G4 zero-alloc, align(8) strictly faster at {shapes_strict_win} shape(s), none behind{}",
            if citable {
                ""
            } else {
                " (TIMING PROVISIONAL — quote the PROVENANCE line)"
            }
        ));
    } else {
        say(
            "NOT-REPRODUCED-LOCALLY — the local fixtures do not exhibit the partition stall on this toolchain; a real measured outcome, recorded as such (the rule stays source-reported; its anti-regime clause stands: measure first)",
        );
    }
    say(&format!("PROVENANCE: {prov_line}"));
    std::process::exit(0);
}
