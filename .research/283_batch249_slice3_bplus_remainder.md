# Research 283 — Batch 249 slice 3: the TensorFold drift-pool B+ remainder (+15) + the delta riders (9)

**Status:** RECORD — slice 3 of Batch 249 LANDED (15 own rules, corpus 863 → 878; +9 count-neutral delta riders, 1 honest NO-FACE); **Plan 253 COMPLETE** (T1–T8 discharged).

## Provenance

Source: ashhart/TensorFold (Apache-2.0 since 0.6.0), the Batch-249 drift pool
`f1aef9c3..9ccf4727` — the same claim as slices 1–2 (plan 253, claim 2026-10-16
00:2x UTC, refreshed 2026-10-18T00:1xZ to expiry 2026-10-20T00:30Z, commit
`cb8ca0a0`). Slice 3 consumed:

- **The 9 originals** (read agents A/B, `.raw/b249/agentA_report.md` +
  `agentB_report.md`, written at the older pin `c7137bb31`): A-2, A-9, A-11,
  A-14, A-16, A-18 (agent A) + B-17, B-8, B-13 (agent B).
- **The 6 delta re-derivations** (D-A-2/5/6/7/8 + D-B-5): the delta read
  agents' reports were TRANSCRIPT-ONLY (lost with the slice-2 session), so
  this session re-derived them from `.raw/b249/delta_kernels.txt` (75
  commits) + `delta_host.txt` (306 commits) at the pin `9ccf4727`.
- **The 10 delta riders** → 9 landed + 1 honest NO-FACE (below).

**T4 verification:** three read agents + the coordinator re-verified every
quote at the pin working tree (checked out at `9ccf4727` this session — the
clone had sat at `9cd52ab4`; quotes were always verified via `git show`).
The only drifts were FOUR line-number shifts, zero text changes: A-16
nemotron_attention.cu 183→184; A-18 aot.zig 195-196→198-199; B-8
lane_host.zig 448→465, 475→478; A-9 nvfp4_experts.cu switch block 302-308
→ 303-309. All 17 commit hashes confirmed; all changelog-backed numbers
confirmed (CHANGELOG.md lines 131/142/157/158/169/172/201/207/248).

## What landed (15 rules, categories A +1 / B +2 / D +2 / E +3 / F +1 / K +1 / L +1 / N +4)

`entries_gpu_102.rs`:

