# 122 — Batch 142: the CloudRift "Outperforming vLLM and Llama.cpp on Gemma4-12B" distill (Research 163's kernel_opt batch)

**Status:** OPEN — plan-first landed 2026-09-10 ~00:5x (this session, zero-build prefix T0–T2 executed under the box gate: the 851 v9 quiet-gate stalemate + agent-B all-features builds make every cargo process a contamination event, so the cargo-gated halves T3b–T6 wait; the zero-build halves are done). **Hold RECLASSIFIED at plan time:** Research 163's original hold cited (a) the 085 dist lane untracked — **LANDED since** (`79dcf9b`, the owner reversal, 6th domain shipped) — and (b) sibling WIP in `entries_gpu_27..35.rs`. Re-verified 09-10: that WIP is 13h-stale MECHANICAL single-line import fixups (`use …::{KernelRule}` → brace-less, 9 files × 1 line, clippy single-item-import class), **disjoint from this batch's write set** (new `entries_gpu_39.rs`-class file + registry), and stays the sibling's uncommitted property — this plan neither touches nor commits it. The binding blocker for T3b+ is build contention, not tree ownership.

## Source (Research 163 = the batch spec; blog = quote authority)

- **Article:** cloudrift.ai/blog/optimizing-gemma-4-12b-rtx — "Outperforming vLLM and Llama.cpp on Gemma4-12B" (2026-08-01, Trifonov/Strumecki/Oleynikov). Fetched full-text 09-09 (scout).
- **Repo:** `cloudrift-ai/emmy` (Apache-2.0), tip at scout `321288d8` (4 past B138's pin `bdd26c53`); serving rev `d3df1f2a`. NOT cloned at plan time (zero-build discipline); fresh clone at T2 with HEAD == ls-remote re-verify.
- **Batch spec:** `.research/163_emmy_gemma4_hybrid_accumulation_verdict.md` — ranked table (2-rule floor + 4–6 rider adjudications), dedup gate, lane-intel NONE (triple-crossed), honest caveats (incl. the #761 post-article correction commit that must be read before quoting baselines).

## What lands (from Research 163, unchanged)

1. `fast-accumulate-periodic-promote-bounded-error` — **A− GOAT lead**: FP16-acc HMMA at 2× rate + f32-shadow fold every 64K elements; error flat ~3.3e-4 at every K vs pure-FP16's K-growing 6.0e-4→6.6e-3; provenance chain DeepGEMM/CUTLASS/Ootomo/Blanchard/Patterson; PyTorch `allow_fp16_accumulation` = the unprotected negative control. Nearest distinct: B127 `per-block-int32-chunk-rescale-fp32-accum` (scale-rescale ≠ accumulator promotion).
2. `smem-budget-single-buffer-refill-at-kill-points` — **B+**: FA d256 96/100 KB budget → single-buffer overlap by refilling at operand kill points; the boundary arm of the B79/B130 staging-depth family.
3. Riders adjudicated at mine: `roofline-gates-transport-upgrade` (rule-or-double-rider vs B138 SFU-limiter + B130 vendor-lag), split-KV occupancy-deficit (rider on B105), plugin-boundary launch-count (rider on B58/B124), dynM catalog gap (rider on B104/B128), accuracy-gate-refuses-fused-form (vs B116), geomean/wave-vs-drain median (rust_perf rider — dedup re-verified clean 09-10).

## Tasks

- [x] T0 — Plan-first (this file); `.highwater` 121 read, folder re-scanned at write (121 = Batch 141, latest).
- [x] T1 — Dedup RE-VERIFIED against the current corpus 09-10 (post-B141 + post-16eaa32):
      `shadow.?accumulat|f16x2|cvt\.f32\.f16|ootomo|deepgemm` → only B138's P→A repack
      (`entries_gpu_38.rs:64` — a register layout handoff, not accumulation) + B130's
      flag-split rider; `kill.?point|staggered.?refill` → zero hits; promotion-class
      grep → no accumulator-promotion coverage (B127 named distinct re-confirmed at
      `entries_gpu_33.rs:47`); `geomean|wave.?vs|median.?inversion|queue.?drain` in
      rust_perf → zero hits (methodology rider clean). Verdict unchanged: **zero
      coverage of both floor rules.**
- [x] T2-spec — Lock the T2 scope from Research 163's caveats: fresh clone at
      then-tip (HEAD == ls-remote), read `experiments/gemma-4-12B/` +
      `scripts/bench_accum_error.py` (re-read or re-run the error harness),
      read #761 BEFORE quoting any baseline number, re-verify every quoted line
      against the live article text + repo files at the pin (B31 discipline).
- [x] T2 — (zero-build, network) DONE 09-10: fresh clone `cloudrift-ai/emmy` → `.raw/emmy` at
      tip `321288d8` (HEAD == ls-remote verified); #761 (`e0d16cf0`) read BEFORE quoting —
      its substance is golden/README.md:18-21+33 (the 5090 attention Emmy lane has NO
      recorded goldens; the old 20015µs blocker no longer reproduces at bdd26c539 = 18.6µs);
      quote re-verification executed as TWO parallel lanes (16 article claims via the live
      full text — ALL VERIFIED; 15 repo quotes line-verified at the pin — 14 EXACT + 1
      line-attribution fix). Soft spots honored: the plugin-boundary percent self-contradicts
      (2-5% intro vs 1-5% conclusion → cite the conclusion); "dynamic-M" is our gloss of
      `dynM`; "fail-closed admission" is our vocabulary.
- [x] T3a — (zero-build write) DONE 09-10: authored `entries_gpu_40.rs` — NOT 39: the plan's
      "file number free" expectation was stale at write time (Batch 139 = `entries_gpu_39.rs`
      landed post-plan, 72c10f7); re-scan at write caught it. 2 floor rules
      (`fast-accumulate-periodic-promote-bounded-error` N, `smem-budget-single-buffer-
      refill-at-kill-points` A) + 6 riders landed (02 launch-count, 07 accuracy-refusal,
      18 splitkv-trigger, 36 vendor-lag + stale-blocker corollary, 38 roofline-gated
      transport, 39 reporting axis). RIDER ADJUDICATIONS: roofline-gates-transport-upgrade
      → double-rider (38 SFU rule + 36 vendor rule) per its rule-or-double-rider grade;
      geomean/wave-vs-drain → the B139 drift-protocol rule (kernel_opt, not rust_perf —
      the target rule lives there). **DEFERRED [-]:** the dynM catalog-gap rider (B104/B128)
      — its target `entries_gpu_34.rs` carries sibling WIP (mechanical import fixups);
      lands with the next batch touching that file. mod.rs wired (decl + concat + slices
      A 129→130, N 43→44 with count comments).
- [ ] T3b — (CARGO-GATED) batch142 standing gates (`batch142_rules_present` /
      `batch142_concepts_covered` / `batch142_pseudocode_parses`) + count pins
      (corpus.rs header ×2, category table, KERNEL_CATEGORIES slices, mod.rs
      history paragraph + entries.len, AGENTS ×2, README ×2, `.docs/11_domains/`).
- [ ] T4 — (CARGO-GATED) GOAT: G1 batch gates by name; G2 lib in all feature
      states; G3 clippy 0 both/all states + retrieval-precision re-measure
      (floors: kernel 100/100/100, blended — re-read the current pins at run
      time; a decay FAILS the batch or re-pins with the delta recorded, never
      silent); rust_perf NotBenchable annotations + context_notes for the
      decision-content rules (this batch is ALL source-measured 5090 content —
      expect every rule NotBenchable-annotated); score-bench + corpus-stats.
- [ ] T5 — (zero-build write, after T4 numbers) Research note: Research 163 →
      RECORD with the landing deltas; `.research/.highwater` bump if a new note
      opens (else 163 closes in place).
- [ ] T6 — (mixed) Landing sequence: commit authoring+gates → docs/pins →
      plan close + queue snapshot (live-state line moves 163 DISTILLED →
      Batch 142 LANDED) → `.raw/emmy` rm → push. Commit path-limited (the
      sibling's 27–35 import fixups stay untouched and uncommitted).

## Non-goals

- No M3/Metal lane claim from this batch (Research 163: the hybrid-accumulation
  content is CUDA-mma-specific; riir-gpu scored lanes are int8-mma/int32-accum —
  no lane reopen; corpus value only).
- No league row (lane-intel NONE triple-crossed; watch-only line stays in 163).
- No touching `entries_gpu_27..35.rs` (sibling's uncommitted import fixups).

## Hold/unblock (the operative state)

T3b–T4 wait for a cargo window: the 851 v9 quiet gate is in an editor-hot
stalemate (see riir-ai `.issues/876` 09-10 note — 169 refusals/14.5 h) where
every cargo process resets the gate's streak, and agent-B sessions hold the
target dir. When the box frees (v9 fires or expires, agent builds drain), run
T2 → T3a → T3b → T4 in one session per Protocol A. T3a (pure write) may ride
earlier ONLY with T2's quote verification done first.
