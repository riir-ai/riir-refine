# Research 148 — Batch 125: the ds4 PR rebase-surge distill

**Status:** RECORD 2026-09-07 — Batch 125 landed: kernel_opt +4 (555 → 559: A 117→118, D 128→129, E 45→46, N 37→38) + rust_perf io +1 (111 → 112, io 15 → 16), all NotBenchable-annotated where honest. Gates: batch125 4/4 FIRST-SHOT + lib default 1167/0 + kernel_opt 1483/0/1i + kernel_opt+latent 1600/0/2i + retrieval re-run kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical to B118–B124) + clippy 0 ×3 + score-bench #75 FLAT 96.2/1.9/100/100. Source `ivanfioravanti/ds4-metal` @ `ffd85d42` (MIT; fresh clone staged by Plan 106 T0, HEAD re-verified == ls-remote, quotes re-verified at the pin, clone removed at landing). Plan 106 / this note.

## The trigger was SMALL — the rebase-rewrite trap (the batch's own lesson)

Plan 106 was written from the GitHub compare `69c38d23...ffd85d42`, which reports **62 commits / ~19K ln incl. 625K insertions** — a phantom surge. The PR line was **REWRITTEN onto upstream main `c0a6119f`**, so the compare double-counts two already-banked populations:

1. The upstream-side window `9ab7053..c0a6119f` (DGX Spark, ROCm gfx1151, Metal SSD expert-cache, tool-text preservation) = **Batch 123's entire 41-file delta** (Research 146).
2. The Qwen3.8 stack below the old PR pin = B118/B121/B122's mines under rewritten SHAs (old `9f4313d` → new `32724a2`, old `69c38d2` → new `948edcd`, etc.).

The `c0a6119f..ffd85d42` window is 14 commits but is the WHOLE Qwen line (upstream never carried Qwen3.8), not the delta. The true new content is ~5 tip commits. **Standing rule reinforced: a compare between a force-pushed-away pin and a rewritten head counts NOTHING — derive the delta by message-level mapping against the mining records, then dedup-grep + quote-verify at entry time.** The old pin tree `69c38d23` is UNFETCHABLE (GitHub refuses force-pushed-away SHAs) — content-level rule-vs-banked verdicts had to be made at entry time.

## What landed (5 rules + 2 riders + confirmations)

