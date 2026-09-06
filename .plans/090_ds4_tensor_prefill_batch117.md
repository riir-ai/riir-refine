# Plan 090 — Batch 117: ds4 Metal tensor-prefill delta distill

**Status:** COMPLETE (2026-09-06; all tasks T1–T11 done; record: Research 141 + the queue-snapshot landing row)

## Source

antirez/ds4 @ fresh clone, mining pin **`9ab7053`** (origin/main at clone time 2026-09-06;
Batch 97/98 pin was `110afdd` — this is a DELTA batch, every quote re-verified at the NEW pin).
License MIT. The `.raw/ds4` clone is gitignored and removed at landing.

Intake lead: `.distill/001_mining_queue_snapshot.md` §Post-mining intake, item
"2026-09-06 NEW — ds4 Metal tensor-prefill pair" (filed by the riir-ai 846 agent after the
watch-coverage gap closed, `9639e7c50`).

## Commits in window

| commit | date | subject | kernel surface |
|---|---|---|---|
| `2ea3839` | 09-04 | Reuse routed activations in Metal tensor prefill | `metal/moe.metal` +107, `ds4_metal.m` +84 |
| `5806d09` | 09-05 | Batch Metal TP DSpark experts and reuse MXFP4 prefill activations | `metal/moe.metal`, `ds4_metal.m`, `ds4.c` (TP seed schedule + verifier-row batching) |
| `b42682f` | 09-05 | Reuse aligned Q8 scratch during CUDA decoding | `cuda/mmq/ds4_mmq.cu` (measured 17.90→19.25 t/s @2K) |
| `0e9cc2d` | 09-04 | Fix GLM attention masking and reductions across GPU backends | quality class — likely no rule, check for a latent-correctness note |

## Tasks

- [x] T1 — read all four commits at the new pin; classify each mechanism (rule vs rider vs no-rule)
- [x] T2 — 2 parallel subagent read lanes (Metal TP kernels lane; CUDA scratch + ds4.c TP lane) with
      byte-exact quotes + line numbers
- [x] T3 — coordinator quote re-verification (every quoted line against the real file at `9ab7053`)
- [x] T4 — dedup grep vs the corpus (same-input family B58/B63/B77/B97; B56 live-fields-only;
      B97 half-domain band; B31/B39/B58 staging/reuse; B26/B56 pool classes)
- [x] T5 — author corpus entries (Python generator, ensure_ascii=False, needle pre-verification)
- [x] T6 — standing gates: `batch117_rules_present` + `batch117_concepts_covered` +
      `batch117_pseudocode_parses` (cfg(not(kernel_verify)))
- [x] T7 — GOAT: kernel_heal lib + default lib + combined floors HELD zero re-pins + clippy 0
      (all states incl. --all-targets) + score-bench flat
- [x] T8 — count pins walked (tests.rs corpus count + expected-array + sums + router.rs + mod.rs
      headers/member lists + AGENTS.md + queue snapshot)
- [x] T9 — Research note `.research/141_*.md` (next free number, re-scan at write)
- [x] T10 — AGENTS.md batch entry + queue snapshot landing row + intake checkbox `[x]` +
      `.docs/11_domains/` if a new category row is needed
- [x] T11 — commit `feat: batch 117 ...` + push; `rm -rf .raw/ds4`

## Non-goals

- No GPU measurement (the numbers cited are the source's own, recorded as source-reported —
  never cite as ours; the M3 851 sweep holds the box).
- No re-mining of Batch 97/98 surfaces (dense.metal/moe.metal already mined there; this batch is
  the DELTA only — new mechanisms in the 4 commits, not a re-read of the file base).
- ds4 `be76012` (seed schedule) + the QA/docs commits are context only.
