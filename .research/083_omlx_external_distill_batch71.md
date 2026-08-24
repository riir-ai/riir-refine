# Research 083 — oMLX external distill, Batch 71 (Research 082 consumed)

**Status:** RECORD — batch landed 2026-08-24 (Plan 022; corpus 370 → 372, A 88 / D 74).

- **Source:** `github.com/jundot/omlx` @ `main`, read 2026-08-24 mine time.
  The Research 082 verdict queued this batch; the 2026-08-24 idle-loop
  decision ("research online to unblock any blocked tasks") scheduled it and
  fetched the three previously-blocked sources.
- **Routing:** `kernel_opt` (host orchestration + speculative-decode state
  management + GPU thread affinity) — unchanged from the verdict.

## What landed

1. **`rotated-ring-undo-via-rebind-reference-snapshot`** (A 87→88) — from
   `patches/mlx_lm_mtp/cache_rollback.py`, re-verified verbatim at mine time:
   the module docstring's phantom-token corruption ("a rotated rotating cache
   is not trimmable ... MTP draft rejection could not discard the rejected
   positions exactly and phantom tokens progressively corrupted output (hit on
   DeepSeek-V4-Flash, sliding_window=128)"), the `_update_concat` rebind
   contract ("only rebinds `keys`/`values` (no in-place setitem), so stashing
   the pre-update attribute references plus the update's inputs gives an exact
   undo"), `trim()`'s restore + replay of the confirmed prefix, thread-local
   arming (`_UNDO_ARMED`, resolved through `sys.modules` so re-imports cannot
   go stale), the `v + 0` detach ("a plain reference would see the post-update
   value") — applied on paths that may mutate in place, with PLAIN refs kept
   on the rebind-only path (`stage_functional_rotating_update`: "the old array
   wrappers are never mutated, so retaining their references is sufficient") —
   and the chained single-step form (keep the original snapshot, concatenate
   newly staged inputs).
2. **`gpu-api-thread-affinity-snapshot-metrics`** (D 73→74) — from
   `process_memory_enforcer.py`, read in full: the verdict's B-grade promoted
   to source-verified, and candidate #3 (pool-clear-waits-for-async-completion-
   callbacks) MERGED as the justification for the flag-and-defer shape. Three
   instantiations: snapshot (`_current_usage_bytes`: "During active requests
   this must not call MLX/Metal APIs from the background enforcer thread. The
   scheduler records the last mx.get_active_memory() sample on the MLX
   executor thread; the enforcer combines that cached value with the kernel
   phys_footprint ledger"), flag-and-defer (`_request_scheduler_cache_reclaim`:
   "Routes the reclaim through the scheduler's shipped, lock-protected,
   engine-stream-synchronized `_sync_and_clear_cache` path ... instead of
   touching Metal from the enforcer thread — the enforcer must never touch
   Metal directly. Setting the per-scheduler flag is GIL-atomic; the actual
   clear fires on the inference thread at the next step boundary, even under
   load — unlike `request_idle_reclaim`, which never fires while requests are
   running"), cache-once (`_refresh_effective_metal_cap_bytes`: "Reading the
   Apple default cap falls back to mx.device_info(), so keep it out of active
   decode ticks once the enforcer is running").

## Pool dispositions

- **#4 MMA indexer (M2 Ultra, PR #2802)** — KILL as dedup: positive instance
  of B16 `cmma-hardware-unit-over-scalar-alu` + the B14/B16 tile-staging
  family. Cited here as external validation evidence only (an independent
  implementation reaching for the same hardware unit + resident tile).
- **#5 Laguna fusions (PR #2827)** — DROPPED: `optimizations.py` was unread at
  verdict time (timeout); read at mine time it is a hardware-detection stub
  ("No additional optimization is needed - mlx-lm is already fast out of the
  box") — the fusions are NOT in the file. The doc-sourced C-grade evaporates
  on source read; the fusion family was already dedup-assessed (B19/B29/B35).
  **Lesson: a doc-sourced candidate graded on secondhand evidence should be
  re-graded against the named file before any disposition survives.**

## Dedup (re-run at mine time against the 370-rule corpus)

- Rule 1: corpus "rotated" hits are orthogonal-rotation math (RoPE / Q
  rotation), `undo` hits are make-unmake (host game state) — zero
  rebind/phantom/ring-undo coverage. Distinct-by-text from B39 (non-rotated
  rewind), B58 (checkpoint SET vs undo MECHANISM), B32c, B22 (readback
  timing).
- Rule 2: corpus "thread-local" hits are GPU thread-local MEMORY (indexable
  thread-local arrays in kernels) — zero CPU-thread-affinity coverage.
  Distinct-by-text from B56 (init-time construction race), B21 (blocking-sync
  ban vs API-call ban), the readback-timing family.

## GOAT

- G1 syn-parse: `batch71_pseudocode_parses` (both fixtures parse; the rule-1
  fixture uses let-else + `thread_local!`, the rule-2 fixture atomic ops).
- G2 lib tests: kernel_heal 879/0/1i · default 669/0/0 · combined
  kernel_heal+latent_retrieval 955/0/2i · all-features 1152/0/13i.
- G3: clippy 0 both states (`--all-targets -D warnings`); `ci_feature_guard.sh`
  ALL LAYERS PASS.
- Retrieval floors HELD with zero decay: kernel self-variants 100/100/100,
  perf@1 96.2, blend@1 94.7, clippy@1 89.6 — identical to the Batch 65/70
  recorded values (the 2 new rules add no crowding on the fixture probes).
- Count pins walked: tests.rs (total/exhaustive-comment/counts-table/A/D/
  sums ×2) + router.rs (comment + assert) + corpus/mod.rs (doc + category
  comments + header line) + entries_gpu_07.rs header + AGENTS.md + `.docs/`.

## Honest caveats

- Python/MLX source; fixtures are ORIGINAL minimal Rust-pseudocode
  reproductions (license hygiene + binary-only distribution). No MLX source
  pasted.
- Rule 2 is host-orchestration decision content — no wall-clock GOAT (the B58
  speculative-family precedent); rule 1's magnitude story is correctness
  (phantom-token corruption), not cycles.
- oMLX moves fast (0.6.3rc2 same week as the verdict); quotes pinned to
  2026-08-24 `main`.

## Lessons (generator)

- **New doc_markdown variant:** stylized ALL-CAPS verbs (`KILLed`, `DROPPed`)
  in `//!` prose trip `doc_markdown` — use plain words. (The known families —
  bare `.py`/`.rs` filenames, math with underscores — did not recur; both
  were pre-empted.)
- **Worktree manifest surgery for riir-clippy:** a /tmp worktree inherits 7
  relative `../repo` sibling paths (katgpt-core, riir-engine, riir-gpu,
  riir-rag, riir-neuron-db + 3 patch entries) that resolve to nonexistent
  locations — absolutize all of them, restore before commit. Same class as
  the riir-ai worktree lesson (exFAT junctions there, path resolution here).
- The batch ran concurrently with two live sibling sessions (the 4090
  Plan-342 cluster + the M3 Issue-039 ELO lane in this repo's main checkout)
  with zero interference via the isolated worktree — the corpus files are
  disjoint from the sibling's `elo.rs`/`traj_store.rs`/`heal.rs` lane.

## Queue state after Batch 71

Next candidates unchanged from the Batch 65/70 snapshot: sec Phase 2 (owner
scheduling call), `rustc_errors` promotion (owner call), the B70-fusion reopen
triggers, or a new pool from the online-search / research channels. The
Research-076 arxiv intake channel + the `distill` skill verdict channel remain
the two external intake paths.
