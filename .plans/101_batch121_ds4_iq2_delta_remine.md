# Plan 101 — Batch 121: ds4 PR #991 delta re-mine — the calibrated IQ2_XXS build commit (`c5ae2eb5`)

**Status:** RECORD — Batch 121 landed 2026-09-07 (3 rust_perf io rules, corpus 102 → 105, all NotBenchable-annotated; GOAT green: batch121 gates 3/3 first-shot, lib 1159/0, kernel_heal+latent 1589/0/2i, retrieval floors byte-identical, score-bench FLAT, clippy clean; commit on `develop`; `.raw/ds4-metal` removed). Yield matched the plan's expectation exactly (3 rules + 1 rider-folded-in; no kernel_opt entries — no kernel code in the delta).

## Source + provenance

- **Commit:** ivanfioravanti/ds4-metal `c5ae2eb5` "Add calibrated Qwen IQ2 build and four-recipe quality comparison" (2026-09-06T18:42:55Z; MIT; `.raw/ds4-metal` cloned + HEAD verified == pin + LICENSE verified MIT).
- **Delta surface (6,734+/3−, 5 files):** `gguf-tools/qwen4_iq2.py` (NEW, 181 ln — two-stage quantize/assemble tool) · `speed-bench/qwen38-iq2-quality.md` (NEW, 62 ln — four-recipe quality + memory honesty) · `speed-bench/qwen38-iq2-quality-results.json` (NEW, 6,435 ln machine-readable results) · `docs/QWEN38_FLASH_NEXT.md` (+15/−3) · `gguf-tools/README.md` (+41).
- **Model family:** Qwen3.8-Flash-Next (as Batch 118). No kernel code in the delta — build tooling + measurement methodology.

## Candidate rules (T0 read; dedup greps run 2026-09-07)

Dedup: no corpus rule covers these axes. Nearest families checked and DISTINGUISHED: `commitment-hashing`-family (gpu_03 — digest INPUT design vs this build-RESUME identity), quantization placement family (gpu_08 state-horizon + dequant placement — runtime placement vs this CALIBRATION workflow), io family (B94/B118 — access patterns vs build orchestration).

1. **`resume-revalidates-provenance-before-reuse`** — a resumable multi-artifact build must revalidate EVERY identity input (source checkpoint, calibration/imatrix hash, tool/library hash) against the on-disk manifest BEFORE reusing cached intermediates; mismatch ⇒ refuse (`Resume provenance mismatch: {key}`), and per-artifact reuse re-verifies the stored digest (`digest(...) == report['tensors'][n]['sha256']`). Failure class: a stale TOOL or swapped SOURCE resumes "successfully" and silently mixes outputs across tool versions.
2. **`write-verify-readback-before-publish`** — a multi-part artifact assembly writes `.incomplete`, then RE-READS every payload through an INDEPENDENT reader and hash-compares against the plan BEFORE the artifact exists (`Written tensor mismatch` refuse), then atomic-rename publishes + emits a sidecar manifest (output SHA, per-tensor records); refuses existing outputs; template guards (architecture kv, block count, external-PLE contract, per-tensor kind/dims) run BEFORE any byte is written.
3. **`isolate-one-variable-by-byte-copying-baseline`** (measurement honesty) — to measure ONE component's precision/behavior change, byte-copy every OTHER component from the baseline artifact and replace ONLY the under-test set ("This isolates the gate/up precision change"); report PAIRED differences (IQ2-alone vs baseline-alone), keep the confounded recipes (Q8 = Q8 down weights too; Q4_0 = earlier recipe) out of the headline comparison ("The most controlled quantization comparison is IQ2 versus Q4_K-imatrix"), and refuse to generalize from small fixed suites ("do not establish general quality equivalence").
4. **RIDER (memory-honesty context note):** peak RSS / footprint "do not reliably account for all mmap-backed GPU weights and must not be interpreted as the total model memory requirement" — file-size + tracked-runtime-allocations = a PARTIAL budget; the host RAM total doesn't validate another machine's headroom. Home: context note on rule 3 (same measurement-honesty surface).

Expected yield after falsification: 2–3 rules + 1 rider. All build-orchestration/measurement-shaped → **rust_perf io/tooling category, annotated NotBenchable** (the B94/B118 precedent — no microbenchable kernel surface).

## Tasks

### T0 — provenance (DONE 2026-09-07 04:2x)

- [x] Fresh clone `.raw/ds4-metal` @ `c5ae2eb5` (shallow, branch `qwen3.8-flash-next`); HEAD verified == pin; LICENSE == MIT.
- [x] Quote re-verification against the clone: `Resume provenance mismatch` (:46), `Output already exists` (:80), `Template must be` (:87), `Written tensor mismatch` (:153), `ds4q_quantize_init(16)` before workers (:39), the RSS/mmap caveat (quality.md:52), the controlled-comparison framing (quality.md:37). All present verbatim.
- [x] Upstream main re-verified `9ab7053` (the PR API `base.sha`).

### T1 — dedup + adjudication

- [ ] Corpus greps for each candidate concept (resume/provenance/readback/publish/atomic/isolate/mmap-RSS) — DONE at T0, recorded above; final per-entry vocabulary probes at authoring time.
- [x] Adjudicate rule count (3 rules + 1 rider folded into rule 3's notes; no drop-table kills — all candidates survived the dedup).

### T2 — entry authoring

- [x] Author the three rust_perf entries (io category, NotBenchable-annotated per B94/B118) with ORIGINAL fixtures, decision content + regime conditions in-rule, the memory-honesty rider + calibration-provenance rider in rule 3's context notes.
- [x] Authoring discipline held (original fixtures, never pasted source; needle-bearing rule text for retrieval).

### T3 — gates + count pins

- [x] Standing gates: `batch121_rules_present` / `batch121_pseudocode_parses` / `batch121_concepts_covered` (incl. verbatim source-phrase pins) — 3/3 FIRST-SHOT.
- [x] Count-pin inventory walked: corpus header (102→105, io 6→9, NotBenchable 15→18), member-list tail (+3), benchable line (87/102→87/105), build_corpus doc line (101→105), total pin, annotated pins ×2 (45→48), serde pin (45→48).
- [x] FULL-STRING pins: none ridden (no existing rule's text grew).

### T4 — GOAT

- [x] G1 parse/syn 3/3; G2 lib 1159/0; G3 clippy 0 in default + kernel_heal lib states; retrieval re-run kernel 100/100/100 + blended 94.7/96.9/97.7 HELD (byte-identical to B118/119); kernel_heal+latent lib 1589/0/2i (floor 1574 held).
- [x] score-bench FLAT 96.2/1.9/100 (row appended UTC 21:37Z).

### T5 — records + landing

- [x] Research 144 (Status → RECORD).
- [x] AGENTS.md rust_perf count 102 → 105 (both sites) + `.docs/11_domains/rust_perf.md` corpus section + snapshot queue line LANDED + the re-armed trigger line.
- [x] League intel recorded in Research 144 + the snapshot (named-not-edited; no league manifest edits).
- [x] Commit + push on `develop`; `rm -rf .raw/ds4-metal` at landing.
- [x] Plan → RECORD (this flip).
- [ ] Plan → RECORD.

## Non-goals

- No kernel_opt entries (no kernel code in the delta).
- No league manifest edits (named-not-edited discipline).
- The PR stays OPEN — the next trigger re-arms automatically (a further head move past `c5ae2eb5`, or upstream main past `9ab7053`).
