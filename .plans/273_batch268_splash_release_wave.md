# Plan 273 — Batch 268: the splash release wave (incoai/splash be3b083c..1e532541)

**Status:** IN FLIGHT 2026-10-21 (shikuwa/4090, riir-refine idle loop).
**Claim:** shikuwa/4090 idle lane, expires 2026-10-23 ~08:2x +07 (48 h); push is the arbiter;
other lanes re-fetch before staging and yield to this row. The M3's Batch 269 landing row
(`3252e7694` era) already routed this pool to THIS lane ("B268 = the 4090 lane's splash pool").

## Source window

`incoai/splash be3b083c..1e532541` — 205 non-merge commits, the release/01-memory→11-cache-quota
wave (enumerated by the B267 standing-watch re-poll, Research 303 §STAGED). Prior splash pins:
B236 (mid-prefill capture boundary @ 711f8e5+c3dcf53+985463c+79a4329), B247 riders (KV-ALIKE +
retained-rows oracle @ 4919202 + 3903390, read at vendor clone `be3b083c5` — the wave's exact
base), B254 slice-2 face `597e5e50` (attention-row-mask-bitmap, uzu). Named leads to grade:

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

- [ ] T0 — clone splash to `.raw/splash` at the range; enumerate the 205 commits by
      release/NN area; dedup grep each candidate concept against the corpus
      (`src/domains/kernel_opt/` + rust_perf) BEFORE proposing; partition into 2 disjoint
      slices (memory/prefill-serving vs cache-quota/durability, exact split at T0).
- [ ] T1 — plan-first landed + claim pushed (this commit).
- [ ] T2 — Slice A (subagent): read its slice, propose rules with verbatim quotes + line
      numbers (pattern, bounded fix-space, category, rationale); every quote re-verified at
      the pin by the coordinator.
- [ ] T3 — Slice B (subagent): same, disjoint file set.
- [ ] T4 — coordinator: dedup adjudication (rider faces vs own rules vs drops with reasons),
      entry authoring (entries_gpu_117.rs NEW — 116 holds B267+B269), mod.rs wiring, count
      pins (every corpus-total site), batch268 needle gate.
- [ ] T5 — gates: kernel_opt lib; default lib; clippy −D ×3 postures (default / kernel_opt /
      no-default); retrieval floors re-measure (kernel_opt rules landed — the Issue-017 gate:
      kernel self-variant 100%, blended floors; re-pin only with attribution).
      freeze/score-bench NOT owed if zero clippy-corpus delta (P30); binary rebuild NOT owed
      (kernel_opt outside the release set).
- [ ] T6 — Research 305 + AGENTS.md corpus count bump + `.docs/11_domains/kernel_opt.md` B268
      entry + watch rows (ANE lag M3 intel; any remaining pool leads) in the SAME landing
      commit.
- [ ] T7 — snapshot landing row + commit `feat: batch 268: ...` + push; `.raw/splash` removed
      at close.
