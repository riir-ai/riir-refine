# Bench 117 — the Batch-242 GOAT fixture: portable CPU-exact prescale (bench_008)

**Status:** RECORD (the G1 exactness GOAT PASSes; the G2 wall-clock record is a
per-face split — two faces win, one measured anti-case, root-caused at the asm level)

- **What:** `benches/bench_008_prescale_masked_field_goat.rs` — the full G1/G2/G4
  battery for the Batch-242 GOAT rule `masked-in-place-field-prescale-other-operand`
  (entries_gpu_98, Plan 244 / Research 274; plan 246). The rule's own benchability
  clause ("a fixture comparing shift-down decode vs masked-in-place + prescale over
  random packed words pins both exactness and op-count", the B231/116 precedent's
  portable sibling) — discharged by this bench. Portable: pure integer masks + fp32
  bit patterns, no ISA gate.
- **Host/provenance (citable run, this box):** `PROVENANCE: power=AC scheme=high
  load=5.52 swap=161MB gpu=99 %, 11714 MiB canary=skipped` — the GPU figure is the
  LOWO sibling lane (this bench is CPU-only by design; no GPU exclusivity claim).
  Portable posture = default target features (SSE2-class lowering, no
  target-feature gates — the point of the law).
- **G1 (exactness — the novel-numerics GOAT): PASS, 988 cases bit-identical.** All
  four arms: 4-bit (off=4p), 8-bit (off=8p), the 2-bit ALU twin (the qmv_nax2 scale
  ladder), and the 6-bit shift-down exception (two-word straddles included). Random +
  edge words (0x00000000/0xFFFFFFFF/0xAAAAAAAA/0x55555555/0x01234567/0xFEDCBA98) +
  the regime boundary x = ±2^-98 (x·2^-28 = 2^-126, the smallest normal — the LAST
  exact prescale). The exponent-cancellation proof executed: each factor exact
  (x·2^-off exact in-regime; field·2^off exact, ≤8-bit significand), the product
  rounds the same true value the shift-down form rounds. **REGIME: |x·2^-off| ≥
  2^-126; below it the prescale can round (subnormal) and bit-identity is NOT
  claimed** — the disclosed anti-regime.
- **G4: PASS** — zero allocations, both arms of all three swept faces, staging
  included (pre-allocated scratch; the counting allocator).
- **G2 (GEMV sweep, 25 shapes × 3 faces, interleaved best-of-5, correctness-coupled
  sinks): the per-face split:**
  - **2-bit ALU twin: 1.28–1.34× — WON all 25 shapes.** The law's plain-ALU face and
    the strongest: the staged ladder removes the per-element shift AND the per-element
    `(x·sv)` multiply (both fold into staging) — anchor rows=512 k=4096: opt 0.587 ms
    vs naive 0.754 ms.
  - **8-bit: 1.07–1.10× — WON all 25 shapes.** Anchor: 0.520 vs 0.561 ms.
  - **4-bit: 0.87–0.94× — LOST all 25 shapes (the measured CPU anti-case).** Anchor:
    0.460 vs 0.406 ms.
  - **Op-count ledger (the rule's own second axis, structural arithmetic): naive pays
    rows×K = 2,097,152 variable shifts in the hot loop at the anchor; opt pays K =
    4,096 staged prescale multiplies and 0 shifts — a 512:1 amortization.**
- **The 4-bit anti-case, root-caused at the asm level** (standalone `-O --emit asm`
  probes of both kernels): the NAIVE 4-bit lowering is tight — `psrld $imm` ×4 +
  `punpckldq` interleave + `pand` + `cvtdq2ps` (the shift-down fields are small, so
  the unsigned convert is plain); the MASKED 4-bit lowering must convert values whose
  bits land in the exponent region (up to 0xF000_0000 ≥ 2^31), and lowers through the
  broadcast + mask-vector + **float-bit-pattern exponent-trick chain** (`por
  0x53000000 / subps 0x53000080 / addps 0x4B000000`) — more vector ops than the
  immediate-shift form on this host/toolchain. At 8-bit the same trick chain STILL
  wins because the naive's shift-down extraction needs scalar-gather loads + FOUR
  lane-repacking shuffles (`punpcklqdq/shufps`) while the byte-aligned masked form is
  a single shuffle-free `pand` — the lane-alignment axis, not the shift count, is what
  the CPU cares about.
- **Lane intel (the delicious part):** LLVM's own lowering of the masked convert IS
  the rule's technique — it builds the float from the exponent field via
  `por`/`subps` bit patterns rather than converting. The compiler independently
  discovered the rule's exponent-field mechanism; the rule's teaching and the
  backend's lowering converged.
- **Verdict shape:** the bench exits 1 (`PARETO-FAIL` under the strict all-faces
  wall-clock bar — the bench_007 verdict discipline, kept unsoftened). The honest
  record: the rule's claimed axes (exactness + op-count) PASS; the wall-clock win
  transfers at byte-aligned widths (the 2-bit twin 1.29×, 8-bit 1.08×) and is a
  measured anti-case at half-lane widths (4-bit 0.88×) on scalar x86 — the law's
  native regime is the GPU/Metal source (simd_qmm) where per-lane shifts are the
  expensive op. The GPU-regime claim is untouched by this CPU record; re-armed on a
  future host/toolchain move (an AVX2/vpsrlvd posture may move the 4-bit face).
- **Corpus tie-in (count-neutral):** the GOAT rule's in-rule benchability clause now
  resolves to this landed fixture with the measured split (entries_gpu_98,
  append-only). AGENTS kernel_opt bullet + `.docs/11_domains/kernel_opt.md` Batch-242
  row carry the same record. Retrieval floors re-measured (kernel-corpus text delta —
  held, no re-pin). Freeze/score-bench NOT owed (zero clippy-corpus delta, P30).
