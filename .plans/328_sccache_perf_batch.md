# Plan 328 — Batch (nominal): sccache io/cache/process → rust_perf

**Status:** STAGED — distill verdict 362 (2026-11-02); claim per the queue before any corpus edit
**Lease released:** zed-m3-sccache-b324, 1793599016 — released 2026-11-02 ~06:1xZ UNLANDED at owner direction ("fix these and bench on bmo" → riir-ai prefill issues 876 + 851, Rule 0); zero corpus edits made (claim + .raw clone + dedup only); plan 328 STAYS STAGED for the next lane

Distill verdict: `.research/362_sccache_distill.md` (YES B− corpus, mozilla/sccache @ `71d107e0ad86239bb8a134edfab82e337ffecaab`, Apache-2.0). This batch converts the io/cache/process candidates into rust_perf corpus rules, led by the ONE genuinely benchable GOAT candidate (spawn-latency syscall class) and the alloc-count-benchable hashing rule.

## Phase 0 — provenance + slot

- [ ] Re-clone at the pin: `git clone --depth 1 https://github.com/mozilla/sccache.git .raw/sccache` + `git rev-parse HEAD` must equal `71d107e0ad86239bb8a134edfab82e337ffecaab` (moved HEAD = re-pin + re-verify every quote)
- [ ] Claim the batch in `.distill/001_mining_queue_snapshot.md` (claim row + 48 h expiry); re-verify no live foreign claim owns the rust_perf slot
- [ ] `py ../katgpt-rs/scripts/dual_allocation_gate.py` at the landing (0/0/0 expected)

## Phase 1 — dedup re-verification at the slot

- [ ] Re-run both-vocabulary greps per candidate AT THE LANDING HEAD (362's zero-hit evidence is dated; the corpus moves): `posix_spawn|pre_exec`, `io::copy`, `lru|recency|mtime|touch`, `reserve|pending_size|capacity`, `get_with_raw|get_raw`
- [ ] The three DISTINCT adjudications are load-bearing: #2 vs `exclusive-mkdir-temp-resource-reservation` (external-resource claim vs in-memory capacity ledger); #3 vs `tier-hit-refreshes-lru-recency-at-owning-tier` (cross-tier coherence) + `slru-probation` (within-cache admission) — mtime axis = cross-RESTART persistence; #5 vs `serialize-once-byte-buffer-store` (store-once vs read-once)
- [ ] The corroboration row is a PROVENANCE note, never a re-mine: sccache's piped-stderr-drain + spawn_blocking + blake3 postures corroborate laws landed from OTHER sources (child-piped-stderr-drain-for-process-lifetime @ Batch 316/impeccable)
- [ ] Read the residual-risk surface before authoring anything arg-shaped: `args.rs` (the verdict's named unread spot-check risk)

## Phase 2 — authoring (target 3–4 rust_perf rules)

- [ ] `pre_exec-disables-posix_spawn` (A — the GOAT lead, io/process): fixture = N × spawn+wait of a fixed child both arms, sole delta = a no-op `pre_exec` closure; wall-clock medians, RUN-ALONE posture (syscall-class bench; box-state provisos); **pin parent allocation in the fixture — the fork+exec cost scales with parent RSS, and an unpinned parent makes the bar non-portable**; pre-register the bar BEFORE the first run; carry the two-regime content verbatim-shaped (child consumes inherited fds → fork+exec; child ignores → posix_spawn fast path); the syscall-fallback mechanism is Linux/macOS `std::process` behavior — note the Windows posture (no posix_spawn fast path there; different baseline)
- [ ] `io-copy-write-adapter-hash-loop` (B — alloc-count benchable, io): fixture = chunked hashing via an accumulating heap buffer (1 alloc + 128 KiB memset per call) vs `io::copy` into a Write-adapter closure (0 heap allocs, uninit stack buffer); the alloc-count axis is the discriminator; regime: many small inputs vs few large streams
- [ ] `reserve-then-commit-capacity-accounting` (B, caching): fixture = single-phase insert under a full lock (write-under-lock or unaccounted overshoot) vs reserve-advertised/commit-re-measure/evict-delta; NotBenchable (lock-hold structure class); regime split on size-known vs size=None
- [ ] `mtime-encoded-lru-recency-persistence` (B, caching): fixture = in-memory LRU lost on restart vs mtime-as-recency-store rebuilt by sort; NOTBENCHABLE annotation + the honest synthetic-bench warning (an A/B of eviction ORDER needs a workload, the fixture-artificiality drop class — decision content only); regime: single-process long-lived (anti) vs restart/multi-process-shared disk cache (pro)
- [ ] `get-with-raw-single-read-parse` (B−): fixture = trait-default double read vs raw-override single read + Cursor parse over refcount-cloned bytes; suggestion-grade
- [ ] Every entry: category per the rust_perf taxonomy (io/caching/process), kinship cites (exclusive-mkdir, tier-hit-refreshes, slru-probation, serialize-once, child-piped-stderr-drain), fixtures ORIGINAL minimal reproductions — never paste sccache source verbatim

## Phase 3 — gates (isolated CARGO_TARGET_DIR)

- [ ] Batch needles + parse gate test
- [ ] `cargo test --lib` default posture (count floor = current + landed)
- [ ] `cargo test --lib --features rust_perf_bench` posture — the A-rule's spawn bench runs here (run-ALONE; box-state preflight per the bench_preflight law; syscall-class wall-clock medians)
- [ ] `cargo clippy --all-targets -- -D warnings` × postures (default / rust_perf_bench / no-default / all-features)
- [ ] Retrieval floors both postures
- [ ] Freeze/score-bench per the P30 rule at landing time (rust_perf IS in the release set — the B255/B308 lesson)
- [ ] Maintained binary REBUILD + install at the landing HEAD; `cargo refine --version` verifies the build stamp

## Phase 4 — count pins + docs

- [ ] Corpus len witness sites re-pinned (tests.rs asserts + era witnesses + mod.rs slice headers if io/caching slices move)
- [ ] AGENTS.md rust_perf Role bullet + README + `.docs/11_domains/rust_perf.md` batch line
- [ ] `.research/362` status line → DISTILLED — batch landed (or the honest partial with the falsification record)
- [ ] Queue snapshot landing row + close the claim; remove `.raw/sccache`

## Phase 5 — commit + push

- [ ] One commit: corpus + tests + pins + docs + note status + queue row (`feat:` prefix; Session line in the body)
- [ ] Push develop
