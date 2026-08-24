# Plan 022 — Batch 71: the oMLX external kernel_opt distill (Research 082 consumed)

**Status:** Active — scheduled by the 2026-08-24 idle-loop housekeeping decision
("mine the next riir-clippy batch, researching online to unblock any blocked
tasks" — the owner instruction resolves Research 082's "batch plan opens when
the batch is scheduled" gate).

Source: `jundot/omlx` (Apache-2.0, Python/MLX serving engine for Apple
Silicon) — the Research 082 verdict's queued candidates. This is the SECOND
kernel_opt EXTERNAL batch (after Batch 70's arXiv paper distill) and the first
from a live serving-engine repo.

## Scope (fixed by Research 082 + this session's source reads)

| # | Candidate | Verdict at mine time | Disposition |
|---|---|---|---|
| 1 | `rotated-ring-undo-via-rebind-reference-snapshot` | A-grade; `cache_rollback.py` re-verified verbatim 2026-08-24 (module docstring phantom-token corruption, `_update_concat` rebind contract, `trim()` restore+replay, thread-local arming, `v + 0` detach nuance, chained single-step snapshots) | **RULE** -> category A (beside append-cache-rewind / write-before-read) |
| 2 | `gpu-api-thread-affinity-snapshot-metrics` (+#3 merged) | B->A-grade after source read; `process_memory_enforcer.py` read in full 2026-08-24: `_current_usage_bytes()` cached-sample docstring, `_request_scheduler_cache_reclaim` flag-and-defer ("the enforcer must never touch Metal directly"), `_refresh_effective_metal_cap_bytes` cache-once; #3's callback-drain race (SIGABRT / prepare-count-underflow) is the JUSTIFICATION for the flag-and-defer shape — merge confirmed by source | **RULE** -> category D (beside concurrent-context-init) |
| 3 | (merged into #2) | — | merged |
| 4 | MMA indexer kernel | KILL dedup (B16 cmma-hardware-unit) | cite as validation evidence in Research 083 only |
| 5 | Laguna fusions via `optimizations.py` | `optimizations.py` READ 2026-08-24 (previously timed out): it is a hardware-detection STUB ("No additional optimization is needed - mlx-lm is already fast out of the box") — the fusions are NOT in this file | **DROP with reason** |

Yield: 2 rules (within Research 082's 1-3 estimate). Corpus 370 -> 372 (A 88, D 74).

## Tasks

- [x] T1 Research online to unblock the blocked sources: fetch `memory_monitor.py`, `process_memory_enforcer.py`, `optimizations.py` (all read in full 2026-08-24; memory_monitor is pure-arithmetic estimation — its guard/front-door pattern feeds rule 2's context, no separate rule).
- [x] T2 Re-verify `cache_rollback.py` quotes at mine time (verbatim match on every load-bearing claim).
- [x] T3 Dedup grep both candidates against the current corpus (B70-era): rule 1 "rotated" hits are orthogonal-rotation math only, zero rebind/phantom/ring-undo coverage; rule 2 "thread-local" hits are GPU thread-local memory, zero CPU-thread-affinity coverage. Both uncovered.
- [x] T4 Author the 2 corpus entries at the tail of `entries_gpu_07.rs` (rules 371-372; file at 355 lines, under the ~950 open-file ceiling).
- [x] T5 `KERNEL_CATEGORIES`: +1 row in A (`rotated-ring-undo-via-rebind-reference-snapshot`, 87->88) + 1 row in D (`gpu-api-thread-affinity-snapshot-metrics`, 73->74); update the header batch-history line + the 370->372 doc mentions; category-count comments.
- [x] T6 Standing gates: `batch71_rules_present` + `batch71_concepts_covered` + `batch71_pseudocode_parses` (with the `#[cfg(not(feature = "kernel_verify"))]` attribute — the B70 lesson).
- [x] T7 Count pins walked (the 18+ sites): tests.rs:66/3811/3844/3847/3869/3879, router.rs:456-461 (x2 + message), entries_gpu_07.rs header, AGENTS.md (4 mentions), `.docs/11_domains/`, corpus/mod.rs:1403.
- [x] T8 GOAT: G1 syn-parse (T6 gate) + G2 lib tests both feature states + G3 clippy 0 + CI guard layers + retrieval floors HOLD (kernel self-variants 100/100/100; blended floors — re-measure, never weaken silently).
- [x] T9 Research note 083 (born-complete, Status: RECORD) + AGENTS.md Batch 71 row + `.docs/11_domains/` count + `.distill/001_mining_queue_snapshot.md` queue-state update.
- [x] T10 Commit `feat: batch 71: oMLX external distill ...` + push (isolated /tmp worktree dance — the main checkout carries the sibling's Issue 039 ELO WIP; rebase onto moved origin if needed).

## Honest caveats (from Research 082, carried)

- Python/MLX source — fixtures are ORIGINAL minimal Rust-pseudocode
  reproductions (license hygiene + the binary-only distribution rule). No MLX
  source pasted.
- oMLX moves fast (0.6.3rc2 same week as the verdict); quotes verified against
  `main` at 2026-08-24 mine time.
- The A/B/C rule-2 instantiations are host-orchestration decision content —
  no wall-clock GOAT (the B58 speculative-family precedent).
