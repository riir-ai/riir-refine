# Research 172 — Agave distill verdict (the pool pack lead)

**Status:** DISTILLED — pending owner decision (verdict recorded 2026-09-16;
batch staged behind the bevy-0.19 charter, Plan 131)

## Source & provenance

- **anza-xyz/agave @ `c95d870639e315ff8f9a90399ee38f918cc448c9`** (HEAD
  2026-09-15, shallow clone), **Apache-2.0** (LICENSE at root). The Solana
  validator monorepo — production-hardened, criterion+bencher deps,
  `[profile.release-with-lto]` fat LTO + codegen-units=1, per-package
  `opt-level=3` dev overrides for ed25519 (Cargo.toml:604-607, :576-583) —
  they benchmark their hot paths.
- 2-lane inspection (the large-source pattern): Lane A validator hot path
  (core/, runtime/, accounts-db/, poh/, streamer/, banking-stage-ingress-types/,
  measure/, rayon-threadlimit/, bucket_map/, bloom/) + Lane B sdk/execution
  layer (svm/, program-runtime/, cost-model/, vote/, account-decoder/,
  bls-sigverify/, storage-*, send-transaction-service/, net-utils/).
  Every CRITICAL reachability claim was re-verified by the coordinator's own
  reads (the Research 057 lesson) — all confirmed, one dedup evidence line
  CORRECTED (see the B123 row below).
- `.raw/agave` removed at record per the `.raw` discipline; the batch re-clones
  at the pinned sha ("re-verified at mine time").

## ⚠ Absent-lane finding (load-bearing for any future agave grep)

~60% of Lane B's charter is **ABSENT from this snapshot**: `sdk/` is a one-file
README pointing at the external `anza-xyz/solana-sdk` repo; `zk-token-sdk/` is
empty (moved out likewise); `bolt/` does not exist at this pin (the bytecode
VM lives elsewhere; only `rbpf-cli` remains in-tree). The hottest primitives
(inspectable in neither lane) also live in external crates: the PoH SHA-256
loop (`entry/`), ed25519 batch SIMD verify + Deduper (`perf/`), scheduling
bitmasks (`scheduling-utils/`). A future verdict on those needs NEW clones of
the external repos with their own pins.

**Vocabulary trap:** "SIMD" throughout agave = Solana Improvement Proposal
(e.g. `MAX_INSTRUCTION_STACK_DEPTH_SIMD_0268`), NOT SIMD instructions. Zero
`std::arch`/`_mm_`/`target_feature` hits in the whole inspected surface.
Grep `target_feature|std::arch|_mm_` only.

## Verdict (composed, both lanes)

**YES — grade B−. Route `rust_perf` (allocation / io / caching).** NO
lane-intel mapping (the open riir-ai lanes are cuBLAS/GPU-bound — 4090
prefill Issue 918 is cp.async GEMM; CPU validator patterns do not map; no
issue matches pool/recycler/madvise/slab keywords). Tempting WRONG domains
named and rejected: `kernel_opt` ("validator" sounds kernel-adjacent; every
candidate is plain CPU Rust, and the kernel_opt file-mining frontier is
CLOSED) and a hypothetical `sec` domain (the BLS material LOOKS
security-shaped; the candidates are perf patterns; sec does not exist).

## Ranked candidates (merged; coordinator-verified file:line)

1. **`bounded-batch-buffer-pool`** — B+ / GOAT. Free-list of buffer-tuples,
   `take` via pop-or-allocate, return path clears + pushes ONLY below a cap
   (unbounded pool = monotonic growth under producer bursts).
   `core/src/banking_stage/transaction_scheduler/scheduler_common.rs:21`
   (MAX_RECYCLED_BATCHES=1024), `:95-115` (take_batch), `:288-293` (bounded
   return); `streamer/src/recvmmsg.rs:21-47` (PacketBufferPool, exact
   replenish, take-out-only-on-success), `:198`; third instance
   `streamer/src/packet.rs:24`. GOAT 4/4: benchable on the Pareto validator
   (alloc-count axis: k allocs/cycle → 0 after warmup — the
   reuse-allocation-clear measured class 4.26×/200001→2); integer-exact G1;
   mechanism-level magnitude (malloc/free round-trips + header bytes);
   deterministic.
2. **`stack-read-fast-path-length-prefixed-record`** — B−. One fixed stack
   `[MaybeUninit<u8>; 4096]` read covering header+typical payload; the
   header's `data_len` decides fit (zero-copy ref) vs heap + second read.
   `accounts-db/src/append_vec.rs:491-549`, exact-sized variant `:559`.
