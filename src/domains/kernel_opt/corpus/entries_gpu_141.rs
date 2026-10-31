//! Entries slice 141 — Batch 310 (Plan 315): the 10-30 staged leads
//! addendum (zed-m3-leads1030, rows S1/S2/S4 + the R2 rider face),
//! converted kernel_opt-only. Sources: tracel-ai/cubecl @ 4549408587cf
//! (#1971) + 07cd07a2da9c (#1975), the #1966 series (MIT OR Apache-2.0)
//! — S1 + S2; PrismML-Eng/llama.cpp @ 9d9efeadbdd1 (#415, the b10831
//! scored-tag tip, the llama.cpp MIT family) — S4. The R2 rider face is
//! attached in place on its family home (`gpu_130`). S3 + R1 + R3 are
//! DEFERRED to the `rust_perf` tier behind the B301/plan-306 claim (the
//! R3 home correction — B200 lives in `rust_perf/corpus/entries_13.rs` —
//! is disclosed in the plan + the landing row). Every quote re-verified
//! byte-for-byte at the fetched .patch pins (plan 315 T0, 2026-10-30).
//! The fixtures are ORIGINAL minimal Rust reproductions — nothing
//! transcribed from the sources (Rust/cubecl IR/CUDA). Not benchable:
//! source-reported device figures (RTX PRO 6000) / fork-CUDA on the
//! opponent's tag; the depth pairing, the placement shape, and the
//! fold-order invariances are CPU-testable and ARE the fixtures.

use super::KernelRule;

