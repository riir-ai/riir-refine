# Plan 241: Batch 239 — the P65+P66 serving state-cache pair (llama.cpp #30514 + #30498)

**Status:** COMPLETE — all tasks landed and gated 2026-10-15 (Batch 239 landed; corpus 198 → 200; gates G1 121/121 + G2 2668/0 + G3 clippy exit 0 + retrieval 21/21 HELD; see Research 270 + the snapshot landing row).

Source: llama.cpp master delta `843e7dcc → e467e5b5e5ba` (3 commits, all
ServeurpersoCom 2026-10-15 04:30–04:39Z, MIT). #30522 (`1ddc2625`, the
never-empty-graph guard) is WATCH-ONLY — NOT in this batch (no corpus host,
llama.cpp-graph-API-specific mechanism).

## Domain routing (decided at staging)

BOTH rules → **rust_perf** (the B219 TensorFold serving-pack precedent —
serving state caches, not GPU kernels). Both **NotBenchable annotated**
(serving infrastructure; no microbench axis). kernel_opt corpus STANDS 813;
clippy corpus untouched → **freeze-table + score-bench NOT owed (P30)**.

## The two rules

### P65 — `partial-state-serialization-honors-rollback-contract-at-leaf-cache` (A−)

Source: #30514 @ `e467e5b5e5ba` (merge tip). Category: caching (D-family
serving). The law, three faces:

1. **Partial state = what rollback CANNOT undo.** A plain (non-SWA)
   `llama_kv_cache` rolls back with `seq_rm` → holds NO partial state → a
   `PARTIAL_ONLY` state_write/state_read must write/read NOTHING. The defect:
   `GGML_UNUSED(flags)` — the flag was literally marked unused — every cell
   serialized; a draft context (MTP head, dense `-md` draft) copied its whole
   KV into each server checkpoint, "growing it by the draft KV size per token".
2. **The flag is applied by EACH cache, keyed on its own `swa_type`; wrappers
   (hybrid / hybrid-idx / dsa-iswa / iswa) DELEGATE, never gate.** Every
   wrapper-level gate was wrong in a different direction: the plain cache
   over-serialized; the wrappers over-dropped — skipping attention caches on
   PARTIAL_ONLY "also dropped an SWA window that is partial state". The
   wrappers' wrong gates were REMOVED (pure delegation), the leaf gained the
   SWA-keyed refusal.
3. **The test invariant IS the gate** (case_state_partial_size): "the partial
   state is not larger than the full state and, without SWA, does not grow
   with the tokens" — partial ≤ full always; equality with tokens when no SWA.

Riders carried in-rule: the hybrid-idx indexer staleness condition
(`mem_idx_stale_set` only `if (!sinfos_attn.empty())` — "a restore that
rewrites cells does it behind the pool layout's back"); the iSWA wrapper
"no longer gates its base cache since it filters itself".

Kin (name in-rule, coin-flip-free): B219 `prompt-cache-canonical-interval-keys`
+ `prompt-cache-rewind-states-min-gap-eviction` (the serving state-cache
family — keys/eviction own their faces, THIS owns the partial-serialization
contract); B236 `mid-prefill-recurrent-state-capture-boundary-unsplit`
(kernel_opt, the what-state-at-a-boundary law — thematic cross-ref, different
domain); the B234 `#292` API-contract class (a documented flag contract the
implementation ignored).

### P66 — `cache-write-admission-requires-restoring-consumer` (B+)

Source: #30498 @ `2639a3b4`. Category: caching. The law, two faces:

1. **Admission by reusability**: a state earns a cache entry only if some
   future task restores it. `is_stateless()` (embedding + rerank: "always
   process their whole prompt, their state is never reused") → cleared on
   release like child slots; BOTH save paths share the rule (the idle-slot
   loop + the release path); LRU restores only for reusing tasks. The
   defect being closed: rerank/embedding/decision/infill states "fill the
   cache up to its limit without ever being read back, and slots already
   cached are offered again at every task start". The mid-PR `cache_pending`
   flag was REPLACED by the final `is_stateless()` + restore-any-reusing-task
   shape — the simpler law won; record the FINAL form.
2. **Workload-keyed default-off**: "Decision models disable the prompt cache
   unless --cache-ram is given, since their states seldom repeat" —
   `cache_ram_explicit` set only by the flag; hit-rate≈0 workloads default
   OFF with explicit opt-in. The default-change disclosure lives in the arg
   help text ("0 for decision models").

Kin: the B219 pair — keys (canonical-interval) / eviction (min-gap rewind) /
THIS (admission) = the three faces of the serving prompt-cache family.

## Tasks

- [x] T0 — claim row + this plan, commit + push BEFORE authoring (claim
      `d983e23a`, pushed first-try; expiry moot — landed same day).
- [x] T1 — quote re-verification at the pins: BOTH `.patch` files fetched;
      every staged quote verified verbatim — ZERO drift (two definitional
      quotes ADDED at conversion: the test header 'the partial state holds
      what seq_rm cannot roll back' + the hybrid-idx delegation
      justification).
- [x] T2 — corpus placement: `entries_27.rs` (NOT 25 — the M3's Batch 237
      slice A owns it per the `0c71bea2` note; the mod.rs comment documents
      the 25/26 gap); both rules authored with kinship citations + NOVEL
      greps + NotBenchable annotations.
- [x] T3 — count pins: rust_perf 198 → 200; annotated 144 → 146;
      unannotated 54 UNCHANGED; caching 8 → 10 (mod.rs ×2 sites + tests.rs
      ×5 assert sites + AGENTS.md ×2 sites).
- [x] T4 — gates: `batch239_rules_present` (22 needles) +
      `batch239_pseudocode_parses` (syn × buggy+fixed × 2 rules).
- [x] T5 — GOAT: G1 corpus 121/121 + lib 2668/0; G2 clippy `-D warnings`
      exit 0 (8 doc_markdown backticks in the fresh lines fixed en-route;
      the residual 'generated 1 warning' lines = the box's hard-link env
      noise); G3 retrieval 21/21 PASS 246.73s — blended 90.6/94.7/95.3 vs
      floors 89.4/93.6/93.6 ALL HELD with margin (top-1 −0.6pp vs the B238
      citation = the +2-fixture pool-competition class, B200 precedent;
      no re-pin, no Issue-063 repair owed).
- [x] T6 — Research 270 (born-complete RECORD incl. the watch-only #30522
      disposition + the mlxfast wrong-base artifact).
- [x] T7 — AGENTS.md lineage (§Role + §Feature), snapshot landing row,
      commit + push.

## Out of scope

- #30522 (watch-only; re-read if the class reappears).
- Any kernel_opt movement (corpus stands 813).
- The B/C disk sets (owner-deferred: pplx, target cargo-cleans).
