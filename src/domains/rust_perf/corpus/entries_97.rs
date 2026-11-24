// Batch 407 — the vLLM GDN decode-fusion family's `rust_perf` half (Plan 406
// / Research 441): vLLM three-commit window `92b52792d` (#67733) +
// `78f20a5a3` (#69065) + `34c653886` (#61428), Apache-2.0 — quotes
// re-verified at the pins at this batch's T0 (blobless clone, checkout at
// `78f20a5a3`, `git show` at `92b52792d`/`34c653886`).
//
// rust_perf 392 → 393: one transcription rule:
// - `output-buffer-prezero-when-kernel-writes-all-rows` (allocation,
//   suggestion-only, DEMOTED NotBenchable-by-platform AT LANDING) — an
//   output buffer pre-zeroed and then fully overwritten by its producer
//   wastes the whole memset pass IN THE TORCH ALLOCATION CLASS (torch.zeros
//   pays a real memset); the Rust face measured NO win (validator
//   ParetoDominated 0 faster / 3 slower / 0 tie; the isolated probe —
//   identical indexing loops, only the allocation differing — read
//   0.967/0.979 twice, inside the tie band): `vec![0.0; n]` rides
//   `alloc_zeroed`, whose zero-page path serves the zeros without a
//   separate memset pass on this platform posture, so the pass the rule
//   teaches deleting is already absent here. Per the pre-registered bar
//   ("a Tie/Slower demotes") the bench spec is gone; the LAW stays
//   decision content (dead-store mechanism + regime condition + canary
//   discipline + the uninit-read correctness half).
//
// The kernel_opt half of the batch is count-neutral (8 faces on their
// owners, same commit). Fixtures: fixtures/b407_fixtures.rs (standalone,
// debug AND -O, executed pre-entry). NOTHING INVENTED — every quote read
// at the clone this session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 407 — the vLLM GDN decode-fusion family's `rust_perf` rule.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b407_rules() -> Vec<PerfRule> {
    vec![PerfRule {
        rule_id: "output-buffer-prezero-when-kernel-writes-all-rows".to_string(),
        name: "An output buffer pre-zeroed and then FULLY OVERWRITTEN by its producer \
               wastes the producer's own initialization work — allocate uninitialized \
               (`torch.empty`, `Vec::with_capacity` + set-len-on-full-write) when every \
               actual row is written before any read, and prove the coverage claim with \
               the NaN/poison canary in the test; the regime is load-bearing: EVERY row \
               written by ONE producer before any read — a buffer read between partial \
               writes, or padding rows consumed by anything but row-wise ops, stays on \
               the initialized path"
            .to_string(),
        category: "allocation".to_string(),
        buggy_code: r#"// NAIVE: the producer overwrites every row, so the pre-zeroing is
// a dead store — the overwritten values are identical, only the
// pre-write contents differ and are never read. (In the torch
// allocation class `torch.zeros` pays a real memset pass, so dropping
// it wins measurably; the Rust spelling `vec![0.0; n]` rides
// `alloc_zeroed`, whose zero-page path can serve the zeros without a
// separate pass — the win is allocation-class-dependent, measured a
// TIE on this platform.) The regime condition still binds either way:
// every row below is written by ONE producer before any read.
fn decode_rows(num_tokens: usize, hv: usize, d: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; num_tokens * hv * d]; // the wasted pass
    for t in 0..num_tokens {
        for h in 0..hv {
            for i in 0..d {
                out[(t * hv + h) * d + i] = kernel_value(t, h, i); // full overwrite
            }
        }
    }
    out
}

#[inline(never)]
fn kernel_value(t: usize, h: usize, i: usize) -> f32 {
    ((t * 7 + h * 13 + i) % 17) as f32 * 0.5 - 4.0
}"#
            .to_string(),
        fixed_code: r#"// FIXED: uninitialized allocation; the coverage claim is the TEST's
// (the source's own canary device — `torch.full_like(output_gate,
// torch.nan)` fails loud if any actual row is missed). Set the length
// only after the full overwrite; every actual row is written exactly
// once, padding rows (if any) never escape the producer.
fn decode_rows(num_tokens: usize, hv: usize, d: usize) -> Vec<f32> {
    let n = num_tokens * hv * d;
    let mut out = Vec::with_capacity(n); // no memset pass
    for t in 0..num_tokens {
        for h in 0..hv {
            for i in 0..d {
                out.push(kernel_value(t, h, i));
            }
        }
    }
    debug_assert_eq!(out.len(), n);
    out
}

#[inline(never)]
fn kernel_value(t: usize, h: usize, i: usize) -> f32 {
    ((t * 7 + h * 13 + i) % 17) as f32 * 0.5 - 4.0
}"#
            .to_string(),
        context_notes: Some(r##"Grade B − decision-content, suggestion-only, NotBenchable-by-platform — DEMOTED AT LANDING per the pre-registered bar ('a Tie/Slower demotes'): the Rust face measured NO win. The Pareto validator priced the pair ParetoDominated (0 faster / 3 slower / 0 tie — the fixed push-loop arm lost at every size), and the ISOLATED probe — identical indexing loops on both arms, only the allocation differing — read the pure-allocation delta at ratios 0.967/0.979 TWICE, inside the tie band (0.95–1.05): the push-loop overhead accounts for the validator's gap, and the allocation itself is a wash. THE MECHANISM: in the Rust allocation class `vec![0.0f32; n]` compiles to `alloc_zeroed`, and for never-read-before-overwrite bytes the allocator/OS zero-page path serves the zeros without a separate memset pass — the pass the rule teaches deleting (torch.zeros pays a real memset, so dropping it wastes the entire memset pass) is ALREADY ABSENT on this platform posture. The demotion is by-PLATFORM, never by-domain: an allocator that memsets alloc_zeroed (reused heap chunks, non-zero-page postures, other allocation classes) re-arms the win, and the source's own measured context (torch.zeros → torch.empty) is exactly that class — the win is real where it was measured and absent where we can validate, which is what by-platform means. The LAW stays decision content: the dead-store mechanism, the regime condition, the canary discipline and the correctness half (uninit-read avoidance) all stand; what fell is only the Rust-side perf claim. The overwritten values are bit-identical, only the pre-write contents differ and are never read — exact by construction, deterministic. SOURCE (vLLM @ 92b52792d, #67733 'Perf][GDN] Drop the prefill copies of the fused spec-decode path', Apache-2.0; quotes re-verified at the pin this batch): the dead-pre-zero elimination — `core_attn_out = torch.zeros(...)` (+ the later `.zero_()` re-fill on the metadata-missing warmup arm) → `torch.empty(...)` with the in-source comment 'Uninitialized like the non-fused path below.', and the SECOND site's law verbatim (the commit body): 'No zero fill in either branch: the kernels write every actual row, and padding rows only feed row-wise ops (same as forward_cuda).' + the in-source comment at the generic alloc: 'Uninitialized like the other attention backends: _forward_core writes every row up to num_actual_tokens. Padding rows behind it are only read by row-wise ops and never reach sampled outputs.' THE CANARY IS THE COMMIT'S OWN DEVICE (tests/kernels/mamba/test_gdn_fused_mtp.py:401-402, verbatim): '# Poison: the packed entrypoint writes every actual row, so no NaN survives.' + `fused_out = torch.full_like(output_gate, torch.nan)` — the poison fills the buffer with NaN and the parity assert fails loud if any actual row is missed; the canary is the REGIME CONDITION's enforcement, not decoration. THE REGIME CONDITION IS THE LOAD-BEARING HALF: (a) EVERY actual row written by ONE producer before any read — a buffer read between partial writes is the uninit-read UB class (the reason `vec-macro-zero-fill` exists as its inverse: THERE the zeros are READ as zeros, so the calloc is the win; HERE the zeros are never read, so the calloc is the waste); (b) padding rows 'only feed row-wise ops' is the named boundary — a padding row consumed by a max/argmax/reduction or escaped to a sampled output is an uninit-value defect, never a perf note; (c) the warmup arm the source kept `.zero_()` on (metadata missing → return with the buffer re-zeroed) shows the boundary in-source: THAT path's buffer IS read-as-zeros downstream, so it keeps the fill — the rule scopes per read-set, not per buffer. KINSHIPS (adjudicated in-rule): `vec-macro-zero-fill` (entries_03) is the INVERSE polarity — need zeros read as zeros → `vec![0; n]` rides `alloc_zeroed` and the explicit memset dies; THIS: zeros never read → the initialization itself dies; the two rules compose on one question — ARE THE ZEROES EVER READ? — and disagree in answer, never overlap. `hot-slot-recycler-flat-tables-zero-alloc-claims` (entries_39) owns ghost-slot REUSE economics in flat tables — unrelated axis; `raw-bytes-with-hit-backfill-once` (entries_69) owns single-physical-read caching — unrelated; scoped-init-at-consume-site (kernel_opt gpu_03, B-slice) is the DEVICE-scratch cousin: only accumulation windows need pre-zeroed memory and producers that fully overwrite need none — same discipline, device tier, cite both directions (the accumulation-target ANTI-REGIME is shared: `matvec_tiled`'s 'y is pre-zeroed; each column tile's partial dot ACCUMULATES' (entries_05) is the correct pre-zeroing THIS rule must never touch — accumulation READS the zeros as zeros); reserve-then-commit-capacity-accounting (entries_58) owns COMMIT-side capacity accounting — the allocation-size axis, not the initialization axis, compose. REGIME: producer/consumer buffer pairs in hot decode/prefill loops (output rows written per token), scratch the producer fully rewrites per call, deserialization targets overwritten field-by-field in full. ANTI-REGIME: accumulation targets (the zeros are data — entries_05's matvec), partial writes before a read, buffers whose padding rows feed reductions or escape the producer, safety-cleared memory an attacker can observe across resets (uninit exposure is a soundness surface, not a knob). Suggestion-only fixable (the rewrite is local; the coverage proof is the test's job). Bench spec removed at the demotion (was sizes [1024, 16384, 262144]; the harness lives in git history under this rule's entry). Batch 407 / Plan 406 / Research 441 (vLLM 92b52792d + the 78f20a5a3/34c653886 window; the canary quote byte-verified at the pin). Fixture original (the source is Python/torch — the fixtures are our own Rust reproductions)."##.to_string()),
        // NotBenchable-by-platform at landing (the GOAT falsifier fired):
        // the validator priced the pair ParetoDominated (0 faster / 3
        // slower / 0 tie) and the isolated probe (identical indexing
        // loops, only the allocation differing) read 0.967/0.979 twice —
        // `vec![0.0; n]` rides `alloc_zeroed`, whose zero-page path makes
        // the pass the rule deletes already absent on this platform. The
        // pre-registered bar demotes; see the context_notes for the
        // mechanism + the re-arm condition (an allocator that memsets
        // alloc_zeroed re-arms the win).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }]
}
