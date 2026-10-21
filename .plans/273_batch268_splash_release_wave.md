# Plan 273 — Batch 268: the splash release wave (incoai/splash be3b083c..origin/main)

**Status:** LANDED 2026-10-21 (shikuwa/4090, riir-refine idle loop) — corpus 926→937 (+11 own kernel_opt rules, entries_gpu_117; the wave's rust_perf serving leads STAGED as Batch 270's pool).
**Claim:** shikuwa/4090 idle lane, expires 2026-10-23 ~08:2x +07 (48 h); push is the arbiter;
other lanes re-fetch before staging and yield to this row. The M3's Batch 269 landing row
(`3252e7694` era) already routed this pool to THIS lane ("B268 = the 4090 lane's splash pool").

## Source window

`incoai/splash be3b083c..origin/main` — **205 non-merge commits** (reconciled at T0: the
original `..1e532541` range-end was the B267 lane's mid-poll view; the 36-commit delta past it
carries ALL 11 remaining named leads — the Spill-series resume pair, publication-under-reader,
the chain-copy free/keep pair, disk relabel, cache floor, alloc-refusal, ANE lag, one-command
decode lanes, pendingGdnRows. Research 303's 205-count matches origin/main exactly). Prior
splash pins: B236 (mid-prefill capture boundary @ 711f8e5+c3dcf53+985463c+79a4329), B247 riders
(KV-ALIKE + retained-rows oracle @ 4919202 + 3903390, read at vendor clone `be3b083c5` — the
wave's exact base), B254 slice-2 face `597e5e50` (attention-row-mask-bitmap, uzu). Named leads
to grade:

- split-K sixteen ways on the register tile's narrowest grids (`6198321`)
- constrained+unconstrained decode lanes in one command (`f52d1ad`)
- QSA indexer q/k projection in one dispatch (`4944799`)
- Flash-Next down projection prepares its own table at decode (`55b0071`)
- auto PLE row cache 512 MiB + streamed experts (`18f9c69`) — DEDUP-ADJACENT to B265's
  `cache-budget-first-sessions-take-remainder-never-reshrink` + `kept-prompt-states-pinned-host-cache-budget`
- persistent cache emptied only on repeated/engine-reported failure (`6e21cc9`)
- eviction-count-gated re-encode (`2712e67`)
- the Spill series: page-at-a-time resume (`fa63d97`), anchor-bounded output (`593f2a6`),
  leaf-to-Spill-page only (`253506f`)
- publication-take-over-under-reader (`716760d`)
- dropped-point chain copies freed before the next point (`a20f344`)
- restore-point chain copies kept when a request recomputes (`65f9713`)
- disk-copy relabel at checkpoint-ordinary (`4fc7465`)
- max-cache-disk held while the volume keeps its floor (`51ae046`)
- host-step event written from one side only (`e2c0af2`)
- model-cache refused when the host refuses the allocation (`f6957e8`)
- ANE lag derived from what the test times (`26c6f1f`, M3 intel — watch row)
- pendingGdnRows moved to the test seam (`cfa91f5` — the B227/B247 oracle family's own lane,
  likely no new rule)

## Tasks

- [x] T0 — clone splash to `.raw/splash`; enumerate the wave; RANGE RECONCILED: the
      staged `..1e532541` was the B267 lane's mid-poll view — the 36-commit delta past it
      carries ALL 11 remaining named leads; scope = `be3b083c..origin/main` = exactly 205
      non-merge (the Research-303 count); dedup greps done (B265 cache-budget family,
      B236/B247 splash owners, splitk/register-tile/mask/verify families); slices split
      A (kernel, ~40 commits) + B (serving/memory, ~55 commits).
- [x] T1 — plan-first landed + claim pushed (`b75ac0dbf`).
- [x] T2 — Slice A (subagents, split 3+3 after two 6-rule stream-idle timeouts): 15
      proposals (11 kernel_opt strong + 4 rust_perf); 11 drops with reasons; full dedup
      ledger. Citations re-verified against the corpus (gpu_37 not gpu_56, gpu_67 not
      gpu_99, c-fragment not a-fragment) and applied.
- [x] T3 — Slice B (subagent): 11 full rust_perf proposals + 4 rider faces + 13
      second-tier + 5 drops, all grep-zero verified — STAGED as Batch 270's pool (NOT
      landed: freeze regen + binary rebuild is a lane of its own).
- [x] T4 — coordinator: 11 kernel_opt rules authored (entries_gpu_117 NEW; the a2 agent
      watchdog-cancelled post-write pre-validation — its 3 opt_patterns repaired to
      inline ref-shape shared items; 22/22 fixtures compile + pass standalone); mod.rs
      wiring (mod decl + extend + 7 category comments + 7 KERNEL_CATEGORIES rule-id
      lists); count pins 926→937 ×30 sites via ONE shape-asserted python pass (2
      historical narrative lines kept at 926); batch268 needle gate (FIRST-RUN PASS);
      the B269 landing's duplicated-phrase comment residue ("the vllm #61497 K lead"
      ×2) repaired in the 5510 re-pin, disclosed.
- [x] T5 — gates: kernel_opt lib 3208/0; default lib 2726/0; combo 3395/0 (3 ignored);
      clippy −D ×3 postures clean (5 doc_markdown healed by the cargo-refine healer,
      compile-gated; 8 secret-scan suggest-only findings = FALSE POSITIVES on kebab-case
      rule ids, disclosed); retrieval floors HELD 22/22 no re-pin (blended 90.6/94.2/95.3,
      kernel self-variant top-1 green); freeze/score-bench NOT owed (zero clippy-corpus
      delta, P30); binary rebuild NOT owed (kernel_opt outside the release set).
- [x] T6 — Research 305 + AGENTS.md corpus count (937) + `.docs/11_domains/kernel_opt.md`
      B268 entry + the Batch-270 staging row in the SAME landing commit.
- [x] T7 — snapshot landing row + commit `feat: batch 268: ...` + push; `.raw/splash` +
      slice/check scratch removed at close.
