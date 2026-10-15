# Plan 241: Batch 239 — the P65+P66 serving state-cache pair (llama.cpp #30514 + #30498)

**Status:** OPEN — claimed 2026-10-15 ~16:xx +07 (shikuwa/4090 lane); staged by the ~16:xx envelope-grade cycle (commit `af914ee7`).

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

- [ ] T0 — claim row + this plan, commit + push BEFORE authoring (this
      commit; claim expiry 48h).
- [ ] T1 — quote re-verification at the pins: fetch the `.patch` files for
      #30514 + #30498; verify EVERY staged quote verbatim against the diff
      (the compare-API payloads were read at staging; the patch files are the
      record of authority). Kill/adjust any drifted claim BEFORE authoring.
- [ ] T2 — corpus placement: read `src/domains/rust_perf/corpus/entries_20.rs`
      + `entries_26.rs` (the B219 pair's home) — extend the right file or open
      the next free `entries_NN.rs`; author both rules (pattern, bounded
      fix-space, category caching, rationale, source quotes, NotBenchable
      annotation, kinship citations with the distinctness reasons).
- [ ] T3 — count pins: rust_perf 198 → 200 at every pin site (AGENTS.md
      §Role + §Feature Flag Discipline's rust_perf bullet; corpus mod.rs
      count constants; any per-category counters — caching moves +2).
- [ ] T4 — standing gates: extend/add `batch239_pseudocode_parses` (the
      syn-parse gate over both new rules' opt_pattern bodies) + the needles
      test (each rule's needle strings present verbatim).
- [ ] T5 — GOAT: G1 lib tests green (`cargo test --lib` at default +
      `--features rust_perf`); G2 clippy 0 new in BOTH postures
      (`--all-targets` default + rust_perf feature set; use
      `CARGO_TARGET_DIR=/tmp/b239tgt` if the main target dir is locked);
      G3 retrieval re-measure — blended floors HOLD
      (`cargo test --features kernel_opt,latent_retrieval --lib retrieval`)
      — re-pin + record ONLY if a measured decay is explained (never weaken
      silently).
- [ ] T6 — Research 270 note (born-complete RECORD: source, method, quotes,
      the two rules, the watch-only #30522 disposition, the mlxfast
      wrong-base artifact recorded for the next poller).
- [ ] T7 — AGENTS.md lineage (§Role rust_perf count/lineage + the feature
      bullet), `.distill/001` landing row + claim tick, commit
      `feat: batch 239 ...` + push.

## Out of scope

- #30522 (watch-only; re-read if the class reappears).
- Any kernel_opt movement (corpus stands 813).
- The B/C disk sets (owner-deferred: pplx, target cargo-cleans).
