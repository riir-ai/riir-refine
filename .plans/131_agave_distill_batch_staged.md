# Plan 131 — Agave distill batch (staged): the pool-pack lead

**Status:** STAGED — 2026-09-16 (verdict recorded, Research 172; batch runs at
a future batch window AFTER the staged Batch 146 bevy-0.19 charter; owner may
pull it forward)

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

- [ ] Re-clone `.raw/agave` at `c95d8706…`, re-verify the five candidate
      file:line anchors (the repo moves fast; re-pin if HEAD moved and
      re-run the dedup greps)
- [ ] Dedup re-verification under BOTH vocabularies (incl. the B123
      distinct-from requirement)
- [ ] Author ORIGINAL minimal-reproduction fixtures (license hygiene —
      nothing verbatim from agave)
- [ ] Corpus entries landed (rust_perf; batch number assigned at mining)
- [ ] Standing gates: lib tests at default + full-feature, clippy 0
      write-set ×3 states, kernel_heal corpus test if any entry touches
      shared files
- [ ] GOAT: G1 correctness (integer-exact for 1/2/4; dalek batch-verdict
      equality for 3), G2 perf (alloc-count axis for 1/4), G3
      no-regression, G4 alloc-free-or-equivalent
- [ ] score-bench run + frontier snapshot append
- [ ] Research 172 status → RECORD; queue snapshot row closed
- [ ] docs: AGENTS corpus counts + HISTORY record + queue snapshot update
- [ ] Commit + push; `rm -rf .raw/agave`

## Honest scope note

The absent-lane finding (Research 172): sdk/, zk-token-sdk/, bolt/ are NOT in
this pin — this batch mines the in-tree validator + program-runtime surface
only. The external `solana-sdk` / `zk-token-sdk` repos are a separate future
verdict if wanted.
