# Plan 287 — Batch 283: the prism b10819 window leads (kernel_opt / rust_perf)

**Status:** LANDED 2026-10-25 — Batch 283 discharged (see the snapshot landing row; expiry 2026-10-27 ~13:3x +07 never reached)

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

- [x] T0: read both diffs at the pin in full; adjudicate rule / rider / drop per lead; license
      posture re-check (fork MIT lineage re-verified this window at the bout pre-stage)
- [x] T1: dedup grep (3+ vocabulary variants per lead) against kernel_opt 969 + rust_perf 270
      BEFORE authoring — named kin checks: #391 vs popcorn `fuse-add-rms-norm` (B280 exact-alias
      face), B116 FWHT ladder + the B181 FWHT-fusion family, B208 admission family; #393 vs
      B265 graph-sticky pool-free invalidation, B257 pdl admission, `one-event-per-writer` /
      batched-dispatch families; honest drops allowed
      (VERDICT: zero prior owners — #391 → new kernel_opt D rule; #393 → new rust_perf caching
      rule + the count-neutral upload rider on chain-dispatches-single-sync-point; the
      graph-sticky family member's real id is per-request-dirty-range-clear-graph-capture-sticky-marks)
- [x] T2: author entries — fixtures standalone-compile + execute; every quote byte-verified at
      the pin (git show reads, no local-edit drift)
      (kernel ref 2/2 + opt 11/11; rust_perf buggy 2/2 + fixed 4/4, rustc --edition 2021 --test -O)
- [x] T3: wire KERNEL_CATEGORIES / mod.rs; batch283 needle + parse gates; count pins via ONE
      shape-asserted python pass (tests.rs sites + router.rs + mod.rs + AGENTS.md + .docs)
      (25 multi-line + 10 single-line + router expert-total + the rust_perf 271/150/218 sites;
      EN-ROUTE: the batch249_slice3 cross-domain pin repaired 267→271 — it missed B282's +3)
- [x] T4: gates — kernel_opt lib, default lib, clippy −D ×3 postures, retrieval floors
      RE-MEASURE (corpus delta owed); freeze/score-bench NOT owed if kernel_opt-only (P30);
      maintained binary rebuild OWED iff rust_perf surface moves (the §0 law), NOT owed if
      kernel_opt-only
      (kernel_opt lib 3232/0; default lib 2731/0; latent lib 2917/0, retrieval floors HELD
      124/130/133 ≥ 123/130/132 no re-pin; clippy −D ×3 clean; score-bench #200 heal 0.9636
      FLAT parse 1.0 + frontier gen-7; binary rebuilt at the release set + installed)
- [x] T5: Research note 320 (`.research/.highwater` 319→320) + AGENTS.md count bump + .docs
      ledger + snapshot landing row; `.plans/.highwater` stale-285 repair (→287) rides the claim
      commit
- [x] T6: commit + push (claim row discharged by the landing row; `.raw/` cleanup if any)

## Notes

- Zero-GPU throughout — corpus gates are CPU lib tests. Box verified quiet at claim
  (GPU 5% / 324 MiB / no compute processes; the b10819 bout landed 13:2x, sibling lanes idle).
- Walk #17 opens 2026-10-26T00:00Z (claimed by this lane, expiry 2026-10-27 ~04:3x +07) — it
  takes research 321+; this batch's note is 320.
- CRLF discipline: the snapshot file is natively CRLF — the claim row appends via binary-safe
  write, never a whole-file rewrite.