#[allow(clippy::too_many_lines)]
fn slice_b310_staged_leads() -> Vec<KernelRule> {
    vec![
        // ── Rule 1 (category B — compute/mma-shape): cubecl #1971+#1975,
        // the #1966 series. ONE atom: a self-contracting product's
        // accumulator run is one A operand — the instruction depth must
        // match the run depth, never one accumulator's width. ──
        KernelRule {
            rule_id: "mma-accumulator-run-as-one-operand-depth-matches-instruction".to_string(),
            gpu_concept: "MECHANISM (a self-contracting product's accumulator run is one A operand; the instruction depth matches the run depth; category B — compute/mma-shape): when a product CONTRACTS ITS OWN OUTPUT — an attention's value∘score, where the score the first product made is the operand the second consumes — every accumulator becomes the A operand of the NEXT instruction, and a SINGLE accumulator's width sets that instruction's K-depth. THE SOURCE LAW, verbatim (the #1966 series body): 'A product that contracts its own output, as an attention's value contracts its score, read each accumulator as the A of an instruction as deep as the accumulator is wide: m16n8 scores fed m16n8k8, which runs at half the rate of m16n8k16 on an RTX PRO 6000 (1.97 ns a call per SM for either).' — one accumulator = depth 8 = the k8 instruction, HALF the k16 rate for the same wall time per call. THE REMEDY, verbatim: '`accumulator_as_a` becomes `accumulators_as_a`: a run of accumulators along the contraction, laid one after another into the A operand, which is where the manual-mma layouts put them for 16-bit operands.' — the RUN, not the accumulator, is the unit: laying R consecutive accumulators into A makes the instruction depth = R × accumulator width, so a run of two m16n8 scores feeds m16n8k16 at the full rate with the same bits and the same fold order. The API shape, verbatim: '`Tile::mma_operand` builds that A for a held accumulator (a run of one) and for a one-high grid of them, so `MmaData::mma` loses its accumulator arm. `PlaneTile::mma` contracts a run at the run's depth; every other factor still contracts at the leaf the accumulator was opened at.' — and the consumer side is one statement, verbatim: 'A level that steps the keys a run at a time above the score's leaf is all a kernel states to use it: the register attention test now runs a run of two (m16n8k16 for the value) beside its run of one.' DECISION CONTENT: (a) the depth pairing is the content — an instruction's K-depth must equal the RUN of operands it consumes, never one operand's width, when the operand IS the contraction's own output; (b) the run is a LAYOUT fact, not a copy — the manual-mma A-operand layout for 16-bit operands already lays the accumulators consecutively; staging or repacking to assemble the run is the anti-shape; (c) the run depth is stated at the consumer level (a level steps the keys a run at a time) and everything below contracts at that depth — a run of one stays legal (a held accumulator), the API degrades per-run, never per-kernel; (d) bit-identity is a construction property — the contraction order along k is unchanged; the run changes how many k-elements one instruction eats, never the order the partials fold. NUMBERS HONESTY: the 1.97 ns a call per SM pair is source-reported on an RTX PRO 6000; the law's content is the depth pairing — the rate pair is the evidence, not the fixture. REGIME: self-contracting products (attention value∘score), manual-mma pipelines whose accumulator type feeds the next instruction's A operand, 16-bit operand layouts. ANTI-REGIME: products whose operands are NOT their own output (a normal GEMM's A comes from upstream — no accumulator run exists); consumers reading accumulators separately (merging them into one instruction destroys the per-accumulator results the caller wants); layouts where the accumulators are not consecutive (assemble the run at the layout layer first, then apply this). KINSHIP (do not double-count): mma-accumulate-into-resident-accumulator-c-operand (B205, gpu_75) owns the C-OPERAND alias — accumulation PLACEMENT (a persistent sum aliased as the mma's C input, the temp+add-back pair deleted); THIS owns the A-OPERAND RUN-DEPTH law — instruction-RATE recovery by run depth; the source itself names the boundary ('MmaData::mma' loses its accumulator arm — the B205 accumulator-as-C shape retires at this layer) — compose, never merge (a run-as-A instruction can still accumulate into a resident C); mma-fragment-layout-m16n8k16 (gpu_09) pins the WITHIN-one-instruction fragment register contract — THIS is the ACROSS-accumulator run at the layout level, and the two compose (one instruction's fragments ARE that contract; many accumulators' run IS this rule); c-fragment-to-a-operand-register-repack (the gpu_41 neighbor) moves values ACROSS layouts through registers — THIS needs no move where the layout already lays the run out, and its admission names that layout. GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'accumulator' 71 files — the owners are B205's C-operand alias, the fragment-contract rules, and dtype senses; NONE owns run-depth; 'accumulators_as_a|run of accumulators|run depth|run-depth' — ZERO owners; 'm16n8k' — the ldmatrix-granularity rule and the J=16-staging rule mention m16n8k32 shapes, never a depth pairing; 'as deep as the accumulator' — ZERO. NotBenchable here (the measured pair is an RTX PRO 6000 datum; no CUDA validator in this corpus; the depth pairing, the run admission, and the fold-order invariance are CPU-testable and ARE the fixtures). Source: tracel-ai/cubecl @ 4549408587cf464cc2ae7b3b7bc0bcf6005b5a92 (#1971) + 07cd07a2da9c4405044993c6ec2dc51b9d685508 (#1975), the #1966 series, MIT OR Apache-2.0 — the commit bodies + the accumulators_as_a diff (11 sites in #1971). Batch 310 / Plan 315. Every quote verbatim at the pin. RIDER (Batch 321 / Plan 321 — tracel-ai/cubecl @ f48e9f53 #2006, Apache-2.0 OR MIT — the A-OPERAND SIDE-INVARIANCE face, this family's third instance after B205's run-depth and this rule): the accumulator's ROWS lay along the free axes of whichever factor is handed over FIRST — the output's own placement where its innermost axes are cols's, else the two trailing axis groups exchanged at comptime — verbatim [in-diff] tile/mod.rs `rows_along` doc: 'the output's own order where its innermost axes are `cols`'s; else its two trailing groups exchanged, every axis above them where it was, so the first factor fills the instruction's `A` whichever of the product's operands it is.' The call site is the comptime placement, verbatim [in-diff]: `accumulator_in::<EA>(comptime!(rows_along(&self.place, &lhs.place.space, &rhs.place.space)), lhs, ...)` — the accumulator opens at the SIDE-CORRECT placement computed at comptime, never a runtime transpose; the commit-message states the payload, verbatim: 'a weight handed over first is decoded into A, its columns the fragment's sixteen rows, the activation loaded straight into B', and the drain side stays ordered: 'Such an accumulator drains into memory listed in the output's own order (Space::in_order_of), each cell reaching the same address by name.' The same commit adds the contract refusal for wrong-sided factors (Tile::mma refuses at the contraction, naming which factor is on the wrong side and how to hand them over). Batch 321 / Plan 321.".to_string(),
            ref_shape: r"// ref_shape (rule: mma-accumulator-run-as-one-operand-depth-matches-instruction):
// each m16n8 accumulator is consumed by its OWN depth-8 instruction --
// the instruction depth equals ONE accumulator, so the k8 rate (half
// the k16 rate, per the source's 1.97 ns a call per SM for either)
// applies to every step. The model counts instructions; the arithmetic
// is a stub whose fold order along k is the point.
#![allow(dead_code)]

const ACC_WIDTH: usize = 8; // one m16n8 accumulator = depth 8

struct Machine {
    calls: usize,
}

// one accumulator -> one instruction per step: depth is ACC_WIDTH
fn contract_one_acc_per_instruction(m: &mut Machine, scores: usize) -> u32 {
    let mut acc = 0.0f32;
    let mut s = 0;
    while s < scores {
        // depth-8 instruction: consumes exactly one accumulator
        for k in 0..ACC_WIDTH {
            acc += element(s + k); // the k-order fold
        }
        m.calls += 1;
        s += ACC_WIDTH;
    }
    acc.to_bits()
}

fn element(_k: usize) -> f32 {
    1.0
}

#[test]
fn every_step_pays_the_depth8_call_cost() {
    let mut m = Machine { calls: 0 };
    let scores = 32; // 4 accumulator widths
    let _ = contract_one_acc_per_instruction(&mut m, scores);
    // 32 scores / 8-wide accumulators = 4 instructions at depth 8 --
    // the half-rate instruction class on every step of the contraction.
    assert_eq!(m.calls, 4);
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: mma-accumulator-run-as-one-operand-depth-matches-instruction):
// a RUN of R consecutive accumulators is laid into the A operand (the
// manual-mma 16-bit layout already places them there), so ONE
// instruction contracts at depth R * ACC_WIDTH -- a run of two m16n8
// scores feeds k16 at the full rate. The k-order fold is UNCHANGED, so
// the result is bit-identical; the call count halves.
#![allow(dead_code)]

const ACC_WIDTH: usize = 8;

struct Machine {
    calls: usize,
}

fn element(_k: usize) -> f32 {
    1.0
}

// one accumulator -> one instruction per step (the ref-shape pairing,
// repeated so the A/B is self-contained)
fn contract_one_acc_per_instruction(m: &mut Machine, scores: usize) -> u32 {
    let mut acc = 0.0f32;
    let mut s = 0;
    while s < scores {
        for k in 0..ACC_WIDTH {
            acc += element(s + k);
        }
        m.calls += 1;
        s += ACC_WIDTH;
    }
    acc.to_bits()
}

// a run of R consecutive accumulators -> ONE instruction of depth
// R * ACC_WIDTH; a run of one degrades to the ref shape and stays legal
fn contract_run_as_one_operand(m: &mut Machine, scores: usize, run: usize) -> u32 {
    assert!(scores % (run * ACC_WIDTH) == 0);
    let mut acc = 0.0f32;
    let mut s = 0;
    while s < scores {
        // ONE instruction consumes the whole run: depth = run * ACC_WIDTH
        for k in 0..run * ACC_WIDTH {
            acc += element(s + k); // the same k-order fold, wider step
        }
        m.calls += 1;
        s += run * ACC_WIDTH;
    }
    acc.to_bits()
}

#[test]
fn the_run_halves_the_calls_and_keeps_the_bits() {
    let mut one = Machine { calls: 0 };
    let mut run = Machine { calls: 0 };
    let scores = 32;
    let a = contract_one_acc_per_instruction(&mut one, scores);
    let b = contract_run_as_one_operand(&mut run, scores, 2);
    // bit-identical: the k-order fold is the same sequential sum
    assert_eq!(a, b);
    // a run of two m16n8 accumulators feeds ONE k16 instruction per pair
    assert_eq!(one.calls, 4);
    assert_eq!(run.calls, 2);
}

#[test]
fn a_run_of_one_stays_legal_at_the_ref_depth() {
    let mut m = Machine { calls: 0 };
    // a held accumulator is a run of one: depth 8, the degraded-but-
    // correct shape the API keeps (per-run degradation, not per-kernel)
    let _ = contract_run_as_one_operand(&mut m, 16, 1);
    assert_eq!(m.calls, 2);
}
".to_string(),
        },
        // ── Rule 2 (category E — parallelism/barriers): cubecl #1971+
        // #1975, the Stages::free atom. SMEM declaration order is an
        // allocation-fragmentation control: persistent barriers first as
        // one run, transient slot data after, or the freed space is
        // pieces that fit nothing. ──
        KernelRule {
            rule_id: "barriers-declared-before-slots-one-free-run".to_string(),
            gpu_concept: "MECHANISM (SMEM declaration order is an allocation-fragmentation control; category E — parallelism/barriers): in a staged ring whose slots are FREED while their barriers stay LIVE, what the allocator can hand out is only a CONTIGUOUS RUN of freed memory — so declaration ORDER decides whether the freed space is one run the next stage fits into, or pieces cut apart by live barriers that fit nothing. THE DEFECT, verbatim (the #1966 series body): 'Each slot declared its data then its full/empty barriers, which stay in use, so a ring filled by the copy engine freed its slots as pieces cut apart by live barriers: an attention cube's 64 KiB partial fit none of them and was placed after the ring, 131 KiB in all, past an RTX PRO 6000's 99.' — per-slot [data, full, empty] interleaving isolates every freed data block between two live barrier pairs; the 64 KiB partial fits no piece, lands after the ring, and the kernel dies at placement over the platform ceiling. THE REMEDY, verbatim: 'Every slot's barriers are now declared first and the data after, one run.' — the long-lived barriers form ONE run at the ring's head; the freed data forms ONE run after them. The mechanism sentence, verbatim: 'a ring's barriers are declared before its slots' data — `Stages::free` hands a ring's shared memory to what the kernel declares after the walk, which the allocator gives only a run of freed memory.' DECISION CONTENT: (a) in any region where SOME declarations die and others persist, group by LIFETIME first — the persistent (barriers) before the transient (slot data) — so the freed set is contiguous; (b) the failure surfaces at PLACEMENT of a later allocation (the 64 KiB partial), never at the ring's own declaration — the defect is silent until the next stage asks for a run; (c) the ceiling is the platform's SMEM budget (99 on the RTX PRO 6000 class) — the remedy keeps the same total bytes but makes the FREED part one placable run, so the partial places inside the ring and the total never grows; (d) the ordering is per-RING — each ring's barriers before that ring's slots' data. REGIME: multi-stage smem rings (copy-engine filled, compute drained), any arena whose allocator hands out contiguous runs, barriers or semaphores that outlive the data they guard. ANTI-REGIME: everything lives or dies together (no fragmentation is possible — the lifetime-disjoint union rule owns the sharing axis); barriers freed with their slots (order is irrelevant); a single-slot stage (one run regardless of order). KINSHIP (do not double-count): lifecycle-disjoint-intermediates-union-smem-scratch (B234, gpu_94) owns lifetime-disjoint SHARING — two users time-sharing one slot; THIS owns the DECLARATION-ORDER axis inside one ring's free walk (what the free leaves behind); the B234 smem-interleave legality face owns free-LEGALITY (a barrier dominates its slot's first use) — THIS assumes that legality and decides the ORDER; mbarrier-staged-in-structural-padding (the category-A family) hides barriers in padding BYTES — a sizing trick orthogonal to the run-fragmentation axis; the smem-budget family owns the CEILING side (how big), never the fragmentation side (how it lays). GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'barrier' 83 files — the owners are reduction barriers, arrival universality, WAR trailing barriers, and prefetch/barrier pairs; NONE ties declaration order to free-run fragmentation; 'fragmentation|declared before|declaration order' — ZERO owners; 'Stages::free|freed its slots|one free run' — ZERO. NotBenchable here (SMEM placement policy — the allocator's contiguous-run behavior is the mechanism; the order-vs-fragmentation shape, the placement overflow, and the one-run remedy are CPU-testable and ARE the fixtures). Source: tracel-ai/cubecl @ 4549408587cf464cc2ae7b3b7bc0bcf6005b5a92 (#1971) + 07cd07a2da9c4405044993c6ec2dc51b9d685508 (#1975), the #1966 series, MIT OR Apache-2.0 — the commit body (the Stages::free atom). Batch 310 / Plan 315. Every quote verbatim at the pin.".to_string(),
            ref_shape: r"// ref_shape (rule: barriers-declared-before-slots-one-free-run):
// each slot declares [data, full, empty]; after the copy-engine walk
// the slot DATA dies but the barriers stay live -- the freed space is
// a piece per slot, cut apart by the live barriers. A partial wanting
// 64 fits no piece, so it places after the ring and the total passes
// the 99 ceiling (the source's own figures: 64 partial, 131 total, 99
// platform limit; this model uses small numbers of the same shape).
#![allow(dead_code)]

const SLOT_DATA: usize = 24;
const BARRIER: usize = 2;
const SLOTS: usize = 3;
const CEILING: usize = 99;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Data,
    Barrier,
}

#[derive(Clone, Copy)]
struct Decl {
    kind: Kind,
    size: usize,
}

fn ring_data_first() -> Vec<Decl> {
    let mut decls = Vec::new();
    for _ in 0..SLOTS {
        decls.push(Decl { kind: Kind::Data, size: SLOT_DATA });
        decls.push(Decl { kind: Kind::Barrier, size: BARRIER }); // full
        decls.push(Decl { kind: Kind::Barrier, size: BARRIER }); // empty
    }
    decls
}

// free runs = maximal consecutive dead stretches after the walk
fn free_runs(decls: &[Decl]) -> Vec<usize> {
    let mut runs = Vec::new();
    let mut run = 0usize;
    for d in decls {
        let dead = d.kind == Kind::Data; // the walk drained the data
        if dead {
            run += d.size;
        } else if run > 0 {
            runs.push(run);
            run = 0;
        }
    }
    if run > 0 {
        runs.push(run);
    }
    runs
}

fn fits(runs: &[usize], want: usize) -> bool {
    runs.iter().any(|&r| r >= want)
}

#[test]
fn the_partial_fits_no_piece_and_lands_past_the_ceiling() {
    let decls = ring_data_first();
    let runs = free_runs(&decls);
    // every freed data block is isolated between live barrier pairs:
    // pieces, never a run
    assert_eq!(runs, vec![SLOT_DATA, SLOT_DATA, SLOT_DATA]);
    let want = 64;
    assert!(!fits(&runs, want));
    // placed after the ring: the total passes the platform ceiling
    let ring = decls.iter().map(|d| d.size).sum::<usize>();
    assert!(ring + want > CEILING);
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: barriers-declared-before-slots-one-free-run):
// the ring declares every barrier FIRST (one live run) and the slot
// data after -- after the walk the data dies as ONE contiguous run and
// the partial places inside the ring; the total never grows.
#![allow(dead_code)]

const SLOT_DATA: usize = 24;
const BARRIER: usize = 2;
const SLOTS: usize = 3;
const CEILING: usize = 99;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Data,
    Barrier,
}

#[derive(Clone, Copy)]
struct Decl {
    kind: Kind,
    size: usize,
}

fn ring_barrier_first() -> Vec<Decl> {
    let mut decls = Vec::new();
    for _ in 0..SLOTS {
        // every slot's full/empty barriers join ONE leading live run
        decls.push(Decl { kind: Kind::Barrier, size: BARRIER }); // full
        decls.push(Decl { kind: Kind::Barrier, size: BARRIER }); // empty
    }
    for _ in 0..SLOTS {
        decls.push(Decl { kind: Kind::Data, size: SLOT_DATA });
    }
    decls
}

// free runs = maximal consecutive dead stretches after the walk
fn free_runs(decls: &[Decl]) -> Vec<usize> {
    let mut runs = Vec::new();
    let mut run = 0usize;
    for d in decls {
        let dead = d.kind == Kind::Data; // the walk drained the data
        if dead {
            run += d.size;
        } else if run > 0 {
            runs.push(run);
            run = 0;
        }
    }
    if run > 0 {
        runs.push(run);
    }
    runs
}

fn fits(runs: &[usize], want: usize) -> bool {
    runs.iter().any(|&r| r >= want)
}

#[test]
fn the_data_frees_as_one_run_and_the_partial_places_inside() {
    let decls = ring_barrier_first();
    let runs = free_runs(&decls);
    // every slot's data is consecutive after the barrier block: ONE run
    assert_eq!(runs, vec![SLOT_DATA * SLOTS]);
    let want = 64;
    assert!(fits(&runs, want));
    // the partial places inside the ring: the total stays the ring's own
    let total = decls.iter().map(|d| d.size).sum::<usize>();
    assert_eq!(total, SLOTS * 2 * BARRIER + SLOTS * SLOT_DATA);
    assert!(total <= CEILING);
}
".to_string(),
        },
        // ── Rule 3 (category D — dispatch/work-mapping): PrismML-Eng/
        // llama.cpp 9d9efeadbdd1 #415, the b10831 scored-tag tip. ONE
        // atom: blocks-per-work-unit < threads-per-CTA idles warps; the
        // warp-per-row re-map is admitted only while blocks_per_row <=
        // warp_size (one block per lane keeps the butterfly). ──
        KernelRule {
            rule_id: "warp-per-row-small-blocks-cta-under-load-law".to_string(),
            gpu_concept: "MECHANISM (warp-per-row at small K; the admission floor is blocks-per-row <= warp_size; category D — dispatch/work-mapping): a whole-CTA block-stride loop over a row's weight blocks gives one thread per BLOCK — when blocks-per-row < threads-per-CTA, the surplus warps load NOTHING, and at one destination column the CTA's capacity sits idle on every row. THE SOURCE LAW, verbatim (the commit body, PrismML-Eng/llama.cpp @ 9d9efeadbdd1 #415, the b10831 tip): 'The generic loop strides the 128-weight blocks over all threads of the CTA, so at K <= 4096 (<= 32 blocks per row) only the first warp loads. Give each warp its own row and let the lanes stride the blocks, with the same per-row accumulation order and epilogue, so outputs are unchanged.' — the remedy re-maps work from (thread, block) over ONE row to (warp, row) over MANY rows: warp w owns row row0+w, its lanes stride that row's blocks, one 128-weight block per lane. THE ADMISSION FLOOR, verbatim (in-diff): 'if constexpr (small_k && type == GGML_TYPE_PTQ1_0 && ncols_dst == 1 && !y_soa && nwarps > 1 && rows_per_cuda_block == nwarps)' gating 'if (ids != nullptr && blocks_per_row_x <= warp_size)' — the crossover is EXACT: at most one block per lane keeps the butterfly ('With at most 32 blocks per row every lane holds one block and the lane sums then go through the same warp_reduce_sum butterfly as the generic path, so the result is unchanged.'); above the floor the warp-per-row form degrades (lanes hold ragged block counts) and the generic loop stays. The in-diff comment, verbatim: 'PTQ1_0 MoE (ids), one column, K small enough for one block per lane: the generic loop below puts one 128-weight block on each thread of the whole CTA, so with K <= 4096 only warp 0 loads while warps 1..3 idle. Warp-per-row instead: warp w owns row row0+w and its lanes read that row's blocks (planar PT layout).' BIT-IDENTITY is a construction property, not a claim to check: with blocks_per_row <= warp_size the generic path's per-lane accumulation is ALREADY exactly one block per lane (lane l takes blocks l, l+THREADS, ... — just block l), so both paths compute the same butterfly over the same lane-to-block map; the epilogue and reduction are copied verbatim. The shape note: this is the MUL_MAT_ID/ids path (expert-row indirection, one destination column) on the planar packed layout. DECISION CONTENT: (a) the crossover predicate compares BLOCKS-PER-WORK-UNIT against THREADS-PER-CTA — blocks < threads is the idle precondition; the REMEDY's floor is tighter: blocks_per_row <= warp_size (one block per lane, the butterfly kept); (b) the re-map needs rows to GIVE: rows_per_cuda_block == nwarps (each warp takes exactly one row) and ncols_dst == 1 (the generic loop's row-parallelism is otherwise load-bearing) and the ids indirection present; (c) keep the generic path as the fall-through — the admission is constexpr per instantiation, the runtime ids/width check picks per call; (d) the epilogue and reduction are COPIED VERBATIM from the generic path — a changed fold order voids the bit-identity. REGIME: quantized mat-vec over expert-indexed rows (MUL_MAT_ID), small-K decode shapes (K <= 4096 for 128-weight blocks), one destination column, planar packed layouts. ANTI-REGIME: blocks-per-row above the warp floor (lanes would hold ragged counts — the generic loop stays); multi-column destinations (the row mapping collides with the column mapping — the split-axis family owns that corner); blocks-per-row >= threads-per-CTA (no idle warps — nothing to reclaim). KINSHIP (do not double-count): the gpu_49 rule owns the PLAIN-matvec crossover ('when a matvec's K blocks per ROW are FEW relative to the thread count' it flattens (row group, K block) pairs into one index) — the output-starved corner on the DIRECT path; THIS owns the IDS-path warp-per-ROW remedy with its one-block-per-lane admission floor (same idle-precondition genus, different path, different remedy — the two compose); the planar-transposed family (B180) owns the PACKED LAYOUT the block-dot call consumes — THIS is the work-mapping over it; the warp-partition reduction family (gpu_09) owns the butterfly the epilogue reuses; one-warp-ordered-reduce-row-batched-no-smem-barrier (B261) is the OPPOSITE resource corner (ONE warp takes whole rows across row-batches; THIS is many warps taking one row each). GREP EVIDENCE (over the kernel_opt corpus, 3+ vocabularies): 'blocks_per_row' — the gpu_49 crossover + a sizing table's smem asserts; NONE owns a warp-per-row admission floor; 'warp-per-row|warp_per_row' — ZERO owners; 'rows_per_cuda_block|ids != nullptr' — ZERO; 'idle' — the B261 resource corner + occupancy families, never this crossover. NotBenchable here (fork CUDA on the opponent's newest scored tag, b10831 — no in-commit numbers, sm_89 impact untested upstream; the crossover arithmetic, the one-block-per-lane floor, and the fold-order invariance are CPU-testable and ARE the fixtures; the league disposition rides the riir-ai watch, never this corpus). Source: PrismML-Eng/llama.cpp @ 9d9efeadbdd1d77458d98717a96fbfa3a016e159 (#415, 'cuda: warp-per-row PTQ1_0 mat-vec for MUL_MAT_ID when K <= 4096', the llama.cpp MIT family) — ggml/src/ggml-cuda/mmvq.cu (+70). Batch 310 / Plan 315. Every quote verbatim at the pin. RIDER (Batch 323 / Plan 323 — the ARCH-CONDITIONAL-ROWS + FUSION-CONTEXT-GEOMETRY faces, PrismML-Eng/llama.cpp #417 @ e20db821 + #419 @ 02bd091e, MIT): (a) rows-per-CTA is ARCH-CONDITIONAL — PQ2_0 one-column mat-vec takes two rows per warp ONLY at `cc == GGML_CUDA_CC_HOPPER && nrows_x <= 17408`, verbatim [in-tree]: 'smaller Hopper projections need more CTAs; keep four-row reuse for the vocabulary head' (the default PQ2_0_MC_ROWS stays 4; the supported() predicate also tightened `blk <= ncols` -> `ncols % blk == 0`); (b) the FUSION-CONTEXT face — a geometry table row is only valid under the fusion context it was measured under: the extended tall mat-vec types DROP to the default geometry when a gate matrix is fused (`const bool ext_ok = ext && mmvq_tall_ext_type(type) && fusion.gate == nullptr;`), verbatim [in-tree]: 'The extended tall mat-vec types are left on the default geometry when a gate matrix is fused (gate+up pair): there 8 rows per CTA ran slower' — PQ2_0 unaffected. Count-neutral faces, Batch 323 / Plan 323.".to_string(),
            ref_shape: r"// ref_shape (rule: warp-per-row-small-blocks-cta-under-load-law):
// the generic loop strides a row's weight blocks over ALL threads of
// the CTA -- one thread per block. At blocks_per_row < threads_per_cta
// the surplus threads load nothing: with 4 warps x 8 lanes and 8
// blocks per row (K = 1024 in the 128-weight block model), warps 1..3
// idle entirely -- 8 of 32 threads carry the row.
#![allow(dead_code)]

const WARPS: usize = 4;
const LANES: usize = 8;
const THREADS: usize = WARPS * LANES;

struct Cta {
    loading_threads: usize,
}

// the generic mapping: tid takes blocks tid, tid+THREADS, ... -- with
// blocks_per_row < THREADS only the first tid < blocks_per_row threads
// load at all
fn generic_row_pass(c: &mut Cta, blocks_per_row: usize) {
    for tid in 0..THREADS {
        if tid < blocks_per_row {
            c.loading_threads += 1; // this thread loads one block
        }
    }
}

#[test]
fn the_surplus_warps_load_nothing() {
    let mut c = Cta { loading_threads: 0 };
    let blocks_per_row = 8;
    generic_row_pass(&mut c, blocks_per_row);
    assert_eq!(c.loading_threads, 8);
    // warps 1..3 (24 threads) idled: three quarters of the CTA's
    // load capacity stood down on this row
    assert_eq!(THREADS - c.loading_threads, 24);
}
".to_string(),
            opt_pattern: r"// opt_pattern (rule: warp-per-row-small-blocks-cta-under-load-law):
// warp w owns row row0+w; its LANES stride that row's blocks. The
// admission floor: blocks_per_row <= warp_size so every lane holds
// exactly ONE block and the lane sums go through the SAME butterfly
// the generic path builds (with one block per lane the generic path's
// per-lane accumulation IS one block) -- bit-identical by construction;
// every warp now carries a row.
#![allow(dead_code)]

const WARPS: usize = 4;
const LANES: usize = 8; // the warp size in the model
const ROWS: usize = WARPS; // rows_per_cuda_block == nwarps
const THREADS: usize = WARPS * LANES;

// the admission: ids present, one destination column, more than one
// warp, and the one-block-per-lane floor
fn warp_per_row_admits(blocks_per_row: usize, ids_present: bool, ncols_dst: usize) -> bool {
    ids_present && ncols_dst == 1 && WARPS > 1 && blocks_per_row <= LANES
}

// the generic path's per-row result: lane l takes blocks l, l+THREADS,
// ... (exactly one when blocks_per_row <= LANES), then the butterfly
fn generic_row_sum(row: &[f32], blocks_per_row: usize) -> f32 {
    let mut lane_sums = [0.0f32; LANES];
    for l in 0..LANES {
        let mut acc = 0.0f32;
        let mut b = l;
        while b < blocks_per_row {
            acc += row[b];
            b += THREADS;
        }
        lane_sums[l] = acc;
    }
    butterfly(lane_sums)
}

// the SAME butterfly both paths build (warp_reduce_sum's xor tree)
fn butterfly(mut lane_sums: [f32; LANES]) -> f32 {
    let mut step = 1;
    while step < LANES {
        for l in 0..LANES {
            if l % (2 * step) == 0 {
                lane_sums[l] += lane_sums[l + step];
            }
        }
        step *= 2;
    }
    lane_sums[0]
}

// warp w owns row w; lane l holds exactly its one block (l < bpr),
// then the same butterfly
fn warp_per_row_sum(rows: &[&[f32]; ROWS], blocks_per_row: usize) -> [f32; ROWS] {
    let mut out = [0.0f32; ROWS];
    for w in 0..ROWS {
        let mut lane_sums = [0.0f32; LANES];
        for l in 0..LANES {
            if l < blocks_per_row {
                lane_sums[l] = rows[w][l]; // one block per lane
            }
        }
        out[w] = butterfly(lane_sums);
    }
    out
}

#[test]
fn the_warp_per_row_sum_is_bit_identical_to_the_generic_path() {
    let rows: Vec<Vec<f32>> = (0..ROWS)
        .map(|r| (0..8).map(|b| (r * 8 + b) as f32).collect())
        .collect();
    let refs: [&[f32]; ROWS] = [&rows[0], &rows[1], &rows[2], &rows[3]];
    let got = warp_per_row_sum(&refs, 8);
    for w in 0..ROWS {
        // same lane-to-block map, same butterfly: unchanged bits
        assert_eq!(got[w].to_bits(), generic_row_sum(&rows[w], 8).to_bits());
    }
}

#[test]
fn every_warp_carries_a_row_now() {
    // the re-map's utilization: 4 rows x 8 blocks = every lane of every
    // warp holds exactly one block (the ref shape idled 24 of 32 threads)
    assert_eq!(WARPS * LANES, ROWS * 8);
}

#[test]
fn the_floor_refuses_ragged_lane_counts_and_wrong_shapes() {
    // 9 blocks per row: lanes would hold ragged counts -> the generic
    // loop stays (the admission floor is exact)
    assert!(!warp_per_row_admits(9, true, 1));
    assert!(warp_per_row_admits(8, true, 1));
    assert!(!warp_per_row_admits(8, false, 1)); // no ids -> not this path
    assert!(!warp_per_row_admits(8, true, 4)); // multi-column -> generic
}
".to_string(),
        },
    ]
}

pub fn entries() -> Vec<KernelRule> {
    slice_b310_staged_leads()
}
