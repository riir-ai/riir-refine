# Plan 063 — Batch 97: DwarfStar (antirez/ds4) Metal distill

Status: PLANNED — verdict `.research/117_dwarfstar_ds4_distill_verdict.md` (YES B+,
route kernel_opt). Batch executes on the next idle cycle; plan-first per Protocol A.

Source pin at mine time: https://github.com/antirez/ds4 — record the exact commit SHA
in the research note addendum before quoting (beta-quality, fast-moving).

- [ ] T1 lane split: spawn 2 parallel subagents with DISJOINT scopes — Lane A =
  `metal/moe.metal` (verified territory: candidates 1/3/4/5/6 from Research 117) +
  `metal/dense.metal`; Lane B = the unverified KV/flash lane (`metal/dsv4_hc.metal`,
  `metal/dsv4_kv.metal`, `metal/flash_attn.metal`, `metal/dsv4_rope.metal`,
  `metal/set_rows.metal`) + the host dispatch file `ds4_metal.m` (host-orchestration
  candidates only, D-category). Each returns pattern | bounded fix-space | category |
  source quote; coordinator re-verifies EVERY quoted line against the pinned commit
  (B31 discipline) and re-runs the dedup greps for every NEW candidate.
- [ ] T2 dedup re-verification: candidates 1-6 vs the corpus under both vocabularies
  (evidence recorded in Research 117); new Lane-B candidates get fresh greps; candidate
  7 (row_blocks) resolved — expected drop vs B85/B44 family.
- [ ] T3 provenance check: confirm candidates are DS4-original (absent in
  ggml-org/llama.cpp ggml-metal at the league pin 9ca265a — mirror the B96 method:
  grep the upstream tree for the mechanism vocabulary, cite both the adaptation and
  the origin).
- [ ] T4 entry authoring: corpus entries in `src/domains/kernel_opt/corpus/`
  (new entries_gpu file per the generator discipline: ensure_ascii=False rustlit
  escaping, category-count comment pins, count-pin sites walked: tests.rs corpus-count
  + expected-array + sums + router.rs expert total + mod.rs header + category headers).
  Original minimal Rust/CubeCL/WGSL fixtures only.
- [ ] T5 standing gates: `batch97_pseudocode_parses` (cfg(not(kernel_verify)) per B70)
  + `batch97_rules_present` + `batch97_concepts_covered` (case-exact needles).
- [ ] T6 exhaustive equivalence gate for candidate 1 (MANDATORY, ported-numeric rule):
  256×16 E8M0×E2M1 pairs vs f32-multiply-then-narrow reference, both band boundaries +
  the 0xff NaN-canonicalization fallback row — bit-exact half payloads, in the shape of
  the source's own `kernel_test_mxfp4_down_half_lut` oracle (CPU Rust twin).
- [ ] T7 GOAT: G1 parse/fixture gates; G2 lib tests (kernel_heal + default); G3 clippy 0
  both states + retrieval floors 14/14 HELD; G-det alloc-stable; score-bench flat
  (never weaken floors silently). Full CI guard only if the affected states don't
  cover a change (the B95/B96 honest-skip shape, recorded either way).
- [ ] T8 research note addendum: Research 117 gains a RECORD addendum (verdict →
  landed) with the pin SHA, final rule table, falsification ledger; AGENTS.md batch
  entry + corpus counts bumped; `.docs/11_domains/` kernel_opt doc row if counts move.
- [ ] T9 snapshot update: `.distill/001_mining_queue_snapshot.md` — Batch 97 landed
  line replaces the queued line (same commit as T8).
- [ ] T10 commit `feat: batch 97: ...` + push on develop.

Defers: `ds4_kvstore.c` / `ds4_ssd.c` / `ds4_streaming_hotlist*.inc` — storage-lane
concepts, NOT corpus material (route note in Research 117); revisit only if a
rust_perf io-shaped rule emerges that is not riir-neuron-db thinking.
