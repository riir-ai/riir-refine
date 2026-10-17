# Research 282 — Batch 249 slice 2: the TensorFold drift-pool own-rule set (+15)

**Status:** RECORD — slice 2 of Batch 249 LANDED (15 own rules, corpus 848 → 863); the batch's B+ remainder + the delta riders stage for slice 3 under Plan 253.

## Provenance

Source: ashhart/TensorFold (Apache-2.0 since 0.6.0), the Batch-249 drift pool
`f1aef9c3..c7137bb31` (1023 commits; read agents A/B, `.raw/b249/`) PLUS the
+770 delta re-enumeration `c7137bb31..9ccf4727` (read agents D-A/D-B this
session; kernel slice 75 commits, host slice 306 commits — reports in the
session transcript; the delta lists at `.raw/b249/delta_kernels.txt` +
`delta_host.txt` + `delta_all.txt`). Pin `9ccf4727` re-fetched at T0 this
session — the pool did NOT move after slice 1's re-measure (the earlier
in-session "7" rev-list read was a fetch-race artifact; the direct re-run
returned 770/1793, matching Research 281). **T4 coordinator verification: 25
needle quotes + line numbers re-verified at the pin via `git show` this
session** — the one initial MISS (contract_controls.metal) was a shell-quoting
artifact (the apostrophe in "row's"); the file and quote verified fine.

Batch plan: `.plans/253_batch249_tensorfold_drift_pool.md` (claim 2026-10-16
00:2x UTC; slice 1 landed 2026-10-18 ~03:5x +07). Slice 2 = the agents' A−
own-rule set (18 candidates → 15 entries after 3 folds).

## The folds (same law, multiple faces — one entry each)

1. **A-4 + D-A-3 → `pipeline-thread-capacity-declared-by-launcher`** — the
   `[[max_total_threads_per_threadgroup(N)]]` contract: launch-N declaration
   + the `maxThreads() >= N` capacity assert + the offline-compile pin
   (928-vs-1024). The silent-green failure class.
2. **B-20 + D-B-9 → `parser-backtrack-bounded-by-monotone-cursor`** — the
   quadratic-rescan bound: monotone cursor + per-marker next-occurrence
   memos + the 8x⇒24x linearity oracle (the slice's one locally-benchable
   rule).
3. **D-A-1 + D-B-10 → `length-keyed-dispatch-cut-invariance`** — a
   length-keyed dispatch boundary selecting different-numerics kernels makes
   a prompt's bits depend on the cut; one arithmetic at every length (the
   measured 3,108→851 tok/s price) or an explicit cut-invariance contract +
   the boundary-straddling oracle (cuts at 1/63/64/65/136).

## What landed (15 rules, categories D +8 / A +3 / N +2 / K +1 / B +1)

| rule | grade | source commits |
|---|---|---|
| pipeline-thread-capacity-declared-by-launcher (D) | A− | e73425a4d #704 + 5364047e2 #715 + 38349612/db391f17 |
| producer-launch-derived-from-actual-group-size (D) | A− | b12a6a03 #698 |
| length-keyed-dispatch-cut-invariance (D) | A− | 53dbc1d6 + 5d679ac7 + cd413796/5dc655b1 + 69fa1cbd |
| tp-checkpoint-byte-agreement-hash-while-loading (D) | A− | 0089ac9c + 0ce6bee2 |
| prompt-cache-branch-point-never-superseded (D) | A− | e17e2776c #735 |
| kept-state-born-supersede-attribution (D) | A− | 5035a8119 + 38423c4ca |
| prompt-cache-stale-invalidation-push-borrower-boundary (D) | A− | 5477b36d + e5b96869 |
| device-divergence-fail-closed-queue-drain (D) | A− | d22c6632 + 4c36b4ac |
| slot-keeping-scan-runs-in-place-publish-only-what-moved (A) | A− | 20b37cd6 #711 |
| kept-state-copies-what-prompt-needs-not-window (A) | A− | 97b0fea5a + 26f738ed1 + 401b57509 |
| residency-hold-gated-ram-fraction-not-working-set (A) | A− | 56800b60 + a2ecbecc |
| layout-check-values-mod-13-break-block-symmetry (N) | A− | 877f6d45 #744 |
| kernel-contract-planted-gate-controls (N) | A− | 8c21d099 + 569b47ff |
| calibrate-verify-window-on-text-not-proposer-chain (K) | A− | 2fd265568 |
| parser-backtrack-bounded-by-monotone-cursor (B) | A− | efbe595eb/b74e684a6 + fe4299b0/904e2bdd |

