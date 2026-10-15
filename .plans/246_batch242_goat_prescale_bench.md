# Plan 246 — the Batch-242 GOAT bench fixture (portable CPU-exact prescale)

**Status:** COMPLETE (T0-T5) — the GOAT bench LANDED as bench_008 / Bench 117: G1 988 cases BIT-IDENTICAL (all four arms, incl. the ±2^-98 regime boundary — the exactness GOAT PASSES); G4 zero-alloc; G2 measured SPLIT at 25 shapes × 3 faces — the byte-aligned faces WIN (the 2-bit ALU twin 1.28-1.34×, 8-bit 1.07-1.10×) and the 4-bit half-lane face is the measured CPU anti-case (0.87-0.94×, asm-root-caused: the naive's psrld-imm+cvtdq2ps lowering beats the masked form's exponent-trick convert chain; re-armed on a host/toolchain move); op-count 512:1. The fixture exits 1 under the strict all-faces bar — the honest split record; the rule's claimed axes (exactness + op-count) PASS. Count-neutral corpus append (the rule's Benchable-via clause resolved); retrieval floors re-measured.

The Batch-242 GOAT rule `masked-in-place-field-prescale-other-operand` (entries_gpu_98,
kernel_opt N-category) carries its own benchability clause: "Benchable via a Rust-idiom
fixture (the x86-GOAT precedent B231/116 — portable CPU-exact exponent-field +
integer-mask logic; a fixture comparing shift-down decode vs masked-in-place + prescale
over random packed words pins both exactness and op-count)". The landing deferred the
battery to a fresh cycle ("deserves the full G1/G2/G4 battery; not rushed there") — this
is that cycle. Portable by design: no ISA gate (the B231/116 precedent's portable
sibling — pure integer masks + fp32 bit patterns, runs on both boxes).

## Tasks

- [x] **T0 — claim row + plan-first push** (commit `9cf33e3d6`, push accepted first try).
- [x] **T1 — author `benches/bench_008_prescale_masked_field_goat.rs` + the Cargo
  `[[bench]]` row** (harness = false, no required-features — std-only). Four arms landed as designed; the 2-bit ALU-twin arm was PROMOTED from G1-only to a full G2 sweep arm after the first measured run showed the 4-bit face losing (the twin + the 8-bit arm are the byte-aligned faces where the law's mechanism wins on CPU — the complete three-face characterization).
- [x] **T2 — run the battery**: provenance PASSED (`power=AC scheme=high load=5.52 gpu=99%` — the LOWO sibling; CPU-only bench); G1 988/988 bit-identical; G4 zero-alloc ×6 arms; G2 split (2-bit twin 1.28-1.34× WIN 25/25, 8-bit 1.07-1.10× WIN 25/25, 4-bit 0.87-0.94× LOSE 25/25 — the anti-case root-caused at the asm level via standalone `rustc -O --emit asm` probes: the naive4 lowering = psrld $imm + punpckldq + cvtdq2ps, the masked4 = broadcast + pand + the por 0x53000000/subps/addps exponent-trick chain; at 8-bit the masked form wins because the naive pays 4 lane-repacking shuffles + scalar-gather loads while the byte-aligned mask is shuffle-free). The op-count ledger line + anchor times printed. LANE INTEL: LLVM's own lowering of the masked convert IS the rule's exponent-field technique.
- [x] **T3 — record**: `.benchmarks/117_prescale_masked_field_portable_goat.md` (+highwater 117); the GOAT rule's count-neutral bench-verdict append (entries_gpu_98, the Benchable-via clause resolved — python unique-anchor splice, LF/CRLF preserved); AGENTS ×2 (Role bullet + Feature list); `.docs/11_domains/kernel_opt.md` Batch-242 row.
- [x] **T4 — gates**: batch242 needles 2/2 green at the appended corpus (re-verified after the loop heal); kernel_opt slice 454/0/1i (corpus parse + count pins 847 + needles); retrieval floors HELD 20/0/1i in 304.68s (no re-pin — the rider precedent); default lib 2672/0/0; clippy ×3 postures 0/0/0 — the kernel posture initially caught TWO `explicit_iter_loop` at batch242's own needle-test loops (the B242 landing's gate-ordering miss — its ×3 ran before the test file landed); healed to the `*needles` form (the binding is `&&[&str]`), disclosed in the landing row. En-route ops: the first detached full-suite run was tree-killed ~24 min in during the quiet retrieval section (the box's zombie-hunter heuristic vs a legitimate long gate) + the killed test exe held the output file (LNK1104) — resolved by slicing the gates (each under the timeout) + killing the orphan.
- [x] **T5 — snapshot landing row + commit + push**.

## Yield ledger

G1 pass = the exactness proof pinned by execution. G2 pass = the amortization law
measured (the shift leaves the hot FMA loop for K staged multiplies). G2 fail at any
shape = an honest recorded outcome (PARETO-FAIL, exit 1) — the fixture still pins
exactness + the op-count ledger; the rule text then carries the honest measured label.

## Anti-goals

- No corpus-count change (one count-neutral append only).
- No freeze/score-bench regen (zero clippy-corpus delta, P30).
- No ISA-gated code paths (the portable point of the law).
