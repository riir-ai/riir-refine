# Research 082 — oMLX (jundot/omlx) distill verdict: MTP rollback + Metal host-orchestration patterns

**Status:** DISTILLED — pending owner decision (queued behind the sec Phase 2 /
`rustc_errors` promotion owner calls; batch plan opens when the batch is scheduled).

- **Sources:** `github.com/jundot/omlx` (Apache-2.0, Python/MLX serving engine for
  Apple Silicon — continuous batching, SSD prefix cache, ANE prefill, native MTP,
  distributed serving) + `github.com/Weschera/Qwen3.8-27B-oMLX-MTP-Mac` (measured
  recipe repo, same-day A/B JSON). Strategic findings for the engine campaigns
  (Issues 717/726/742) recorded separately in
  `../riir-ai/.research/343_omlx_ane_mtp_metal_state_of_art.md`.
- **Inspected:** 2026-08-22. Releases page (0.5.8→0.6.3rc2) + repo structure +
  `omlx/patches/mlx_lm_mtp/cache_rollback.py` (read in full) + zread architecture
  docs (memory enforcer / scheduler / SSD tier / engine pool).
- **Routing:** `kernel_opt` (host orchestration + speculative-decode state
  management + GPU thread-affinity). NOT `rust_perf` — every candidate is
  GPU-adjacent. The tempting wrong target would be mining the Python scheduler
  logic as generic concurrency — it is GPU-API-bound orchestration.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `rotated-ring-undo-via-rebind-reference-snapshot` — a ROTATED ring cache (sliding-window KV) cannot trim: the evicted token's slot is overwritten, so a rejected speculative verify write (S==2) leaves the rejected token as a PHANTOM that progressively corrupts output (live bug: DeepSeek-V4-Flash, sliding_window=128). The undo is enabled by the update path's CONTRACT: the S==2 verify always takes the concat path which only REBINDS the `keys`/`values` attributes (no in-place setitem), so stashing pre-update attribute refs (detached: `v + 0` — a plain ref would alias the post-update lazy graph, the handle-clone-not-ref discipline) + the update's inputs gives an exact undo; restore refs, then REPLAY the confirmed prefix as a normal decode-sized update. Armed ONLY around the speculative forward (thread-local scoping) so non-spec flows keep stock semantics. Two-regime: S==1 takes the in-place ring write → reference snapshots invalidated → NOT undoable (documented boundary). | `patches/mlx_lm_mtp/cache_rollback.py` (read in full) | **A** | Maps directly onto our Issue 717/742 `checkpoint/rollback_speculative_gpu` + Issue 715 rotating-cache work. Distinct from B39 `append-cache-rewind-overwrite-in-place` (that is the NON-rotated append-only regime — pointer rewind; the rotated regime is structurally un-rewindable), from B58 `write-before-read-state-skips-checkpoint` (decides the checkpoint SET; this decides the undo MECHANISM when trim is impossible), from B32c `make-unmake-over-clone-per-candidate` (host game-state delta log). Dedup greps for rollback/phantom/undo/rebind: all hits are the three rules above — the rotated-ring + rebind-contract combination is uncovered. |
| 2 | `gpu-api-thread-affinity-snapshot-metrics` — never call GPU/Metal APIs from a background thread during active request processing (it corrupts command-buffer internal state); the background enforcer reads a CACHED value recorded by the scheduler on the GPU execution thread. Same discipline in the SSD writer: two-phase extraction (GPU thread materializes lazy graph to raw bytes at the `mx.eval` boundary; background thread does pure file I/O — "inference thread never blocks on syscalls, writer thread never touches Metal API"), and the engine pool serializes ALL cache-clear/compile operations onto one global executor because MLX has a single global Metal command queue. | zread docs (memory enforcer, SSD tier, engine pool); `omlx/memory_monitor.py` + `omlx/process_memory_enforcer.py` UNREAD — verify at mine time | **B** (pending source read) | Adjacent to B56 `concurrent-context-init-needs-process-wide-lock` (init-time) and B21 `no-blocking-sync-in-async-pipeline` (no blocking sync) — neither covers steady-state API thread-affinity. Dedup grep for metal/thread/clear_cache/callback-drain: zero covering hits. |
| 3 | `buffer-pool-clear-waits-for-async-completion-callbacks` — clearing the MLX buffer cache immediately after request completion races Metal's async `completeMemory()` IOKit callbacks → SIGABRT (M3) / 'prepare count underflow' kernel panic (M4); reclamation must serialize behind a lock AND wait for the callback drain. | zread docs (scheduler, issue #1106); source UNREAD | **B−** (pending source read) | Adjacent to our Issue 714 device-lost cascade family (graceful-OOM + sweep). May merge into #2 as one thread-affinity rule — decide at mine time after reading the source. |
| 4 | M2 Ultra MMA indexer score kernel (`simdgroup_matrix`, K-tile resident in threadgroup memory, 1.37-1.38× bit-exact, PR #2802) | release notes; kernel file UNLOCATED | **KILL — dedup** | Positive instance of B16 `cmma-hardware-unit-over-scalar-alu` + the B14/B16 tile-staging family. External validation of existing rules, not a new rule. Cite as validation evidence in the note only. |
| 5 | Laguna XS 2.1 bit-exact compiled decode fusions (router renorm + expert combination + softplus gating + SiLU gate/up, +3.96% decode M4 Max, PR #2827) | `omlx/optimizations.py` — read TIMED OUT, unverified | **C / likely dedup** | The fusion-placement family is heavily covered (B19 fuse-residual, B29 CODA epilogue, B35 kernelize-past-batching-floor). Re-read at mine time; expect a drop unless the bit-exactness discipline (token-identical greedy trajectory through compiled fusion) adds a distinct decision rule. |

## GOAT candidate

`rotated-ring-undo-via-rebind-reference-snapshot` — benchable as a standard
kernel_opt corpus entry (fixtures + concept gates + count pins + retrieval
floors; the B58 speculative family precedent — host-orchestration rules do not
carry wall-clock GOAT). Semantics: the rule is about the undo CONTRACT, fixture
shapes are host-side pseudocode — no float-tolerance question. Magnitude story:
correctness (phantom-token corruption), not cycles. Deterministic.

## Yield estimate

1-3 rules after coordinator falsification: the A-grade rollback rule is
fully source-verified; #2/#3 need source reads (may merge); #4 dead; #5 likely
dedup. Strategic value (recorded in the riir-ai note) exceeds the corpus value.

## Honest caveats

- Python/MLX source, not Rust — fixtures must be ORIGINAL minimal reproductions
  in the corpus's Rust pseudocode shape (license hygiene + binary-only
  distribution rule). Never paste MLX source.
- `optimizations.py`, `memory_monitor.py`, `process_memory_enforcer.py`,
  `speculative/batch_generator.py`, the MMA kernel file: UNREAD (timeouts /
  unlocated). Anything graded above on their evidence is doc-sourced — the
  zread docs carry line references but are secondhand.
- The ANE prefill kernel ships as a PREBUILT binary (drowzeys' DualANE recipe,
  private Apple runtime interfaces) — NOT mineable; the pattern's strategic
  value is recorded in the riir-ai note instead.
- oMLX moves fast (0.6.3rc2 same week); snapshot/quote at mine time.

## Queue slot

Queued as the next `kernel_opt` external batch (after the sec Phase 2 /
`rustc_errors` promotion owner calls resolve). Batch plan (`.plans/NNN`) opens
when scheduled — plan-FIRST, no corpus edit before it.