Domain routing: ALL kernel_opt (`entries_gpu_101.rs`) — the B242 mlx-serve
precedent (serving rules live beside the kernel rules in kernel_opt), avoiding
the rust_perf freeze/score-bench surface for this slice; the parser rule's
rust_perf option noted as a possible later migration.

## Gates

`batch249_slice2_rules_landed` (15 rules × needles + parse halves) first-run
PASS after entry-side verbatim-quote fixes (5 entries paraphrased where the
needles demanded the source's own words — the gate caught exactly the
drift it exists to catch); kernel_opt lib **3145 passed / 0 failed**;
default lib 2687 passed; clippy `-D warnings` clean at kernel_opt `--tests`
+ kernel_opt,latent_retrieval `--tests` + default `--lib --tests`;
retrieval (`kernel_opt,latent_retrieval`) 21 passed / 0 failed — blended
rows read 90.1/91.2/94.2 vs floors 89.4/93.6/93.6 — **floors HELD, no
re-pin**. Freeze/score-bench NOT owed (kernel_opt text-only delta; no
rust_perf entries — the plan-137-T7.5 condition is a clippy-corpus delta).

## Incidents (disclosed)

1. **The entries_gpu_100 collision** — slice 2's 15 entries were first
   written to `entries_gpu_100.rs`, which ALREADY held Batch 246's
   `shard-granularity-lcm-never-cut-rotation-block` (the session's initial
   `ls corpus | tail -20` truncated alphabetically — `entries_gpu_100`
   sorts BEFORE `entries_gpu_83`). Caught by a dead_code warning on
   entries_gpu_37 (a fuzzy edit had displaced its extend line in the same
   window); full recovery: the 15 entries moved to `entries_gpu_101.rs`,
   `git restore` on `entries_gpu_100.rs`, the extend chain re-ground
   against HEAD (gpu_37 restored between gpu_100 and gpu_36; no popcorn
   line — HEAD never had one). Final mod.rs diff verified: only intended
   lines (+101 mod/extend, category inserts, count comments).
   **Lesson:** the next slice checks `git ls-files` for the next filename,
   never a directory listing window.
2. **Pre-existing HEAD break (NOT this batch):** `cargo clippy
   --all-targets` fails E0601 on `benches/bench_009_f16c_lut_vs_intrinsic_goat.rs`
   (`fn main()` nested inside the arch-gated `mod x86_goat` whose closing
   brace never comes before main; verified present at HEAD via a stash
   probe — 2 E0601 occurrences with my changes stashed). The slice-1/246
   "clippy clean" postures were `--tests`-shaped, which is why it went
   unseen. Left unowned here — the bench_009 lane's to fix.

## What remains (slice 3, under plan 253)

- The B+ remainder: agent A's A-2 (chain attends own keys), A-9
  (compile-time tile-count switch), A-11 (canonical module key), A-14
  (per-sequence lazy graph capture), A-16 (by-value seg structs), A-18
  (assume-less AOT fallback); agent B's B-17 (fill foreground-then-fewest),
  B-8 (pool-wait bounded bypass), B-13 (tokenizer segment memo); delta
  D-A-2 (activation operand scale-fold underflow), D-A-5 (one-barrier tile
  occupancy cap), D-A-6 (rank-partial fp32 epilogue), D-A-7 (hi/lo pow2
  prescale), D-A-8 (cross-rank rank-order reduce), D-B-5 (paced-load room
  counts reclaimable).
- The 10 delta riders (count-neutral context_notes faces): D-A riders on
  lop3-int4-to-half2-dequant / weight-decode-amortized-across-output-columns
  / simdgroup-redundant-small-reduction-over-barrier-broadcast /
  mid-prefill-recurrent-state-capture-boundary-unsplit /
  discretionary-cache-growth-keys-on-spare-not-available /
  row-independent-pass-block-bounded-scratch; D-B riders on
  prompt-state-fingerprint-probe-bits-keyed-persistence /
  metal-keepalive-residency-unwire / identity-keyed-graph-retention-stable-arena
  / prefill-chunk-command-buffer-capacity-coupled.
- The D-A-21/A-21-class B-grade remainder (device-resident count in
  graphs, grammar bits one address, OR-offset unpack, row-map gather,
  deferred weights, failed-round blast radius, and the D-B B/B− set) —
  graded thin; slice 3 takes them only if the B+ set lands above
  estimate (the B242 disclosure shape).
