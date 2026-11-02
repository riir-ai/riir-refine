# 362 — sccache Distill

**Status:** DISTILLED — pending owner decision (Plan 328 staged behind the live queue heads)
**Date:** 2026-11-02 (zed-m3-mozilla-quartet/M3, user-directed four-repo @distill — uniffi-rs + sccache + rust-code-analysis + supply-chain; this note covers sccache, its sibling note is `.research/361`)
**Queue slot:** nominal batch per queue order / Plan 328 — staged behind the live queue heads

## Source

- **Repo:** `mozilla/sccache` — distributed compilation cache (tokio server + compiler wrappers + `lru_disk_cache` subcrate + cloud backends)
- **Provenance pin:** `71d107e0ad86239bb8a134edfab82e337ffecaab` (shallow clone HEAD, 2026-11-01, actively maintained; Apache-2.0; every quote below byte-verified at this sha)
- **Shape:** ~55k LOC in `src/` (68 files); `todo!`/`unimplemented!` = **0**
- **Profile:** `codegen-units=1, lto=true, strip=true` — dist-grade release posture. Hash: **blake3** primary (sha2 only behind `dist-client`), `HASH_BUFFER_SIZE = 128 KiB`.

## Verdict

**YES B− corpus [3–4 own rust_perf rules — io/cache/process classes; ONE genuinely benchable GOAT lead (spawn-latency syscall class)] / league-lane N/A STATED (no GPU/serving surface — a build tool) / product-coverage MODERATE — the tokio family is small (5 rules standing) and sccache's candidates are more io/cache/process than tokio-core; the family's named follow-up pool (Alice Ryhl "What is Blocking?" + the official blocking docs + tokio-rs/mini-redis) stays the primary seeding path; sccache composes into it.**

Route **`rust_perf`** (io + caching + process categories). The tempting WRONG domain: `sec` — sccache's blake3 choice corroborates `weak-hash-to-blake3` but carries no NEW security class; `kernel_opt` is nowhere (no GPU code).

## Ranked candidates (coordinator-verified at the pin)

**1. A `pre_exec-disables-posix_spawn`** — `src/mock_command.rs:269–282`; the mechanism comment VERIFIED VERBATIM this session:

> "`configure` registers a `pre_exec` closure to clear `CLOEXEC` on the jobserver's file descriptors, and any `pre_exec` makes `std` fall back from `posix_spawn` to `fork`+`exec`. Almost nothing sccache spawns can use a jobserver -- preprocessors, version probes and C/C++ compilers all ignore it -- so the default is not to share, and the callers that spawn `rustc` ask for it."

