# Plan 077 — Batch 107: uzu + lily defers batch (the B105 queue residue)

**Status:** COMPLETE (2026-09-05, 4090 box, CPU-only) — 8 rules landed, corpus 482 → 490, GOAT all green.

Consumes the B105 deferred queue (`.distill/001_mining_queue_snapshot.md` line ~541):
uzu @ `115609b6ef7ce5a94fc761bf395780f032b09f9b` (Release 0.5.23, **MIT**) + lily
(pplx-garden `lily/`) @ `1ed972ed3f0bd5616c997c9507c25616c63394fc` (**Apache-2.0**).
Verdict-time grades carried from Research 127/128; re-verification at the pin is
mandatory (quotes drift, grades flip at mine time — B105 falsified two verdict-time
claims). Expect ~5-8 rules after falsification; every candidate re-adjudicated.

## Candidates

Lane A (uzu, 7): `sparse-demand-paged-kv` (B+), `gumbel-argmax-unified-sampling-descent`
(B+), `multi-field-argmax-tuple-reduction` (B−), `flat-topology-fast-path` (weak),
`integer-mma-epilogue-dequant-scale-residency` (B−), `single-threadgroup-fused-count-scan`
(C+), `radix-select-ordered-key-topk` (B−).
Lane B (lily, 5): `chunk-command-buffer-prefill`, `cross-level-fused-residual-norm`,
`RIF-sentinel`, `tensor-ops-device-q`, `weight-hygiene cluster`.

## Tasks

- [x] T0 Clone sources at pins into `.raw/` (gitignored; `core.longpaths=true`), verify SHAs + licenses; remove at landing.
- [x] T1 Dedup grep all 12 candidates against the corpus (both vocabularies) BEFORE any entry is authored; record verdicts.
- [x] T2 Spawn 2 parallel subagent lanes (A: uzu, B: lily) — re-verify quotes byte-accurate, full rule specs (pattern, decision content, bounded fix-space, category, file:line quotes), drop ledger.
- [x] T3 Coordinator re-verification of every landed quote + grade adjudication; kill/merge per dedup.
- [x] T4 Author corpus entries (Python heredoc generator, ensure_ascii=False, needle pre-verification) into `entries_gpu_20.rs`; bump all count pins (tests.rs totals/expected-array/sums/grouped, router.rs expert total, mod.rs doc line + category headers).
- [x] T5 Standing gates: `batch107_rules_present` + `batch107_concepts_covered` + `batch107_pseudocode_parses` (cfg(not(kernel_verify)) per B70) + one falsifiable content gate per novel mechanism where warranted.
- [x] T6 GOAT: kernel_opt lib + default lib + kernel_opt+latent (retrieval floors HELD, zero re-pins) + clippy 0 both states; walk every count pin.
- [x] T7 Research 130 note + AGENTS.md Batch 107 entry + kernel count line + `.docs/11_domains/kernel_opt.md` D-row + queue snapshot row consumed.
- [x] T8 Commit `feat: batch 107: ...` + push; `rm -rf .raw/*`; scratch cleaned.

## Defers / risks (resolved)

- lily `chunk-command-buffer-prefill` did NOT collapse into the B105/B26 family — it landed as its own rule
  (`prefill-chunk-command-buffer-capacity-coupled`) on the capacity-coupling decision content.
- lily `cross-level-fused-residual-norm` collapsed as predicted — RIDER on B63 (external confirmation + the
  bf16-ULP compiler-dependent contract).
- uzu `fused-count-scan` landed as its own H rule (the fused single-TG prologue shape was judged real content);
  carries the upstream hazard note in-rule.
- uzu upstream hazard `counts_shared[128]` OOB — CONFIRMED at the pin + a SECOND defect found (chunk-read missing
  `base`); both recorded in-rule + in Research 130; upstream report is owner-lane.
- T4 amendment: entries authored via direct `write_file` (whole-file), not the Python generator — the generator's
  value (needle pre-verification) was covered by the concepts gate, which caught one case-mismatched needle pre-commit.
