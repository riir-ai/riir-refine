# Plan 108 — Batch 127: the PrismML llama.cpp CUDA ternary MMQ mma distill

**Status:** LANDED 2026-09-07 — Batch 127 complete: kernel_opt +6 (559 → 565), 5 riders, GOAT full-pass (gates 3/3, lib 1167/1486/1603, retrieval byte-identical, clippy 0 ×3, score-bench #76 FLAT). Research 149. Clone removed at landing.

## Source

**PrismML/llama.cpp @ `e311ed38f`** (MIT; the league opponent pin, tag `prism-b10660-e311ed3`). The CUDA ternary MMQ mma family — UNMINED (prior contact B96/B113/B116 was Metal/format lanes). Target surfaces per the lead:

- `ggml/src/ggml-cuda/mmq.cu` — Q1_0/Q2_0/PQ2_0 MMQ cases (ternary-class group-128 codecs)
- `ggml/src/ggml-cuda/mma.cuh` — the m16n8k16-class mma primitives (pin-verified byte-stable vs the vendor clone's HEAD)
- `ggml/src/ggml-cuda/mmq-config-{ampere,blackwell,cdna}.cuh` — per-arch tile tables (the lead's "config" class)
- `ggml/src/ggml-cuda/ggml-cuda.cu:~1867` — dispatch fast-path (Q1_0/PQ2_0 @ ne11 ≥ 128)
- `mmq-hopper-q1.cu` — the opt-in Hopper wgmma path (present in the vendor tree too)

Lead estimate: 3–6 kernel_opt rules; the per-arch-config / multi-stage cp.async / ldmatrix-swizzle class. Dual-use note: structure reference for riir-ai Issue 884's adopt port (T6 there) — corpus-side mining does NOT touch that lane's riir-gpu code.

## Dedup needles (grep BEFORE proposing)

`B96` (telescoped-digit / bit-plane Metal extraction), `B98` (dequant-in-tensor-op-GEMM / GEMV carve-outs), `B113` (fa-vec tuning tables — the per-arch-table CLASS neighbor), `B116` (PTQ1_0 format), `Research 107` (ktransformers W4A16 — the fp16-acc anchor class B877 corrects), plus vocabulary variants: `ldmatrix|swizzle|cp.async|cp_async|pipeline|m16n8k16|wgmma|mma|accum|quad|row-block`.

## Tasks

- [x] T0 — stage `.raw/prismml-cuda` fresh clone at `e311ed38f`; verify HEAD == pin + LICENSE MIT; do NOT touch `../vendor/prismml-llama.cpp` (sibling league-watch state, NOT at the pin — verified: pin not an ancestor of its HEAD, mmq.cu differs). **DONE: org is `PrismML-Eng` (the lead's `PrismML` shorthand 404s); clone staged, HEAD == pin, MIT**
- [x] T1 — dedup grep the corpus (needles above) + 2 parallel read lanes proposing rules (pattern / bounded fix-space / category / rationale / source quote); coordinator re-verifies every quoted line at the pin. **DONE: 12 proposals → 6 landed + 5 riders; all 9 coordinator quote checks verbatim at the pin; byte_perm decode demoted to B20 rider (the prmt dequant recipe family already covers it)**
- [x] T2 — corpus entries (entries_gpu_33.rs) + `batch127_pseudocode_parses` gate + count pins walked (totals ×3 sites, per-category, annotated, mod.rs lists, doc counts). **DONE + the parse gate caught 3 C-flavored snippets (the B122/B123 lesson re-proven); count pins ×4 totals sites (incl. the stale 559 hiding in batch116_rules_present) + category table + router + 3 stale headers healed**
- [x] T3 — Batch 127 GOAT: batch gates first-shot; lib default / kernel_opt / kernel_opt+latent floors held; retrieval re-run (kernel 100/100/100, blended ≥94.7/96.9/97.7); clippy 0 ×3 states; score-bench FLAT. **DONE: 1167/0, 1486/0/1i, 1603/0/2i, retrieval byte-identical, clippy 0 ×3, #76 FLAT 96.2/1.9**
- [x] T4 — landing: Research 149 (next per `.research/.highwater`), AGENTS.md corpus counts, `.docs/11_domains/`, snapshot row; commit `feat: batch 127: ...` + push. **DONE: Research 149, AGENTS 565, .docs count row, snapshot MINED annotation**
- [x] T5 — remove `.raw/prismml-cuda` at landing; final snapshot line (pin current: `e311ed38f`). **DONE: clone removed; pins recorded current**
