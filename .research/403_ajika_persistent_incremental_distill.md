# Research 403 — rust-glancer/ajika persistent-incrementality distill

**Status:** DISTILLED — batch-359-landed (2026-11-13, zed-amo-b359-ajika/amo; Plan 370, the landing commit is the arbiter). All five candidates landed (5/5, the top of the 3–5 estimate): R1 `named-outputs-lazy-handles-partial-decode` BENCHABLE ParetoDominant 3/0/0 two-run stable (partial-read posture [8,64,256]: 7.18-7.48x/49.65-55.10x/158.11-161.96x, allocs 16->2/128->2/512->2, bytes 8x-256x down — the pre-registered bars G1 bit-identical / G2 >=2-of-3 ParetoDominant + >=3x bytes-read + NO read-all regression / G3 no-regression / G4 alloc census ALL PASS; the read-all anti-regime posture measured Tie/Tie/Tie both runs — the framing tax honest as pre-registered); R2/R3 landed B− as staged (the in-rule adjudications carried: execution-artifact-trust-ladder / resume-revalidates-provenance / entries_19 for R2, write-verify-readback / entries_05 for R3); R4 DROP-OR-KEEP resolved KEEP (zero owners of checkpoint-ordering; the intern* families own adjacent axes — adjudicated in-rule); R5 DROP-OR-FACE resolved LAND (zero owners of scope-release; eager-pointer owns fetch-time not boundary-release; the GPU transient vocabulary collision adjudicated in-rule). T0 quotes re-verified byte-exact at the pin (70c1e26d, tip unmoved — the verdict pin IS the batch pin; the research's `[`Output`]` doc-link brackets were dropped in transcription, noted, not drift). rust_perf 327 -> 332 (entries_75; benchable 133 -> 134, notbenchable 194 -> 198, categories 15 unchanged).

- **Date:** 2026-11-13 (zed-amo-ajika-distill, amo)
- **Source:** blog <https://rust-glancer.github.io/blog/persistent-incrementality-in-rust/> (2026-11-06, handwritten per the author) + repo `rust-glancer/ajika`
- **Pin:** `70c1e26df847d3da10d2a59a5a6aefe37db2bf55` (tip 2026-11-12, release 0.1.7), **MIT OR Apache-2.0**, on crates.io (`ajika`); shallow clone `.raw/ajika`, deleted at close of the verdict session
- **Provenance chain:** Ajika is original work (not a port); its DESIGN is the red-green/memo model salsa popularized — cite salsa-rs/salsa + the rust-analyzer "Durable Incrementality" blog (2023-07-24) as the upstream idea lineage when mining. The author discloses LLM-assisted development; the **Ajika Book is LLM-generated** — quote code + the handwritten blog only, never the book.
- **Live counts re-derived at verdict time:** rust_perf **327**, kernel_opt **1086** (`grep -c rule_id` over the corpus dirs; AGENTS prose counts are claims).
- **Staged-pool conflict check:** `.distill/001` grep `ajika|glancer|salsa|incremental computation|incrementality` = **zero** — no re-staging conflicts with the 11-13 sweep pools.

## Verdict

**YES B− corpus (nominal Batch 359, Plan 370) / lane-intel NONE (with one home-corroboration note) / product-coverage YES — seeds the corpus's FIRST persistent-incremental-computation family (salsa-shaped memoization: 0 owners today).**

## Domain routing

`rust_perf`, categories **io / allocation / caching**. The tempting wrong domain is NOT kernel_opt (zero GPU surface) and NOT sec (the cache-reset law is recomputable-state robustness, not an attacker-facing input-validation class; its threat model is bit-rot/version skew, and the sec corpus's `execution-artifact-trust-ladder-before-deserialize` owns the OPPOSITE polarity — execution-capable artifacts must be verified before deserialize, recomputable caches must be discarded). Routing rule of thumb satisfied: storage/serialization/hot-path Rust → rust_perf.

## Source inspection (what was read)

- `crates/ajika-storage/src/lib.rs` (module doc = the throw-away law), `pack/mod.rs` (append-seal-publish + `TARGET_BYTES = 64 MiB`), directory listing of `database/format/maintenance/statistics/transaction/value`
- `crates/ajika/src/interner/mod.rs` (266 lines, head + doc), `model/mod.rs` (Stamp + retention docs), `runtime/output.rs` (handles), `runtime/cache.rs` (Memo/State docs)
- Workspace layout: `ajika` + `ajika-macros` + `ajika-storage`; `benches/runtime/{graph,inputs,outputs,persistence,sessions}.rs` exist (completeness signal positive); **no `[profile]` section in the workspace root** (untuned release profile — honest caveat)
- Blog read live over the wire (full text, quotes below verbatim)
- **Unread (re-read at mine time):** runtime `{evaluate,record,frame,coordination,parallel,registry,diagnostics,value}.rs` bodies, backend `{encoding,session,mod}.rs`, storage `{database,transaction,format,maintenance,statistics,error}.rs`, both macro crates, tests/, bench bodies

## Ranked candidates

| # | pattern | source | grade | notes |
|---|---|---|---|---|
| R1 | `named-outputs-lazy-handles-partial-decode` — a persisted computation that produces several answers together emits NAMED, independently readable outputs and returns cheap handles; consumers decode one field on demand; the unread fields never deserialize | `runtime/output.rs` doc + blog §Outputs | **B+ LEAD (benchable GOAT candidate)** | in-source: "Some work naturally produces several answers together, but consumers do not need all of them. A query can emit named outputs and return a set of `Output` handles." Blog: "Handle is cheap to obtain and deserialize, and you only query the data you actually need." Regime: big persisted outputs, partial reads. Anti-regime: every consumer reads every field (framing overhead is pure loss); tiny outputs (framing ≈ payload). UPSTREAM: present at the pin (their own design — decision content, never a PR). |
| R2 | `derived-cache-throwaway-version-reset-recovery` — persisted derived state (a cache, not source of truth) resets on schema/version mismatch and recovers-over-hard-fail on corruption; the app never crashes on cache problems | `ajika-storage/src/lib.rs` module doc + blog §Enter persistency | B− | in-source verbatim: "The directory is a disposable cache, exclusively owned until `Database` drops. Opening can discard corrupt cache data. After an uncertain commit, call `Database::reopen` on the same owner to recover the published checkpoint." Blog: "the cache must be _throw-away_. Both Ajika and application supply schema versions ... if the stored cache doesn't match it, we reset it." ADJACENT-DISTINCT (in-rule adjudication mandatory): sec `execution-artifact-trust-ladder-before-deserialize` (execution-capable vs recomputable — opposite polarity); rust_perf `resume-revalidates-provenance-before-reuse` (B121 refuses-and-stops; R2 discards-and-continues on recomputable state); entries_19 cost-table repair (noise repair of a measured table, not version/corruption reset). POSITIVE INSTANCE AT HOME: refine's own `index_cache` refuses-loud-and-rebuilds on corrupt/stale — the same discard-and-recompute posture (record, don't double-land). |
| R3 | `pack-store-gc-decides-by-recovered-index` — a multi-blob persisted store writes values to append-only pack files sealed BEFORE their locations publish; failed preparation leaves harmless orphans; after a commit attempt ONLY the recovered index decides which files are safe to delete | `ajika-storage/src/pack/mod.rs` doc + blog §Ajika storage | B− | in-source verbatim: "Append values to private files, then seal them before publishing their locations. A unique file name is safe to create before the checkpoint commits: readers only use names in the index. Failed preparation leaves harmless orphans. Once commit has been attempted, only the recovered index can decide which files are safe to delete." Plus `TARGET_BYTES = 64 * 1024 * 1024` pack rollover and the redb-index/pointer design. Distinct from `write-verify-readback-before-publish` (single-artifact publish gate) and entries_05 artifact assembly (one artifact's verify+rename; this is a STORE's crash-safe GC over many blobs). io category. |
| R4 | `persisted-interner-assignments-checkpoint-with-refering-values` — persisted results referencing interned IDs are valid only while the interner state that minted them persists; assignments checkpoint TOGETHER WITH (never after) the values that refer to them; IDs never recycle within a live interner; an epoch reset discards both together | `src/interner/mod.rs` doc | B− (drop-or-keep at authoring) | in-source verbatim: "New assignments stay in memory until a durable query checkpoint saves them alongside the values that may refer to them." + "IDs are never recycled within a live interner." Dedup: `intern|interner|interned|string.?cache` over rust_perf + kernel_opt corpora = zero rules. caching category. |
| R5 | `scope-scoped-memo-release-idle-holds-no-decoded-state` — between operations the durable backend holds compact records; decoded values, keys, metadata and locks are NOT resident while idle; memos reconstruct from records on demand | `runtime/cache.rs` doc + blog §Inside Ajika | C+/B− (drop-or-face at authoring) | in-source: "The backend owns completed records between operations; these working objects do not keep their keys, output metadata or locks resident while the database is idle." Distinct axis from the eviction families (SLRU/LRU/bounded pools evict by CAPACITY; this releases by SCOPE with durable reconstruction). Adjacent risk: may read as a positive instance of lazy-materialization — adjudicate in-rule or drop. |
| — | input-stamp completeness ("Cover everything the input value exposes to queries"; "a build counter alone would make unchanged inputs look different every time") | `model/mod.rs` Stamp doc | POSITIVE INSTANCE — recorded, no action | `cross-run-profiles-key-on-content-not-stat-triplets` (kernel_opt gpu_10) already owns BOTH directions: foreign warm start (under-covering key) AND spurious invalidation ("a legitimate copy or touch of the SAME artifact invalidates a still-valid profile"). The gate worked. |
| DROP | red-green revision validation; "cache complexity is multiplicative O(comp × cache)"; retention knobs (max_entries/max_bytes/max_idle) | blog | KILL | ambient architecture (the memo/validity model is salsa 101, not a bounded fix); the architectural argument motivates engines, not a rule; retention = positive instance of the bounded-eviction families. |

**Dedup evidence (both vocabularies, per candidate):** `memo|incremental|salsa|invalidat|recompute` → no incremental-computation rules (hits are prose); `corrupt|throw.?away|discard.*cache|stale.*cache` → the io/artifact families named above, no throw-away owner; `pack.?file|append.?only|blob.?store|redb|lazy.?deserial|on.?demand.?load` → zero (all `offset` hits are GPU buffer-index rules — unrelated axis); `intern*` → zero; `transient` → GPU buffer-lifetime family only (different axis than persistence-scope transience); staged-leads pool → zero. Dedup-at-mining binds.

## GOAT candidate (R1) — the four criteria

1. **Benchable:** yes — synthetic persisted store, N fields × M rows; direct-deserialize-whole vs handle+partial-read; the perf-bench Pareto validator + exact alloc-count axis host it. Bytes-read is a deterministic second axis.
2. **Exact semantics:** yes — both arms return bit-identical values; G1 trivial.
3. **Magnitude story:** reading 1 of K fields of a persisted output deserializes O(total) bytes in the direct arm vs O(field) in the handle arm — ~K× on K-field outputs, scaling with payload size; the framing cost is the honest overhead the bench must include.
4. **Deterministic:** yes — no timing-dependent verdict; same input, same decode tree.

All four pass → GOAT-eligible lead. Pre-registered bars (to be pinned in Plan 370 before any corpus edit): G1 bit-identical; G2 ParetoDominant at ≥2 of 3 sizes on the partial-read posture with ≥3× bytes-read and no wall-clock regression at the read-all posture (the framing tax must not regress the full consumer); G3 no-regression; G4 alloc census. A missed bar demotes in the landing commit (ungoat law).

## Lane-intel axis

**NONE for the league** — no kernels, no serving path; grep of the mechanism keywords against the league surface has no mapping. **Home corroboration note (record, no issue):** the blog's throw-away posture and the stamp-completeness law independently corroborate refine's own `index_cache` design (issue 224/226: content-addressed key over corpus entries + retrieval config + format versions; corrupt/stale refuses loud and rebuilds — the discard-and-recompute posture R2 names). No workspace issue moves.

## Product-coverage axis

**YES — seeds the persistent-incremental-computation family (0 corpus owners today).** Salsa is the backbone of rust-analyzer — top-tier-adjacent ecosystem infrastructure; the family's consumers are any long-lived IDE/analysis/build tool. Follow-up source pool (with the accumulation clause — pool sources read before the batch window fold into the same batch): `salsa-rs/salsa` (THE canonical source), the rust-analyzer "Durable Incrementality" blog (2023-07-24), the salsa book, `adapton`, the blog's own survey rows (`picante`, `turbo-tasks`, `DICE`). The batch's own rule count does not depend on the pool.

## Honest caveats

- **Young + fast-moving:** 0.1.7, first release Nov 2026, author says improvements are in flight — re-clone + re-pin at mine time; every quote below was read at the 2026-11-12 pin.
- **LLM-assisted authorship disclosed by the source**; the in-source module docs are clean and quotable; the BOOK is LLM-generated and is not quote material.
- **Consumer-base narrowness:** Rust Glancer is the primary consumer; salsa remains the ecosystem default (the author says so). The rules are engine-agnostic decision content, not salsa-specific.
- **R5 vocabulary collision risk:** "transient" and "release" collide with the GPU buffer-lifetime family — in-rule adjudication or drop at authoring.
- No `[profile]` tuning in the workspace root (benches exist; the bench evidence is thin — grade rests on decision content + the R1 mechanism, not their numbers).

## Yield estimate

**3–5 rules after ~50% coordinator falsification** (R1 near-certain; R2/R3 near-certain; R4/R5 contestable; the stamp face and three drops recorded).

## Queue slot

Nominal **Batch 359** under **Plan 370** (plan-first, staged before any corpus edit). NOTHING INVENTED — every quote read at the clone or the live blog this session.

Session: zed-amo-ajika-distill, epoch 1794540722