3. **`batch-verify-optimistic-aggregate-fallback-individual`** — B−
   CONDITIONAL (needs the dalek stand-in interpretation; BLS is not a
   workspace dep). N=1 regime skips batching; aggregate first; on failure
   parallel individual verify identifies culprits; prepared hashed message
   carried across the fallback boundary.
   `bls-sigverify/src/bls_vote_sigverify.rs:215-298` (N=1 regime L225-246,
   aggregate math L344-352, prepared-msg reuse L324-325/L353-356).
4. **`partial-reset-watermark-pool`** — C+. Fixed-size freelist (`next_empty`
   counter, not linked list); return zeroes ONLY the used prefix
   (`mapped_len`). `program-runtime/src/mem_pool.rs:34-55` (get/put),
   `:57-65` (len-prefix reset), `:141-156` (put clamps to mapped_len).
5. **`batch-local-memoization-negative-cache-tombstone`** — C+. Per-batch
   wrapper caches hits AND misses (default-insert); deleted entries are
   tombstoned (lamports==0), never removed — read-your-writes within the
   batch. `svm/src/account_loader.rs:256-296` (negative insert L268-271,
   tombstone L281-284).

KILLS (positive instances / thin, full list in the lane records):
zero-copy POD view over aligned mmap (`encoded-layout-borrowed-view`);
u32-logical/u64-file offset split (`usize-index-to-u32`); unschedulables
buffer reuse (`reuse-allocation-clear`); mem::take capacity retention
(`mem-take`); madvise(Random) one-liner (mmap-advice family); ThreadSet u64
bitmask (bool-to-bitpack family); priority-floor-before-verify
(`filter-before-expensive-transform`); const-assert wire layout
(correctness-shaped); dirty-flag-gated sync_all (thin); CPU core pinning
(ops, no fix-space); dual-representation sysvar cache (domain-specific);
`align_offset(16)` padding idiom (wouldn't suggest it); bounded retry pool
(`bounded-channel-backpressure` positive); lazy static pubkey→parser table
(`precompute-table` positive); token bucket + sharded limiter
(`mutex-to-atomic` positive); `verified_certs` dedup prune (ambient).
RIDERS: regime-split-function-pair (`streamer/src/packet.rs:104-112`, adj.
`mir-destprop-branch-site-merge`); lock-amortized-consumer-drain
(`poh/src/poh_service.rs:464-495`, the consumer mirror of
`channel-send-batching-unpark`); jemalloc collect-once note
(`program-runtime/src/serialization.rs:276-280`, regime annotation for the
existing `collect-avoidance` rule). LEAD (unmineable, in-source TODO):
subgroup-check-on-aggregate-only (`bls_vote_sigverify.rs:385-390`).

## Dedup evidence + the one CORRECTION

Greps over `src/domains/rust_perf/` under both vocabularies:
`freelist|free.list|watermark` → 0; `aggregate.verify|batch.verify|optimistic`
→ 0; `recycler|buffer.pool|batch.pool` → **NOT zero**: one context note in
`entries_05.rs:873` names the existing rule
`batch-pool-read-makes-serial-readahead-redundant` (B123 — a parallel READ
pool deleting a redundant readahead hint; IO shape, not memory recycling).
The Lane A "0 hits" evidence line was wrong in the literal sense; the NOVELTY
verdict stands (different mechanism), but candidate 1 MUST carry a
distinct-from row vs B123 at admission, alongside the already-required rows
vs `reuse-allocation-clear` / `vec-reuse-across-lifetime-change` / `mem-take`
/ `smallvec-stack`.

## Yield estimate

**2-4 rules land** after coordinator falsification (candidates 1-2 strongest;
3 conditional; 4-5 C+). Inside the honest 1-3 band per lane, composed.

## Caveats

- Unread files (re-read at mine time): `core/src/repair/*`, `core/src/consensus/*`,
  `replay_stage.rs`, `window_service.rs`, `tvu.rs`, most of `gossip/`,
  `runtime/src/*`, `accounts-db/src/accounts_index/*`, `poh_recorder.rs`
  (service half only), `ledger/src/shred/*` (ReedSolomonCache = likely
  `precompute-table` positive), `program-runtime/src/invoke_context.rs` (2646L),
  `cpi.rs` (2425L), `loaded_programs.rs` (4438L, skimmed), `svm/src/account_loader.rs`
  beyond L340, `svm-conformance/*`, `programs/zk-elgamal-proof` (unread, could
  hold crypto-perf content), `vote/src/vote_state_view.rs`.
- Lane overlap adjudicated: `bls-sigverify` treated as Lane B's (inline-crate
  clause); its consumers (votor/consensus) are Lane A's.
- riir-ai league read was 152 commits behind origin — lane-intel "none" is
  indicative, not measured against latest.
- Batch fixture authoring: ORIGINAL minimal reproductions only (license
  hygiene; Apache-2.0 noted, nothing verbatim).
