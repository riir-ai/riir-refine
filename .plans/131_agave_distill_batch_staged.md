# Plan 131 — Agave distill batch (staged): the pool-pack lead

**Status:** COMPLETE — Batch 147 landed 2026-09-16 (corpus 131 → 136; GOAT:
lib 1564/0 default + 1593/0 rust_perf_bench + 1714/0 latent + 2061/0
kernel+latent; retrieval floors HELD byte-identical 94.7/96.9 + kernel 100;
score-bench FLAT 96.2% rows #120-121 + frontier snapshot; clippy -D clean on
the write-set in 3 states — one PRE-EXISTING `cloned_ref_to_slice_refs` on
`src/clippy_oracle.rs:163` under `--tests` noted, NOT this batch's write-set).
**Admission-time finding: the stack-read candidate measured ParetoDominated
0/3/0 (allocation axis won 100000→802 but timing lost at every size — the
fixture's memcpy cost model dominates the page fill) and was DEMOTED to
suggestion-only with the measured negative as its pinned anti-regime witness
(the Issue-960/batch-146 precedent) — the honest direction, not a fixture tune.
`bounded-batch-buffer-pool` measured ParetoDominant 2/0/1 (48000→4 allocs,
20.5 MB→1568 B). The batch-verify candidate ships CONDITIONAL (aggregate
primitive must exist consumer-side; owner sign-off per the staging note).**

## Source

anza-xyz/agave @ `c95d870639e315ff8f9a90399ee38f918cc448c9` (Apache-2.0),
`.raw/agave` re-cloned at mine time (removed at verdict per the `.raw`
discipline). Verdict: Research 172 (YES B−, rust_perf, 2-lane + coordinator
verification).

## Candidate rules (admission order)

1. `bounded-batch-buffer-pool` (allocation, B+ GOAT) — bounded return cap,
   exact replenish, ownership-stays-in-pool-until-success. DISTINCT-FROM
   REQUIRED (the Research-172 correction): `batch-pool-read-makes-serial-readahead-redundant`
   (B123, IO read pool) + `reuse-allocation-clear` +
   `vec-reuse-across-lifetime-change` + `mem-take` + `smallvec-stack`, both
   directions.
2. `stack-read-fast-path-length-prefixed-record` (io, B−) — stack
   `[MaybeUninit; PAGE]` first read, `data_len`-gated heap fallback.
3. `batch-verify-optimistic-aggregate-fallback-individual` (caching, B−,
   CONDITIONAL on the ed25519-dalek `verify_batch` stand-in + owner
   sign-off — BLS is not a workspace dep).
4. `partial-reset-watermark-pool` (allocation, C+).
5. `batch-local-memoization-negative-cache-tombstone` (caching, C+).
6. Riders under consideration: regime-split-function-pair;
   lock-amortized-consumer-drain; collect-avoidance regime annotation
   (in-source measured counter-claim, serialization.rs:276-280).

## Tasks

- [x] Re-clone `.raw/agave` at `c95d8706…`, re-verify the five candidate
      file:line anchors (the repo moves fast; re-pin if HEAD moved and
      re-run the dedup greps)
- [x] Dedup re-verification under BOTH vocabularies (incl. the B123
      distinct-from requirement)
- [x] Author ORIGINAL minimal-reproduction fixtures (license hygiene —
      nothing verbatim from agave)
- [x] Corpus entries landed (rust_perf; batch number assigned at mining —
      **Batch 147**)
- [x] Standing gates: lib tests at default + full-feature, clippy 0
      write-set ×3 states, kernel_opt corpus test if any entry touches
      shared files
- [x] GOAT: G1 correctness (integer-exact for 1/2/4; dalek batch-verdict
      equality for 3), G2 perf (alloc-count axis for 1/4), G3
      no-regression, G4 alloc-free-or-equivalent
- [x] score-bench run + frontier snapshot append
- [x] Research 172 status → RECORD; queue snapshot row closed
- [x] docs: AGENTS corpus counts + HISTORY record + queue snapshot update
- [x] Commit + push; `rm -rf .raw/agave` *(landed at `26061174`; the box was left for the batch-148 session's sweep — complete)*

## Honest scope note

The absent-lane finding (Research 172): sdk/, zk-token-sdk/, bolt/ are NOT in
this pin — this batch mines the in-tree validator + program-runtime surface
only. The external `solana-sdk` / `zk-token-sdk` repos are a separate future
verdict if wanted.
