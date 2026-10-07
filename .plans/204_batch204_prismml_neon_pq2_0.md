# Plan 204 — Batch 204: the PrismML NEON PQ2_0 vec_dot pool

**Status:** COMPLETE — Batch 204 landed 2026-10-07 (see the snapshot row +
Research 239 §CONVERTED); allocated against `.plans/.highwater` 203→204,
dual_allocation_gate clean at allocation.

Source: `PrismML-Eng/llama.cpp` @ `6bfcd79a2d426abcd2b50e3c2d09ae2225e70a17`
(MIT; org-name trap `PrismML` 404s), commit "arm: NEON vec_dot for PQ2_0 (#265)".
Graded record: `.research/239_prismml_neon_pq2_0_vec_dot_distill.md` (YES B−).
Host box: shikuwa/4090 (Windows x86_64) — NEON compiles to NOTHING here, so rule 1
is NotBenchable on this box (the B182/B186 corpus-only precedent); the M3-Max GOAT
(rust_perf Pareto validator, integer-exact G1) stays an M3-session task.

## Tasks

- [x] T0 freshness + clone: re-clone the pin into `.raw/prismml-b204` (blob-filtered),
      confirm `6bfcd79a2` is still an ancestor of origin/master (or record drift),
      verify every Research-239 quote VERBATIM at the pin via `git show` before
      authoring; remove the clone at close (ephemeral-clone law). — DONE: the pin IS
      origin/prism's tip at clone time (zero drift); ALL quotes re-verified verbatim,
      0 corrections; clone removed at close.
- [x] T1 dedup re-grep: both vocabularies at the live corpus (vqtbl/replicat, sdot,
      swar/widened, dispatcher/orphan, telescoped/trit; rust_perf neon/simd) — confirm
      Research 239's zero-collision evidence still holds at Batch-203 head. — DONE:
      all clean (B194 = table-width replication, this = source-byte replication).
- [x] T2 rule 1 (rust_perf entries_17.rs) — landed as specced (simd category;
      NotBenchable on the x86_64 host; regime/anti-regime + the B62 law + the staged
      M3 GOAT ride context_notes; all numbers provenance-labeled).
- [x] T3 rider A (B116-addendum, entries_gpu_27.rs) — the ISA boundary arm landed
      (in-PR drop note quoted verbatim from arch-fallback.h @ the pin; three-arm
      decision: dp4a-widen / NEON-scalar / i8mm-smmla).
- [x] T4 rider B (B127, entries_gpu_33.rs) — the CPU-NEON instance + the exact
      vec_dot test methodology + the vpadalq_s16 compile-gate lesson landed.
- [x] T5 rider C (B103-addendum, entries_gpu_16.rs) — the pairing-migration orphan
      + the `ggml_cpu_pq2_needs_q8_0` row-extent escape hatch landed (conversion
      added the TWO-SITE override detail verified at ggml-cpu.c:1186-1204).
- [x] T6 standing gates: `batch204_rules_present_and_parse` (rust_perf, 11 needles)
      + `batch204_riders_carry_their_verified_contract` (kernel_opt, 3 hosts ×
      needles + the landed-then-dropped clause) — both GREEN.
- [x] T7 count pins: rust_perf 180→181 (tests.rs len pin + the mod.rs doc line);
      annotated 126→127 ×3 pins (the :1886 membership pin + :1984 serde pin +
      :2025 drafter pin — the last two were missed by the first pass and caught by
      the slice run, the pins did their job); without_spec 80→81; kernel_opt 761
      UNCHANGED (riders add no rules); AGENTS.md ×2 + README (181 rules row + the
      stale 113-annotated count repaired to 127 with an honest label) +
      rust_perf.md simd 9→10.
- [x] T8 gates: batch204 ×2 GREEN · rust_perf slice 2471/0 · kernel_opt+rust_perf
      slice 2886/0 · lib default 2471/0 · clippy -D ×4 postures clean (default /
      kernel_opt / rust_perf / no-default; 9 doc_markdown mechanicals healed by
      the riir-refine healer, compile-gated) · retrieval floors 21/21 HELD (210.7s,
      no decay, no re-pin — the B202 precedent: span-less rust_perf additions do
      not move the blended metrics) · freeze/score-bench NOT owed (zero
      clippy-corpus delta, P30).
- [x] T9 records: Research 239 §CONVERTED addendum (0 quote corrections; the
      en-route README count repairs disclosed); `.docs/11_domains/rust_perf.md`
      Batch 204 paragraph + `.docs/11_domains/kernel_opt.md` riders paragraph;
      AGENTS.md + README counts; snapshot LANDED row in the SAME commit.
- [x] T10 commit `feat: batch 204: ...` + push.
- [-] T11 (defer) M3-Max GOAT for rule 1 — rust_perf Pareto validator + integer-exact
      G1 (power-of-two scales, naive-codec scalar reference per the B62 lesson,
      256..4096 K × 32..512 rows GEMV sweep); x86_64 host compiles NEON to nothing —
      the bench runs on the M3 idle loop's next rust_perf slot; recorded in the
      snapshot row. — DEFERRED to the M3 session (their box owns aarch64 execution).

## Notes

- Method: coordinator authors the rule + riders directly (single-source conversion,
  Research 239's quotes already verified at the pin by the grading session) +
  re-verification at T0 per the freshness lesson; the B62 lesson (scalar reference =
  naive codec form) rides the rule text + the staged GOAT.
- The PTQ1_0 NEON kernel exists only in the PR's intermediate commits — quotes for
  rider A's negative come from the drop note + arch-fallback comment AT THE PIN
  (final tree), never from the dropped intermediate blob.
