# 122 — Batch 142: the CloudRift "Outperforming vLLM and Llama.cpp on Gemma4-12B" distill (Research 163's kernel_opt batch)

**Status:** COMPLETE 2026-09-10 (the 09-10 cargo window; all tasks [x]; commits `5991eafa` batch+gates+pins + the research-RECORD commit; .raw/emmy removed). T0–T2 plan-first + zero-build halves landed 09-10 ~00:5x under the box gate; T3a authored same session; T3b–T6 executed 09-10 ~11:3x–12:1x when the 851 v9 bench exited and the window opened (the quiet-gate hold below is HISTORICAL — the bench fired/expired and T4 ran clean). **Hold RECLASSIFIED at plan time:** Research 163's original hold cited (a) the 085 dist lane untracked — **LANDED since** (`79dcf9b`, the owner reversal, 6th domain shipped) — and (b) sibling WIP in `entries_gpu_27..35.rs`. Re-verified 09-10: that WIP is 13h-stale MECHANICAL single-line import fixups (`use …::{KernelRule}` → brace-less, 9 files × 1 line, clippy single-item-import class), **disjoint from this batch's write set** (new `entries_gpu_39.rs`-class file + registry), and stays the sibling's uncommitted property — this plan neither touches nor commits it. The binding blocker for T3b+ was build contention, not tree ownership.

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
- [x] T3b — (CARGO-GATED) DONE 09-10: batch142 standing gates WRITTEN + GREEN (`batch142_rules_present` / `batch142_concepts_covered` / `batch142_pseudocode_parses` 3/3 — the static pre-clearance below held, one wrong host-id guess caught by the gate itself: `two-pass-split-softmax-long-context` → `split-k-two-pass-attention-decode-long-context`) + count pins LANDED (corpus.rs-header equivalent = mod.rs 605-rule comment + Batch-142 history paragraph; category-table comment counts A 130 / N 44 already wired by T3a; KERNEL_CATEGORIES slices verified consistent; mod.rs `entries.len` pin 605→607 in tests.rs ×2 + category totals ×2 + router total + the batch116 pin moved with its message; AGENTS ×2 (605→607); README ×2 (Batch 1-139→1-142, 605→607, Emmy ×3→×4); `.docs/11_domains/kernel_opt.md` seed-corpus row + Batch-142 ledger paragraph). **STATIC PRE-CLEARANCE 09-10 (zero-cargo review, plan-time session): all 8 string fields (2 rules × 4) escape-aware delimiter-balanced; field sets complete; vec structure closed; import already in the clippy brace-less single-item form (sibling's 27–35 fixups do not touch this file); mod.rs wiring + slices + counts verified consistent (605→607; A 130 @ mod.rs:1887, N 44 @ :2537, both rule_ids present in the correct slices); all 6 rider splices (02/07/18/36/38/39) end with intact `.to_string(),` terminators + RIDER attribution. The syn-parse hazard class was pre-cleared statically — T3b's real work at the window was the compile gates + the doc-surface pins, nothing else.**
- [x] T4 — (CARGO-GATED) DONE 09-10: GOAT G1 batch gates 3/3; G2 lib ALL states — default 1190/0, kernel_heal 1530/0/1i, kernel_heal+latent 1652/0/2i (290s), all-features 2235/0/17i (366s; the `91493a1` .parses fix HOLDS — the plan's expectation confirmed); G3 clippy 0 from the batch write set in all four states (default/kernel_heal/all-features-lib/+latent) — the residue is NOT this batch's: 2 pre-existing at HEAD (`rust_perf/corpus.rs:11944` `let mut feed` + one pedantic fn-length) + 4 WIP-introduced in sibling-overlapped files (corpus_gen e0502 44→103 lines by their table expansion, traj_store/cf trailing-`;`), isolated via a stash-baseline probe (stash→clippy→pop, 4-file scope, restored byte-identical); retrieval-precision re-measure — kernel 100/100/100 (9th consecutive), blended 94.7/96.9/97.7 byte-identical; router 92/131 = 70.2% (was 91/131 at B137) — floor re-pinned 86→87 per the Plan-030 −5 convention, delta recorded in the floor fn, never silent; rust_perf NotBenchable n/a (this batch is ALL kernel_opt; the NotBenchable-in-this-stack rationale is IN-RULE per the source-measured 5090 content); score-bench #93 FLAT 96.2/1.9/100 edits-64 vocab-38 artifact byte-identical `faa021c8…` (10th consecutive — kernel_opt corpus provably does not move the scored clippy/perf path); corpus-stats 607/124/49/15/31/4. **T4's Issue-085 rider: ALREADY DISCHARGED before this session** — the score-bench +dist twin (#92) + the condition-(4) consumer scans + B1/B2 landed at the 085 window sessions (Bench 082 + its §6 addendum; `759e209d`); remaining gate there is owner pull evidence ONLY, not a run.
- [x] T5 — (zero-build write, after T4 numbers) DONE 09-10: Research 163 →
      RECORD with the landing deltas (no new note needed; 163 closes in place;
      no `.research/.highwater` bump).
- [x] T6 — (mixed) DONE 09-10: landing sequence executed — commit authoring+gates
      (`5991eafa`, path-limited: entries_gpu_40 + mod.rs + tests.rs + router.rs +
      retrieval_eval.rs + AGENTS + README + kernel_opt.md + score_history.jsonl/svg)
      → docs/pins in the same commit → research RECORD (separate docs commit) →
      plan close (this) + queue snapshot live-state line (163 DISTILLED → Batch 142
      LANDED, same-commit below) → `.raw/emmy` rm → push. The sibling's 27–35
      import fixups + their wider WIP stay untouched and uncommitted.

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
target dir. **Tightened 09-10 03:32 (idle session, read the 876 record): the
stalemate is STRUCTURAL and owner-owned — round-the-clock agent activity makes
the 600s streak unreachable ("no clean windows exist while agents run"), the
knob-loosening path is explicitly NOT-recommended (Bench-649 contamination
class), and the unblock conditions are all owner calls. CONCRETE TIMELINE: v9's
deadline is 09-11 05:59 — on expiry the launcher ABANDONs per-length and the
quiet-gate pressure LIFTS (v10 relaunch = owner decision, not automatic).
T3b–T4's expected window therefore opens after 09-11 05:59 unless v9 fires
first or the owner intervenes; until then, waiting is the correct discipline,
not starved politeness.** When the box frees, run T2 → T3a → T3b → T4 in one
session per Protocol A. T3a (pure write) may ride earlier ONLY with T2's quote
verification done first.
