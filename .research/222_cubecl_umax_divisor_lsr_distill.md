# Research 222 — the cubecl #1731 umax-divisor LSR distill (Batch 190)

**Status:** RECORD — Batch 190 landed 2026-09-30 (Plan 187, corpus 684 → 685, category O 4 → 5; this note filed in the same unit).

## Source

- Repo: `tracel-ai/cubecl` @ `30a7ef00cd56e72716130b7111f2cfaa8664664d` (PR #1731)
- Commit: "fix(llvm): clamp unsigned divisors so LSR can reduce addresses behind them (#1731)" — `SamuelBelanger`, Tue Sep 29 12:52:15 2026 -0400
- License: MIT OR Apache-2.0 (dual)
- Clone: `.raw/cubecl` ephemeral — deleted at batch close (the ephemeral-clone law)
- Grading lineage: the 09-30 01:5x item-11 pass graded the lead **B+** from the commit message and designated it the next kernel slot's adjudication ("diff re-verification owed at the slot"); this slot's diff read **CONFIRMED** the mechanism and located the fix in the compiler LOWERING (stronger than the message alone suggested — every cubecl-llvm unsigned div/rem gets the clamp, not one kernel).

## The commit message (verbatim — the load-bearing quotes)

> An unsigned division or remainder by a runtime value becomes a udiv in
> scalar evolution, and LSR only expands a udiv whose divisor it can prove
> non-zero. An address whose loop-invariant start holds one, such as a cube
> position split by a count known only at launch, was left rebuilt from the
> loop counter on every iteration. In a matmul reading its weight along K,
> ptxas then issued 3 of the loop's 9 loads before the first FFMA instead of
> all 9, and the loop waited on memory several times per iteration.
>
> Dividing by umax(rhs, 1) proves the divisor non-zero. Division by zero was
> undefined in LLVM, and now gives what WGSL defines: x / 0 = x, x % 0 = 0.
> A constant divisor folds the clamp away.

## The diff (three hunks, quotes re-verified at the pin)

1. **`crates/cubecl-llvm/src/shared/to_llvm/math.rs`** — the fix: `insert_umax_one(ctx, rewriter, value)` builds `llvm.umax.<ty>(value, splat(1))` (vector-aware via `insert_splat`); the `lower_int_bin_arith!` macro gains an optional rhs-transform arm and applies it to the UNSIGNED pair only:
   - `lower_int_bin_arith!(UDivOp => llvm::UDivOp, insert_umax_one);`
   - `lower_int_bin_arith!(URemOp => llvm::URemOp, insert_umax_one);`
   - `lower_int_bin_arith!(SDivOp => llvm::SDivOp);` / `(SRemOp => llvm::SRemOp);` — deliberately UNTOUCHED (the unsigned-only contract's source evidence).
   - The doc comment on `insert_umax_one`: "A divisor SCEV cannot prove non-zero keeps LSR from reducing any address derived from it."
2. **`crates/cubecl-llvm/src/shared/offline_kernels.rs`** — `strided_walk(input, output, bands, pitch, stride, steps)`: `let start = (CUBE_POS_Y % bands) * pitch;` then `sum += input[(row * stride + start) as usize]` over `row = step * 32 + UNIT_POS_X` — the doc: "Walks a column in steps of 32 rows, one row per unit: how a matmul reads its weight along the reduced axis. A cube starts at its position split by a runtime count, so the start of every address holds a division by a value only known at launch."
3. **`crates/cubecl-llvm/src/nvptx/offline_tests.rs`** — `a_strided_walk_advances_its_address`: compiles the kernel to PTX offline, extracts the loop body (first label to the back-branch), asserts `!body.contains("mad.lo") && !body.contains("mul.lo")` — "the address is rebuilt from the counter". Doc: "The address of a strided walk advances by an add each iteration. Rebuilding it from the loop counter instead costs a multiply per load, and `ptxas` then interleaves the loads with the arithmetic that waits on them."

## Verdict (the distill-law adjudication at the slot)

- **Corpus:** YES (single-lead solo pack, the Batch-176 shape). Category **O: IR strength reduction** — the family's first RUNTIME-divisor member (the existing four are the constant-pow2 shift arms; a constant divisor folds the clamp into exactly those, so the family generalizes coherently).
- **Decision content:** bounded fix-space (one source-level op: divide by `umax(rhs, 1)` / `max(rhs, 1)`), mechanism-level magnitude story (SCEV/LSR non-zero provability → add-recurrence vs counter-rebuild; the 3-of-9-loads scheduling observable), two-regime (runtime divisor vs constant — the constant arm is the existing O rules), semantic-alignment bonus (WGSL div-by-zero), and the UNSIGNED-ONLY boundary as an anti-regime with source evidence.
- **GOAT:** NotBenchable in this corpus (criterion 1 fails for DOMAIN reasons — no validator hosts a ptxas/SASS scheduling metric; the Batch-136 NotBenchable-decision-content class). Determinism: integer math, bit-exact for rhs ≥ 1; rhs = 0 changes LLVM-UB to WGSL-defined (a fix direction on GPU targets).
- **Dedup (both vocabularies, zero rule collisions):** greps `divisor|udiv|umax|strength.?reduc|scalar evolution|LSR|address reduction|magic number|libdivide` over `src/` — no kernel_opt rule owns the runtime-divisor-LSR mechanism. Eight adjacencies READ and distinct-from'd in-rule (see the entry's KINSHIP block): `strength-reduce-div-pow2`/`-mod-pow2` (constant arm), `shader-fastdiv-mulhi` (hand-written magic — compiler-runs-LSR is the routing axis between them), `division-by-invariant-multiplication` (rust_perf CPU hand form), `hoist-bit-address-out-of-packed-row-sweep` (LICM: dividend invariant vs advancing — the cleanest both-directions split), `pack-scalar-params-into-combined-buffer` (host float reciprocal — exact integer indexing cannot take it), `predicate-gated-skip-empty-workset-step` (count≥1 at host vs at div site), `base-3-trit-packing` (constant divisors in decode), `telescoped-digit-coefficients` (float-domain reduction, distinct-from'd via its own strength-reduce-div-pow2 row).
- **Lane intel (detection-only, the TensorFold-220 precedent):** our-lane reach = any of our CubeCL/MSL/CUDA kernels dividing by a launch-time count in ADDRESS math (tile-index/position splits — the `CUBE_POS_Y % bands`-class shape). The riir-gpu/riir-infer kernels consume vendored cubecl 0.11.0-pre.2 — this fix lands upstream in a cubecl-llvm lane we do not yet vendor (cubecl-llvm is the NEW pliron-based compiler backend, separate from the pre.2 runtime we patch), so NO immediate behavior change for us; the rule's value is (a) the source discipline for our own kernels' address math and (b) a post-bump intake note: when the cubecl bump eventually brings the LLVM backend, this clamp ships for free (one more row for the standing post-bump-shift ledger: #1685, drop_queue, #1702, #1723, #1728, **#1731**).

## Drops

- The other two commits in the +3 watch window stand as graded at the 01:5x pass: #1728 (capture-record projection — correctness fix, post-bump-ledger row only), #1727 (tuner-elected — watch-only). No re-read owed.
- The `loop_body` PTX-extraction helper itself is ambient test plumbing (a few lines) — folded into the rule's VERIFICATION DISCIPLINE sentence, not separate content.

## Batch record

- `entries_gpu_58.rs`: 1 rule, `clamp-runtime-divisor-for-lsr-address-reduction` (O).
- Pins walked 684→685: `corpus/tests.rs` ×4 sites (entry count, category table O 4→5 + sum, round-trip, the rolling batch116 counter) + the category-table comment + `router.rs` expert-coverage pin (auto-wired through `KERNEL_CATEGORIES`; only the pin text moved). PLUS the C-category comment repaired 49→50 (stale since B186's `consumer-offset-absorption` C+1 — the honest-label pattern; measured by the slice-count script, the comment-sum said 683 against the pinned 684).
- Gates: `batch190_pseudocode_parses` + `batch190_unsigned_only_contract_rides_the_rule` (2 new tests); kernel_opt lib **2768/0**; default lib **2385/0**; clippy `-D warnings` clean at default / no-default / all-features (one doc_markdown backtick fix in the new file's header — `SamuelBelanger`, caught by the all-features posture); retrieval floors **21/21 HELD** (201.9s, no decay); score-bench + freeze regen NOT owed (zero clippy-corpus delta, P30).
- Global binary rebuilt at the release set post-landing (kernel_opt 685).

## Cadence disclosure

B189 landed ~03:3x +07 this same day; B190 lands ~1.5h later. The same-day second batch follows the B184→B185 (~2.5h) and B186→B187 (same-day staged-zero-remining) precedents: the lead was pre-designated by the B189 queue row as the NEXT kernel slot's adjudication, the box is cargo-quiet (load ~3.8, zero sibling cargo), and the adjudication itself consumed the slot's interval. Disclosed here rather than in the commit message alone.
