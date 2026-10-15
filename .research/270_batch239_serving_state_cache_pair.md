# Research 270 — the llama.cpp serving state-cache pair (Batch 239 conversion)

**Status:** RECORD (Batch 239 landed 2026-10-15; rust_perf corpus 198 → 200, Plan 241).

## Source + method

- Source: ggml-org/llama.cpp master delta `843e7dcc → e467e5b5e5ba` (3 commits,
  all by ServeurpersoCom 2026-10-15 04:30–04:39Z, MIT). Two commits converted;
  the third (#30522) watch-only.
- Method: plan-first (Plan 241, claim `d983e23a` pushed before authoring);
  both `.patch` files fetched and EVERY quote verified verbatim AT CONVERSION
  (the staging row `af914ee7` carried compare-API reads; the patches are the
  record of authority — zero drift found).
- Staged grades: P65 A− → landed as own rule; P66 B+ → landed as own rule.

## The delta (3 commits, graded)

1. **#30514 `kv-cache: honor PARTIAL_ONLY in the plain KV cache` @
   `e467e5b5e5ba`** — RULE-GRADE A−, converted as
   `partial-state-serialization-honors-rollback-contract-at-leaf-cache`
   (caching). The law: partial state is DEFINED as what rollback cannot undo
   ("the partial state holds what seq_rm cannot roll back: recurrent states
   and SWA windows" — the test's own header); a plain non-SWA cache rolls
   back fully so holds NO partial state and must write/read nothing under the
   flag; the flag is applied AT THE LEAF (keyed on the cache's own
   `swa_type`), wrappers (hybrid / hybrid-idx / dsa-iswa / iswa) DELEGATE and
   never gate — every wrapper gate was wrong in a different direction (the
   plain cache over-serialized with `GGML_UNUSED(flags)` literally marking
   the flag unused, growing each server checkpoint "by the draft KV size per
   token"; the wrappers over-dropped, skipping attention caches on
   PARTIAL_ONLY "also dropped an SWA window that is partial state"). The gate
   invariant: "the partial state is not larger than the full state and,
   without SWA, does not grow with the tokens". Riders carried in-rule: the
   hybrid-idx indexer stale-mark narrowed to `if (!sinfos_attn.empty())` ("a
   restore that rewrites cells does it behind the pool layout's back"); the
   failed-restore clear became unconditional ("a failed restore leaves the
   sequence empty in both caches"); the mid-PR arc — the per-slot
   `cache_pending` flag REPLACED by the final is_stateless shape — recorded
   on P66.
2. **#30498 `server: save only restorable slot states to the prompt cache` @
   `2639a3b4`** — RULE-GRADE B+, converted as
   `cache-write-admission-requires-restoring-consumer` (caching). The law:
   cache-write ADMISSION requires a restoring consumer — `is_stateless()`
   (embedding + rerank: "always process their whole prompt, their state is
   never reused") never writes, both save paths share the ONE predicate home
   (the pre-existing inline duplicate folded into it), LRU restores only for
   reusing tasks; PLUS the workload-keyed default-off: "Decision models
   disable the prompt cache unless --cache-ram is given, since their states
   seldom repeat" (`cache_ram_explicit` + the operator log line). Kinship:
   the B219 pair owns KEYS + EVICTION; this owns ADMISSION — the three faces
   of the serving prompt-cache family (entries_21).
3. **#30522 `graph: keep the graph non empty without embeddings` @
   `1ddc2625`** — WATCH-ONLY, NOT converted. An embedding-only graph with no
   logits ends up empty and "its inputs are never allocated"; the fix expands
   `t_embd` (`ggml_build_forward_expand`). The law generalizes (a graph
   producing no outputs never allocates its inputs) but the mechanism is
   llama.cpp-graph-API-specific; NO corpus host found (grep over both
   corpora: zero never-empty-graph vocabulary); +4 lines. Re-read if the
   class reappears in a future delta.

## Gates

- **G1 corpus**: 121/121 (`batch239_rules_present` — 22 needles across both
  rules — + `batch239_pseudocode_parses`, syn on buggy AND fixed × 2 rules).
- **G1 count pins**: entries 200 (was 198); annotated 146 (was 144);
  unannotated 54 UNCHANGED (both new rules annotated); caching 8 → 10. Pin
  sites: mod.rs header + Returns doc, tests.rs count assert + the 4
  annotation-count asserts.
- **G2 lib**: 2668/0 (`cargo test --lib`, all domains).
- **G3 clippy**: `cargo clippy --all-targets -- -D warnings` exit 0 — EIGHT
  `doc_markdown` backtick misses in the fresh doc lines (swa_type,
  is_stateless, NotBenchable ×2, entries_25/26 ×3, ServeurpersoCom) fixed
  en-route by hand (own-fresh-lines class, same outcome as the healer); the
  residual "generated 1 warning" lines are the box's standing
  incremental-cache hard-link environment noise, one per crate, not lint
  findings.
- **Retrieval** (`--features kernel_opt,latent_retrieval --lib retrieval`):
  21/21 PASS in 246.73s; blended 171 labeled, measured **90.6 / 94.7 /
  95.3** vs floors 126/141=89.4 / 132/141=93.6 / 132/141=93.6 — **ALL HELD
  with margin** (top-1 margin ≈ 2 spans). Honest delta: top-1 reads −0.6pp
  vs the B238 citation (91.2) — the +2-fixture pool-competition class (the
  B200 precedent: one new serving fixture outranks an existing span inside
  the pool; cross-pool competition from legitimate corpus growth, not a
  domain regression — columns 2–3 byte-identical). NO re-pin; the Issue-063
  zero-margin repair is NOT owed (measured sits above floor).
- **Freeze-table + score-bench**: NOT owed (P30 — zero clippy-corpus delta;
  this batch is rust_perf-only).

## Coordination record

- The M3's Batch 237 is in flight (plan 239): slice B (entries_26/P60) file
  landed `0c71bea2` UNREGISTERED (their batch landing wires it); slice A
  (entries_25, "the llama.cpp pair") reserved per that commit's own note;
  slices A/C pending. THIS batch therefore took **entries_27** and wired ONLY
  its own file — the mod.rs comment documents the 25/26 gap. The count reads
  198 + 2 = 200 at this landing; their landing carries it to 203.
- The staging-cycle mlxfast wrong-base artifact (recorded `af914ee7`): a
  first poll read `0c61273..origin/main` = 25 commits against the B193-era
  base and nearly staged an already-consumed wave — `0c7c392` (the B195 pin)
  IS the tip; the current-pin-first rule is named for the next poller.

## Provenance

- Patches: `https://github.com/ggml-org/llama.cpp/commit/e467e5b5e5ba43071d6151b3d881deb9300d000e.patch`
  + `.../2639a3b4e029d1be80ca4a472d62dfea23609075.patch` (fetched this
  session; every corpus quote verbatim against them).
- Corpus: `src/domains/rust_perf/corpus/entries_27.rs` (both rules, both
  `NotBenchable` annotated, `bench_spec: None` — the B219 serving-pack
  precedent).
