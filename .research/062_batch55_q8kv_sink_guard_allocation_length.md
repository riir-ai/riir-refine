# Research 062 — Batch 55: q8kv sink guard + the allocation-length class (2 measured-GOAT rules)

**Status:** RECORD
**Date:** 2026-08-18
**Source:** riir-ai Issue 716 / [Bench 691](../riir-ai/.benchmarks/691_q8kv_sink_guard_goat.md) (landed `e794b0033`, 2026-08-18) + Bench 642 / Issue 639 (2026-08-12) — both root-caused with fixes before distillation
**Corpus:** 349 → 351 kernel rules (N 19 → 20, D 63 → 64)

## What was distilled

The fourth measured-GOAT distillation after the riir-gpu corpus-complete
declaration (the B49/B52/B54 reopen path (c)): riir-ai landed the q8kv sink
guard the same day this cycle ran, with a complete bench record — the exact
shape the path calls for.

### Rule 1 — `outlier-span-escapes-quantization-via-lossless-sidecar` (N)

The arXiv:2608.12149 massive-activation failure mode confirmed LIVE on the
Gemma-2 q8kv decode path (M3 Metal, GPU-exclusive): 1-2 channels at ×100/×300
on sink pos 0 + a delimiter inflate q8kv attention output error **584×/18,844×**
over uniform data, concentrated at the poisoned 32-blocks (88×/299× vs clean
dims); CPU row-level neighbor collapse 58× — the KV-side twin of the Research
085/086 weight-side outlier→scale collapse.

Decision content:
- The rows carrying outliers are **known a priori** (attention sinks = oldest
  fixed positions; hybrid schedule boundaries enumerable from config) — no
  detection.
- Fix shape: the known span escapes quantization into a **lossless f32
  sidecar** (`[keys || values]`); the q8 buffers keep the rows as **dead data**
  so strides never shift; the kernel derives row count from the sidecar
  allocation length / stride; per-position branch in BOTH scoring and
  V-accumulation.
- A/B (the defend-wrong shape): sidecar **beats Hadamard rotation on both
  axes** — rotation spreads MA to absmax ≈ MA/16 (neighbors regain ~8×
  resolution, NOT exactness) at strictly larger kernel cost (rotated-Q buffer +
  sink accumulator + un-rotate cross-thread reduction). Sidecar: exact on the
  guard span (4.781 → 0.000698 = 6,850×; full sidecar ≡ f32 kernel at
  0.00000000), 0.96× perf (f32 reads replace byte-dequant loops).
- Off-state discipline: S=0 bit-identical to the unguarded launch (to_bits).

### Rule 2 — `buffer-length-derived-from-allocation-not-binding` (D)

A **two-occurrence** root-caused class (stronger than the usual single-fix
distillation — the class recurs):
1. **Bench 642 / Issue 639 (over-length):** `let dim = input.len()` reads the
   ALLOCATION — a one-row binding of a larger buffer made rmsnorm reduce x²
   over the FOLLOWING rows (pure scale error). Fix: both-ends binding trims
   (`offset_start(start).offset_end(tail)`); filed class-wide (5 ternary GEMV
   kernels shared `let m = output.len()`).
2. **Bench 691 / Issue 716 (zero-length-over-real-handle):** an "absent
   optional array" cannot be `from_raw_parts(handle.clone(), 0)` — cubecl
   derives the kernel-visible length from `handle.size_in_used()` (the
   `BufferArg` length is validation-only; `launch_unchecked` never sees it), so
   the 2048-f32 query handle derived `sink_rows = 1` and position-0 K/V
   silently became the QUERY buffer (uniform error 0.000268 → 0.12 across
   every test). Fix: a REAL 4-byte allocation that floor-divides to the
   semantic zero (1 f32 → 1/2/1024 = 0). Contract: **EXACT-sized** — oversizing
   changes semantics.
- Probe corollary (642, encoded inside the rule): a probe exercising only the
  benign case (the LAST row, where remaining allocation coincidentally equals
  dim) is **worse than no probe** — it converted an open question into a wrong
  answer. Probe row 0.

## Dedup kills (pre-proposal greps)

- Rule 1 vs `water-fill-tail-dim-shared-codebook` — bits-by-importance WITHIN
  one quantized representation vs EXCLUDING a row span into a lossless channel.
- Rule 1 vs `dequant-placement-by-component-traffic-share` (B51) — WHERE the
  dequant conversion happens per component vs WHAT representation each row
  span is stored in.
- Rule 1 vs `single-sweep-minmax-for-quant-scale` — scale computation vs the
  collapse being inherent to ANY per-block scale carrying a ×100 outlier.
- Rule 2 vs `launch-unchecked-caller-owned-size-contract` (B29) — eliding
  validation via documented caller contracts vs the caller CANNOT pass a
  length (the runtime derives it; fix is allocation-SHAPING).
- Rule 2 vs `empty-slice-sentinel-launders-shape-violation` (B43) — a sentinel
  passing validation into a swallowed Err vs nothing swallowed (wrong buffer,
  silently wrong numerics).
- Rule 2 vs `pad-uniform-pod-to-min-binding-size` (B30) — hardware binding
  minimums vs semantic length derivation.

## GOAT

- **G1 parse/syn:** `batch55_pseudocode_parses` PASS (both rules, both
  shapes); first-compile clean (generator method; all 23 anchors unique on the
  dry run — zero aborted attempts).
- **G2 lib tests:** 738 kernel_heal (B54: 735; +3 = the batch55 trio) /
  549 default / 163 corpus subset.
- **G3 no-regression:** clippy 0 in default + kernel_heal states (2
  doc_markdown backtick fixes pre-commit — `handle.size_in_used()` +
  `BufferArg` in `//!` header prose, the recurring identifiers-in-header
  gotcha); retrieval floors HELD with zero decay: kernel self-variants
  **100/100/100**, blended 3-domain **94.6/97.7/97.7** of 130 — identical to
  the Batch 53 re-pin.

## Honest notes

- The 716 landing itself stays **opt-in** in riir-ai (`q8kv_sink_guard` —
  enabling S>0 is a caller policy decision, the CLR/stealth precedent). The
  RULE distills the measured decision content regardless of promotion state —
  same as B52 (dflash fix) and B54 (riir-train fix).
- Bench 691's side finding (the zero-length trap) is the SECOND occurrence of
  the Bench 642 class but the FIRST distillation of it — 642's fix (both-ends
  trims) predates the corpus's D-category attention to derived lengths; the
  re-hit is what makes it rule-worthy now.
- Mined from COMMITTED files only (`attention_q8kv_cubecl.rs` + the two bench
  docs) — riir-ai's tree is otherwise dirty with sibling WIP (Batch 47
  precedent).
- `q8kv_sink_guard` promotion + the Kimi MLA port (T5) remain riir-ai
  consumer-side follow-ups; nothing queued here.

## Landing

- Corpus entries: `entries_gpu_06.rs` (rules 350-351; slice at ~870 lines —
  next batch opens `entries_gpu_07.rs` per the >900 rule).
- Count pins updated: 20 sites across `corpus/mod.rs` + `corpus/tests.rs` +
  `router.rs` + the `entries_gpu_06.rs` slice header.
- Batch 55 gates: `batch55_rules_present` + `batch55_concepts_covered` +
  `batch55_pseudocode_parses`.
