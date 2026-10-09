# Research 250 — cubecl #1802 k=8 manual MMA staged lead (accumulator-as-A-fragment)

**Status:** DISTILLED — pending owner decision. C+ kernel_opt lead, staged at the rider tier BEHIND Lead 12 (PrismML #306 PQ2_0 multicol) and the conversion set. Diff read in full this session (2026-10-10 ~01:0x +07, ephemeral clone `/tmp/cubecl_1802`, rm'd at close; zero-cargo under Plan-337) — the "re-verify the full diff at conversion" precondition is SATISFIED; what remains at the batch unit is dedup re-grep + transcription + compile, folded into whichever batch lands next.

## Source

- tracel-ai/cubecl `73cecbfc241e48fa174754ba299b790fb84e71e3` — `feat(cuda): offer the k = 8 manual mma for f16 from Turing on, and for bf16 from Ampere on (#1802)`, 2026-10-09 13:00 -0400. Apache-2.0 (cubecl).
- Files: `crates/cubecl-cpp/src/cuda/mma/manual.rs` (+25/−5) + `crates/cubecl-core/src/runtime_tests/cmma.rs` (+2). Small.
- Pin movement context: `281e70aab..f70376aba` (+4, envelope row 00:5x); this commit is the range's one perf-content atom.

## The law (quote authority: the diff, read at the pin this session)

The half-depth MMA shape exists for a layout identity, not a tile-size knob — comment verbatim:

> "The `k = 8` shape lays its `A` fragment out cell for cell as an `m16n8` accumulator, so a product's output contracts again without leaving registers: it is kept beside the `k = 16` one wherever both exist."

Body verbatim: "An m16n8 accumulator is, cell for cell, the A fragment of an m16n8k8 instruction: a product contracting its own output, as attention's value contracts its probabilities, reads it without leaving registers."

Test verbatim: "The `k = 8` shape stays offered past Turing: its `A` fragment is an `m16n8` accumulator's layout, what a product contracting its own output reads."

## The mechanism (three atoms)

1. **The layout identity:** an `m16n8` fp32 accumulator's cell layout == the `A`-fragment layout of `m16n8k8`. A second MMA that CONTRACTS the first product's output (P·V attention is the canonical case — the value contracts the probabilities) can read the accumulator DIRECTLY as its A operand — zero smem round-trip, zero layout shuffle.
2. **The availability widening (the diff's actual change):** before, k=8 f16 was offered ONLY on Turing (`>= 75 && < 80`) with a warning that it "likely does not follow the same layout pattern as those after 80"; now it is offered Turing+ (`>= 75`, comment + test assert it survives past sm_80 through sm_120), AND a bf16 A/B variant is offered Ampere+ (`>= 80`). The layout identity is what licenses keeping it beside k=16 everywhere both exist.
3. **The arch-keyed admission predicate:** `arch.get_version() >= 75` (f16) / `>= 80` (bf16) — ptxas refuses `.m16n8k8` below sm_75 and sm_70 has `m8n8k4` alone. Admission is a compile-time arch key, never a runtime probe.

## Grade rationale (C+, not B−)

- The identity is real, crisply stated, and test-pinned upstream — but the RULE's fix space for OUR corpus is "keep/choose the half-depth shape when a chained contraction follows," which is a kernel-shape AVAILABILITY decision most engines make once, not a per-site transform. Rider-or-rule adjudication at conversion: it may rider the mma-fragment-layout family rather than stand alone.
- 4090-relevant: riir-gpu consumes cubecl directly (riir-gpu = CubeCL + cudarc twins); sm_89 (Ada) is inside the widened admission for both f16 and bf16.

## Dedup (prep-time)

- `mma-fragment-layout` + the entries_gpu_09 family (xor-swizzle / ldmatrix / cpasync-multistage riders) own fragment OWNERSHIP + loading paths — the accumulator-consumed-as-next-A-operand composition is the unowned atom.
- The attention P·V epilogue family (split-KV / online-softmax / flash tiling) owns WHERE P·V happens — none owns the instruction-shape choice that keeps the P·V chain in registers.
- Fresh greps at conversion per Protocol A.

## Draft gpu_concept skeleton (the batch expands)

> Keep the half-depth `m16n8k8` MMA in the offered set beside `m16n8k16` wherever both exist — its `A`-fragment layout IS the `m16n8` accumulator's layout cell-for-cell, so a product that CONTRACTS the previous MMA's output (attention's value contracting its probabilities is the canonical shape) reads its A operand straight from the accumulator registers: no smem round-trip, no fragment shuffle, the chain never leaves registers. Availability is arch-keyed at compile time (ptxas floor: sm_75 for f16 — Volta has `m8n8k4` alone; sm_80 for bf16). Choose k=8 for the CONTRACTING stage of a two-MMA chain; k=16 remains the throughput default where no chain follows. Source: tracel-ai/cubecl @ `73cecbfc` (#1802) — crates/cubecl-cpp/src/cuda/mma/manual.rs `supported_mma_combinations` + the layout-identity comment/test. Batch (at landing) / Plan (Protocol A step 0) / Research 250.

## Queue position

Rider tier, BEHIND Lead 12 (PrismML #306 PQ2_0 multicol) and the Leads 1+2+P45+Lead 3 conversion set; lands inside the next kernel_opt batch unit (with Lead 12 or the conversion set, whichever batch opens first — the batch unit dedups and cuts).
