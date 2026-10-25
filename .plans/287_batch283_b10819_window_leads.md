# Plan 287 — Batch 283: the prism b10819 window leads (kernel_opt / rust_perf)

**Status:** IN PROGRESS — claimed 2026-10-25 ~13:3x +07 (shikuwa/4090 riir-refine idle loop; expiry 2026-10-27 ~13:3x +07; push is the arbiter)

Source: PrismML fork scored tag `prism-b10819-909f0ca` (`909f0cac9c`), window = the scored-tag ladder
`b3ebc47..909f0ca` (10 commits). The staged B283 leads per the 10-25 ~11:2x watch row (the
sm_89-relevant content of the b10819 window — both measured league-movers in Bout 1000's
opp-side +2.11% decode move):

- **#391** `e75baeeb24` — fuse RMS_NORM (+ residual ADD) with the SIGNED Hadamard transform at
  both norms of every layer; bit-identical; +4.8-5.4% decode on H200; plus the layer-start
  look-ahead. Their 27B-neutral datum was REFUTED at our geometry (Bout 1000 names #391 in the
  opp move) — the rule must carry both data.
- **#393** `63f16c408f` — cut ~55-65 µs of host time per decode step: batched sched input
  uploads + no node scans on CUDA graph replay; byte-identical; all-arch DEFAULT.

The rest of the window is already adjudicated: #387 measured ZERO at sm_89 (Bout 1000,
opt-in/Hopper; the Bench-999 anticipation held); #394 Hopper-only; #392/#325 CPU x86/Turing
(class watch); #383 opt-in; #389/#400 sampler host-time (adjacent to the B281-era sampler
coverage — dedup grep decides); #296 tests.

Clone: `E:/git/vendor/prismml-llama.cpp` ALREADY at the pin `909f0cac9c` (bout pre-stage
checkout; rebuilt llama-bench.exe present) — read-only, no fetch, no .raw needed.

## Tasks

- [ ] T0: read both diffs at the pin in full; adjudicate rule / rider / drop per lead; license
      posture re-check (fork MIT lineage re-verified this window at the bout pre-stage)
- [ ] T1: dedup grep (3+ vocabulary variants per lead) against kernel_opt 969 + rust_perf 270
      BEFORE authoring — named kin checks: #391 vs popcorn `fuse-add-rms-norm` (B280 exact-alias
      face), B116 FWHT ladder + the B181 FWHT-fusion family, B208 admission family; #393 vs
      B265 graph-sticky pool-free invalidation, B257 pdl admission, `one-event-per-writer` /
      batched-dispatch families; honest drops allowed
- [ ] T2: author entries — fixtures standalone-compile + execute; every quote byte-verified at
      the pin (git show reads, no local-edit drift)
- [ ] T3: wire KERNEL_CATEGORIES / mod.rs; batch283 needle + parse gates; count pins via ONE
      shape-asserted python pass (tests.rs sites + router.rs + mod.rs + AGENTS.md + .docs)
- [ ] T4: gates — kernel_opt lib, default lib, clippy −D ×3 postures, retrieval floors
      RE-MEASURE (corpus delta owed); freeze/score-bench NOT owed if kernel_opt-only (P30);
      maintained binary rebuild OWED iff rust_perf surface moves (the §0 law), NOT owed if
      kernel_opt-only
- [ ] T5: Research note 320 (`.research/.highwater` 319→320) + AGENTS.md count bump + .docs
      ledger + snapshot landing row; `.plans/.highwater` stale-285 repair (→287) rides the claim
      commit
- [ ] T6: commit + push (claim row discharged by the landing row; `.raw/` cleanup if any)

## Notes

- Zero-GPU throughout — corpus gates are CPU lib tests. Box verified quiet at claim
  (GPU 5% / 324 MiB / no compute processes; the b10819 bout landed 13:2x, sibling lanes idle).
- Walk #17 opens 2026-10-26T00:00Z (claimed by this lane, expiry 2026-10-27 ~04:3x +07) — it
  takes research 321+; this batch's note is 320.
- CRLF discipline: the snapshot file is natively CRLF — the claim row appends via binary-safe
  write, never a whole-file rewrite.