| # | Rule / rider | Domain | Source commit | One-line decision content |
|---|---|---|---|---|
| 1 | `kv-reuse-prefix-ends-at-last-evaluated-token` | kernel_opt A | 658b2fc | A KV-reuse checkpoint keyed on rendered text ends at the last EVALUATED token — tool decoding that stops at `</tool_call>` (before `<|im_end|>`) must trim the terminator from the remembered prefix or the next turn consumes a KV hole; remember gated on the DECODE finish reason, remember-or-clear otherwise |
| 2 | `new-syntax-family-enumerates-every-keyed-surface` | kernel_opt D | ffd85d4 + 4010cc6 rider | Adding a model-syntax family requires extending EVERY keyed surface (tracker / prompt builders / parsers / CLI options); the failure faces are silent perf loss (KV checkpoint never saves), silent capability loss (vision flag unthreaded), and loud-at-user-time (option parser); POSITIVE per-family engagement tests, not no-crash tests |
| 3 | `interleave-independent-accumulators-only` | kernel_opt E | f91e217 | Under exact-parity gates, throughput restructure may interleave only accumulator-INDEPENDENT sums; every consumer-visible sum keeps its textual order — the ORDER axis at fixed precision (twin instances: Q4K multi-row dot + the chunked norm/inject hoist; the source's own comment: 'combining the chunk dots would change rounding') |
| 4 | `fixed-point-parity-needs-reference-rounding` | kernel_opt N | 681b9ce | A fixed-point reimplementation of a reference transform replicates the reference's COEFFICIENT PRECISION and ROUNDING MODE (JFIF `<<16` constants + round-half before `>>` + clamp); truncated low-precision shifts impose a −0.5-LSB directional bias that survives smoke and fails exact parity. DOMAIN CAVEAT carried in-rule: first CPU-side (codec) instance in this corpus |
| 5 | `control-reproduces-on-baseline-trims-overclaim` | rust_perf io | ffd85d4 | A failing parity/regression control is reproduced on the UNTOUCHED baseline FIRST; a reproduction indicts the ORACLE — trim it to consumed outputs under matching compute shape (prefill/decode split; unowned fused scratch), never widen tolerance |
| R1 | RIDER on B57 `smem-slot-role-reversal-needs-barrier` | kernel_opt | 034cc3d (drift lane) | PER-ITERATION MULTI-BUFFER face: per-chunk reduction slots (`inject_red[QWEN4_HC_CHUNKS][4][4]`) delete the back-edge barrier entirely — bit-identical by construction at tiny smem cost; extends B118's distinct-slot complement to every iteration |
| R2 | RIDER folded into rule 2 | kernel_opt | 4010cc6 | The config-surface face: `--ple` in the agent parser with a parse-round-trip test — three per-binary parsers sharing one engine-config struct, one missing member, loud exit 2 at user time |

Confirmations/already-banked (no entry): 948edcd = B122 `model-aware-format-over-hand-assembled-tokens` (rewritten SHA of 69c38d2); b98e009 = results-only (B121 provenance family); 4968b3d's speed-bench protos carry B118 shapes verbatim (`gqa_mma_proto.metal:459-462` = `compute-role-split-by-accumulator-pressure` + `role-scoped-store-must-match-compute-guard`; `t2d_probe.metal` = the operand-precision family); f91e217's `hc_gate_mix_pair` + `gdn_scan_r4 NSG` = B118's B77/B95 riders; 034cc3d's dispatch flips = the B118 break-even/dispatch family. Deferred (below threshold, recorded): the tool-delimiter escape round-trip (ds4_tool_text.h — single-source, revisit on a second instance); the speculative-rollback DRY consolidation; `%zu`→`PRIu64`.

**Lane note:** the two read lanes proposed 6 candidates total; the coordinator landed 5 rules + folded one rider and declined the escape-round-trip rule (single-source) — the dedup/threshold filter doing its job.

## The doc-lint cascade lesson (new standing class for the parse/clippy gates)

Adding the FIRST new `- ` bullet to corpus.rs's rule-list doc in a toolchain where `clippy::doc_lazy_continuation` + `doc_markdown` are active re-walks the ENTIRE `//!` module doc — and the **pre-existing run-on `+`-chain prose** (L17–31, assembled one "+ the X annotation" at a time across B118–B124) parses its line-start `+` as a LIST MARKER, flagging 30 lazy-continuation findings on lines the new bullet never touched. Fixed structurally, not by suppression: the chain's last link now reads `plus the evaluation-design annotation (...) plus the oracle-contract annotation (...)` (no line-start `+`), and `Batch 48...` got the blank line its sibling paragraphs already had. **Rule for future batch landings: when appending a `- ` doc bullet, never end a prose sentence on a line starting with `+ `/`- `/`* ` — close the chain with the word, not the symbol.** Drive-by: 5 `doc_markdown` backticks in B123's `entries_gpu_31.rs` header (`ROCm` `gfx1151` `DSpark` `CUDA` `Q4_K` `MXFP4` `rust_perf` — latent at HEAD, caught only now that the module doc re-linted under `-D warnings`; the B118 drive-by precedent).

## League intel

None new — the DSpark EOS-early-stop + ROCm gfx1151 rows are upstream-side (B123 window, already surfaced there). The Qwen MTP/IQ2 content in the rewritten commits is the B118/B121-mined material; no new league row.

## Provenance

- Clone: `.raw/ds4-metal` @ `ffd85d426313ace6dae805e9f2fb4424d5b427fd` (branch `qwen3.8-flash-next`, HEAD == ls-remote at stage time 2026-09-07 09:24 +07; LICENSE MIT verified; quotes verified via sed at the pin by both lanes + coordinator spot-check; clone removed at landing).
- Gates: batch125_rules_present + batch125_concepts_covered + batch125_pseudocode_parses (fn-wrapped snippets, the B122/123 lesson) + rust_perf batch125_rule_present_and_parses — 4/4 FIRST-SHOT; lib 1167/0 default, 1483/0/1i kernel_opt, 1600/0/2i kernel_opt+latent; retrieval 100/100/100 + 94.7/96.9/97.7 byte-identical; clippy 0 default/kernel_opt/kernel_opt+latent; score-bench #75 FLAT 96.2/1.9/100/100.
- Count pins walked: tests.rs totals 555→559 ×2 + per-category (A/D/E/N +1 each) + the batch116-era 555 pin + router.rs expert total 555→559 + rust_perf total 111→112 + annotated 54→55 (×2 sites) + mod.rs category lists + the corpus.rs doc count line (112/io 16/25 NotBenchable).
