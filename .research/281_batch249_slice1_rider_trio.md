# Research 281 — Batch 249 slice 1: the TensorFold drift-pool rider trio (count-neutral)

**Status:** RECORD — slice 1 of Batch 249 LANDED (riders ×3, corpus STANDS 848); the batch's own-rule slices remain OPEN under Plan 253.

## Provenance

Source: ashhart/TensorFold (Apache-2.0 since 0.6.0), the Batch-249 drift pool
`f1aef9c3..c7137bb31` (1023 commits at claim) — **re-measured 2026-10-18 at the
T6 condition: the pool grew +770 mid-claim** (`c7137bb31..origin/main` =
770 commits, window total 1793; non-merge 596; 344 touch `zig/src`/`zig/kernels`).
Vendor clone `E:/git/vendor/tensorfold` fetched `origin/main 9ccf4727` this
session; every quote below verified at the pins via `git show`.

Batch plan: `.plans/253_batch249_tensorfold_drift_pool.md` (claim 2026-10-16
00:2x UTC, the 4090 idle loop). Read agents: `.raw/b249/agentA_report.md` (22
kernel-mechanics candidates) + `.raw/b249/agentB_report.md` (26 serving/cache
candidates) — the T1/T2 slice + read work, done 2026-10-16.

## What landed (slice 1 — the rider set, count-neutral)

Three riders, the Batch-247 shape (gpu_concept-only extensions + needle test,
no new entries, no ref/opt edits, floors re-measured and held):

1. **A-10 → `masked-in-place-field-prescale-other-operand`** (entries_gpu_98):
   the B242 GOAT's law carried to 2-bit codes in a Metal simdgroup-tile matvec
   (`c9673eaf` + `badfc9d6`, #784). The compound-offset prescale
   `as_type<float>(uint(127 - 2*s - 16*h) << 23)` — the offset a function of
   TWO indices (pass s, half h) — is the law's own boundary condition
   (offset = f(pass)) generalized to a two-axis pass lattice. Bit-equality
   witness: 1,516,773,088 pairs. The tile-window re-read face is already owned
   by `shape-only-plan-row-invariant-kernel` — declared, not double-counted.

2. **A-15 → `splitk-reduce-one-launch-slice-order-pinned`** (entries_gpu_67):
   the CONSEQUENCE axis the host's ANTI-REGIME ("atomics — nondeterministic add
   order") implied, now witnessed at training-ACCEPTANCE granularity
   (`4315bc2d`, #632): atomicAdd slice adds flipped a lesson's keep/remove
   checks run-to-run (the Lisbon witness); repair = parts buffer + slice-order
   sum kernel; the gate doubles as the bench (run twice, same bits).
   Workspace kinship: katgpt-rs Issue 940's determinism law (synced f32
   reductions pin fold order — a train-check flip is the acceptance face).

3. **B-5 → `zero-anchored-prompt-grid-chunk-identity`** (entries_gpu_82): the
   source SUPERSEDED its own grid-only era (`cc692e6f6`, #670) — the
   constructive-identity law held at EVERY cut (17-cut-pattern byte-equal
   dumps, sm_121/sm_120), so resumability widened to any row and the grid fell
   back to a chunk-length convention. The rider records the succession so a
   future reader cites the right era before claiming grid-only resumability.

Gates: `batch249_slice1_riders_landed` (needles ×26 + host-parse guards,
corpus STANDS 848); kernel_opt lib 3145 passed; retrieval
(`kernel_opt,latent_retrieval`) — kernel self-variants 100%, blended
90.6/94.2/95.3 ≥ floors, retrieval_eval 20 passed. Clippy clean at
`kernel_opt` + `--tests`. **Floors HELD, no re-pin; freeze/score-bench not
owed** (kernel_opt corpus text-only delta — the clippy freeze windows are
untouched; no new rust_perf entries).

## What remains (the batch's own rules — Plan 253 T3–T8)

The two read agents graded **22 own-rule candidates** beyond the riders
(A− ×4: A-1 in-place scan by slot-ownership, A-4 declared-thread-capacity +
capacity-assert, A-8 mod-13 symmetric-block test values + must-fail twin, A-12
launch-extent-from-actual-group-size — the silently-unwritten-output class;
B+ ×~16 incl. A-2 chain-attends-own-keys dispatch elimination, A-14
per-sequence lazy graph capture, A-16 by-value seg-struct batching, A-18
assume-less AOT fallback; serving A− ×5: B-1 branch-point-never-superseded,
B-2 born-supersede-attribution, B-4 copies-what-prompt-needs, B-12
verify-window-calibration-on-text, B-20 parser-backtrack-bounded-by-monotone-
cursor — LOCALLY BENCHABLE). Plus the +770 delta pool (T6 re-measure; early
enumeration found the 4-bit→tile width law `33fb0eae`, the 2-bit→half-pair
mantissa decode `e9bf19d9` — kin of the B242 GOAT family, the one-barrier-a-
tile vision attention `f7336e8a`, the GLM sparse-attention row-bound early
stop `18ae6758`, the on-device cast+position rows `dcaceaa9`, the stale-cache
expiry pair `5477b36d`+`e5b96869`, the living-weights stale-admission race
`4c36b4ac`). The next lane composes slice 2 from the agents' strongest-8
lists (protocol: re-fetch at T0, re-verify quotes at re-pinned hashes, dedup
greps 3+ vocabularies).

## Lessons

- The pool-growth-mid-claim hazard is now measured at +75% (1023 → 1793): a
  mega-window claim MUST re-measure at T6 before authoring own-rule slices —
  the staging pins were stale the moment the 1.0.12 merge wave landed.
- Rider-first slicing works: the read agents' RIDER adjudications convert to a
  landable count-neutral unit without blocking on the own-rule authoring
  session — the queue head (B250) unblocks 4h earlier than a full-batch wait.