| rule | cat | source commits |
|---|---|---|
| chain-rows-attend-own-keys-in-prefix-per-row-key-bound | F | 64cc7b01 (#712) + 6b5878f5 + 89836cc0 |
| nvfp4-row-tile-count-compile-time-switch-dispatch | B | 4e63c652 (#805) |
| canonical-module-key-any-spelling-disagree-refused | L | b27726a4 (#755) + 02764513 + 8c0c3f52 |
| per-sequence-lazy-graph-capture-on-own-buffers | D | 38787850d0 (#597/#615) |
| per-stream-buffers-by-value-seg-kernel-copies | E | da676b0d (#770) |
| triton-aot-variant-assume-less-fallback | K | 25690c3e (#791) + 462f1a15fa (#794) |
| fill-foreground-then-fewest-remaining-tokens | E | c4d07a549 (#651) + 3533f64f2 |
| pool-wait-bounded-bypass-later-fitting-goes-first | E | 1eaa2c531 (#107) + 6c151c408 |
| tokenizer-segment-memo-between-mutations | B | 2d2930d8b (#767) + 53646784c |
| dpas-block-scale-after-accumulate-not-in-operand | N | 9f0d4c79 + 10c2d9de (double landing) |
| one-barrier-a-tile-prefetch-occupancy-pair | D | f7336e8a (+01bc5e52 secondary) |
| rank-partial-fp32-travel-round-once-at-epilogue | N | 029bb341 + 47db917b |
| hi-lo-split-pow2-prescale-clears-subnormal-floor | N | 913cf15e |
| cross-rank-reduce-pins-rank-order-not-ring-order | N | d309dad9 + 4c04156c |
| paced-load-room-counts-free-plus-reclaimable | A | 779c38cc |

## The 9 riders (count-neutral, text-only)

kernel_opt gpu_concept riders (6): `lop3-int4-to-half2-dequant` (6b9d392e
+ e9bf19d9 — the twin-decode axis + the measured 16^-j-prescale underflow
negative), `weight-decode-amortized-across-output-columns` (dd7c516a — the
window axis + the expert-slot grouping face), `simdgroup-redundant-small-
reduction-over-barrier-broadcast` (77ae1501 — the scratch-reuse barrier
face), `mid-prefill-recurrent-state-capture-boundary-unsplit` (47e479e0 —
the DeltaNet wide-chain marks face), `identity-keyed-graph-retention-
stable-arena` (c67a8067 — the capture-boundary face: collectives run
between graphs, never inside one), `prefill-chunk-command-buffer-capacity-
coupled` (bee4b967 — the traffic-conditioned divisor face).

rust_perf context_notes riders (3): `prompt-state-fingerprint-probe-bits-
keyed-persistence` (807c6d42 — the within-build reproducibility face),
`metal-keepalive-residency-unwire` (4da6dda1 — the empty-tick face), and
`discretionary-cache-growth-keys-on-spare-not-available` (f875fe51/
25ef56c5 — the device-memory lazy-commit face).

**The NO-FACE:** `row-independent-pass-block-bounded-scratch` — no delta
commit genuinely matches (the scanned hits are other mechanisms:
5e8b0a9a/98f369a2 plan scratch, ee03f1d6/351a87e7 token-budget scratch,
ba9193f0 split-K scratch; eb9a9601 states the INVERSE trigger in a
cleanup). Disclosed, no rider authored — never invent.

## Gates

`batch249_slice3_rules_landed` + `batch249_slice3_rust_perf_riders`
first-run PASS (en-route: 2 shapes rewritten C++→Rust flavor — the parse
gate is a Rust-syntax pruner, the C++ verbatims live in comments; 1
needle-case fix); kernel_opt lib **3147 passed / 0 failed** (+2 new
gates); default lib 2687/0; clippy `-D warnings` clean at kernel_opt
`--tests` + kernel_opt,latent_retrieval `--tests` + default `--lib
--tests` — incl. the TWO pre-existing `too_many_lines` findings on
batch234/batch242 needle gates (disclosed in Research 282 as
pre-existing-at-HEAD; allow-only here with provenance comments — the
owning lanes' to refactor); retrieval (`kernel_opt,latent_retrieval`)
21 passed / 0 failed — blended **90.6 / 94.2 / 95.3** vs floors
89.4 / 93.6 / 93.6, kernel self-variants **100%** — **floors HELD, no
re-pin**. Freeze/score-bench NOT owed (zero clippy_lints corpus delta;
the rust_perf delta is context_notes text-only — P30).

## En-route incidents (disclosed)

1. **The trailing-comma parse error** — the first write of
   `entries_gpu_102.rs` ended `vec![...] ,` (a comma after the macro's
   closing bracket), failing at the LAST line with a misleading
   "found `,`". Bisected by cumulative-prefix rustc runs (~15 min; the
   first "head compiled clean" read was a /tmp-path artifact — rustc
   never read the file). All 15 entries were individually clean.
2. **The category-list mis-insert** — the B rules briefly landed in the
   N list (the second edit matched N's slice-2 comment instead of B's);
   caught by re-grep before any test run, repaired in the same session.
3. **The needle-case sweep** — a bulk replace of `"count-neutral",` also
   rewrote 3 slice-1 gate needles; caught by count (12 vs 9) and
   reverted in the same session.

## Batch close-out

Plan 253 **COMPLETE**: slices 1 (rider trio) + 2 (+15 own rules) + 3
(this slice) = the TensorFold drift pool fully converted at the pin —
corpus 848 → 878 (+30 own rules) + 13 count-neutral riders + 1 NO-FACE
disclosure. The B-grade remainder (D-A-21/A-21-class: device-resident
count in graphs, grammar bits one address, OR-offset unpack, row-map
gather, deferred weights, failed-round blast radius, the D-B B/B− set)
stays UNSTAGED — graded thin at slice 2; the honest close stands (the
B242 disclosure shape). Vendor clone left detached at `9ccf4727`;
`.raw/b249/` retained for the next reader.