The fix: gate fd-inheriting `configure()` behind per-child opt-in (`share_jobserver`); children that ignore the jobserver keep the `posix_spawn` fast path. Two-regime: the child consumes inherited fds (rustc — pay fork+exec) vs the child ignores them (preprocessors, version probes — fast path). This is sccache's OWN landed fix — the verdict mines the decision, fixtures are original.
**GOAT (passes all four):** (i) microbenchable — N × spawn+wait of a fixed child, sole delta = a no-op `pre_exec` closure, wall-clock medians, run-ALONE posture (syscall-class decision content, the corpus's syscall-rule class); (ii) mechanism magnitude — `posix_spawn` is `clone(CLONE_VM|CLONE_VFORK)`-class; fork+exec copies the PARENT's page tables and the parent here is a long-lived fat daemon; expected 2–10× per-spawn, aggregate-real at thousands of spawns/build; **caveat: magnitude scales with parent RSS — pin parent allocation in the fixture for a portable bar**; (iii) deterministic. **The batch's GOAT lead.**
Dedup: `posix_spawn|pre_exec` over `src/domains/` → **0 hits** (coordinator-run).

**2. B `reserve-then-commit-capacity-accounting`** — `src/lru_disk_cache/mod.rs:327–367` + `cache/disk.rs::put_raw`.
Two-phase insert: reserve the advertised size (pending vec + pending_size) under the lock → write the temp file lock-free → `commit` re-locks, RE-MEASURES the real size, evicts the delta if actual > advertised. Distinct from `exclusive-mkdir-temp-resource-reservation` (entries_56 — a claim on a named EXTERNAL resource; this is an in-memory CAPACITY LEDGER for in-flight writes). Regime: size known → exact reserve; `insert_by(size=None)` → no reservation possible.

**3. B `mtime-encoded-lru-recency-persistence`** — `src/lru_disk_cache/mod.rs:68–78, 186–209, 386–389`.
The law: LRU recency stored AS the filesystem — every hit touches the file's mtime, boot-time init sorts by mtime to rebuild the eviction order, zero metadata file. Distinct from `tier-hit-refreshes-lru-recency-at-owning-tier` (B308 — cross-TIER coherence) and `slru-probation-protected-promote-on-second-touch` (entries_35 — within-cache admission): THIS axis is cross-RESTART persistence. Two-regime: long-lived single process → the per-hit mtime touch is pure syscall cost (use an in-memory LRU); crash/restart- or multi-process-shared disk cache → mtime IS the recency store.

**4. B `io-copy-write-adapter-hash-loop`** — `src/util.rs:42–65` (+ `HASH_BUFFER_SIZE:40`).
Chunked hashing driven through `std::io::copy` into a `Write`-adapter closure to inherit its stack-buffer specialization: zero heap allocs, UNINITIALIZED stack buffer — no 128 KiB memset per input. Dedup: `io::copy|io_copy` → **0 corpus hits** (coordinator-run). Regime: many small inputs (headers) → zeroing dominates; few large streams → a hoisted heap buffer is equally fine. **Alloc-count axis validator-friendly (0 vs 1 heap alloc per hash call).**

**5. B− `get-with-raw-single-read-parse`** — `src/cache/cache.rs:76–90` + `cache/disk.rs:114–121`.
The trait default reads a hit TWICE (`get` then `get_raw`); raw-capable backends override `get_with_raw` → one read, parse from a `Cursor` over a refcount-cloned `Bytes`. Distinct from `serialize-once-byte-buffer-store` (owns store-once; this owns read-once). Regime: backend serves raw bytes → override; else the 2× read is accepted.

**Drops:** multilevel `backfill-on-hit` (C — write-behind promotion is architecture, not a bounded fix); boot temp-residue sweep + oversize eviction (C — hygiene); zip `Stored` + zstd blobs (C — format decision); zstd-level env read per `put_object` (DROP — positive instance of the hoist family); jobserver helper-thread→oneshot bridge (known async pattern); `close_inherited_fds` on daemonize (correctness, no regime); `kill_on_drop` (standard tokio practice, unconditional).

**Corroborations (not rules — the gate working):** blake3 primary hash (`weak-hash-to-blake3` ✓); `spawn_blocking` for parse/deps; piped stderr drain for child lifetime (`child-piped-stderr-drain-for-process-lifetime` ✓ — landed at Batch 316 from impeccable, a DIFFERENT source owning the same law: provenance note, never a re-mine).

## Dedup evidence (coordinator-run at the corpus head, both vocabularies)

- `posix_spawn|pre_exec` → 0. `io::copy|io_copy` → 0.
- `lru|evict|recency|touch` family → `tier-hit-refreshes-lru-recency-at-owning-tier` + `slru-probation-protected-promote-on-second-touch` exist; the mtime-persistence axis unowned.
- `exclusive-mkdir-temp-resource-reservation` (entries_56) exists; the capacity-ledger axis unowned.
- `serialize-once-byte-buffer-store` owns store-once; the read-once axis unowned.

## GOAT

Lead candidate #1 passes all four GOAT criteria (benchable, exact semantics via the syscall class, mechanism magnitude, deterministic) — the batch's measured arm. #4 rides the alloc-count axis (the validator's exact alloc-count counter). #2/#3/#5 are NotBenchable-class decision content (lock-hold structure, syscall-per-hit placement, read-count placement) — annotations, never demotions.

## Caveats

- **Unread/spot-checked:** the 8 cloud backends (ambient HTTP), `dist/` (~5k LOC, spot-checked), per-compiler frontends (arg-parsing + env hashing, spot-checked), `config.rs` (3,572 L, ambient), the `server.rs` conn-handling region, and `args.rs` — **residual risk: an arg-canonicalization trick may hide in `args.rs`; re-check at mine time.**
- #1's regime split is sccache's landed fix; grade A assumes the corpus lacks the mechanism (0 greps support it) — the standard corpus-confirm re-grep at authoring time is mandatory (the corpus moves).
- Shallow clone — no history archaeology.

## Bottom line

Mine as ONE rust_perf batch: 3–4 rules, lead with **`pre_exec-disables-posix_spawn`** as the GOAT-bench candidate (the only measured arm), `io-copy-write-adapter-hash-loop` as the alloc-count benchable. Plan 328 staged behind the live queue heads.

Session: zed-m3-mozilla-quartet, epoch 1793597021.
