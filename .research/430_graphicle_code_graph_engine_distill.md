# 430 — Graphicle code-graph engine distill (Graphicle-AI/graphicle)

**Status:** LANDED — Batch 415 (2026-11-25, `entries_102` rust_perf + the sec pair; the R1 WAL feed landed as the staged Plan-385 R2's authoring with this note's second instance folded in; R2 decided FACE on entries_62, R5 decided FACE on entries_78; see the snapshot LANDING row)

- Source: https://github.com/Graphicle-AI/graphicle — PIN `b0bb41407d1ffe863221181112656c28c483046c`, tip 2026-11-21T17:44:08+05:30, Apache-2.0, 606★, 357 commits, ~392k lines of Rust across three crates (`graphicle-graph` the engine lib, `graphicle` the runtime/service bin, `graphicle-memory` the memory lib). Shallow clone `riir-refine/.raw/graphicle`, REMOVED at close.
- Inspection date: 2026-11-21 (m3, zero cargo, zero GPU — docs-only verdict lane).
- Batch plan: `.plans/396_graphicle_transcription_batch.md` (staged, unclaimed).
- Provenance chain: graphicle IS a port — every file carries `// Adapted from Graphify 0.9.103 (16218bfc), Apache-2.0; Upstream: graphify/<file>.py` headers plus a per-file pinned port commit (`//! Port of graphify/ids.py at 20a20d30…`). Graphify (Python, Safi Shamsi) is the upstream of record for the deep variant set; graphicle is the Rust shape of interest and the measured-optimization story (README: Kubernetes 25,899 files 3m20s vs Graphify 33m11s build; 21s no-op update; "after ten updates, Graphicle's graph matched a fresh build of the same commit").
- NOT the scikit-hep particle-physics `graphicle` (PyPI package of the same name) — the web-search first hit routes there; the GitHub org Graphicle-AI is the coding-agent code-graph product. Disambiguation recorded so future citation sweeps don't cross the streams.
- Same product class as colbymchenry/codegraph (Research 418 / Batch 369/371) — the dedup gate ran against the codegraph mined set FIRST (below); graphicle is native Rust (no FFI plane), SQLite/Postgres store, tree-sitter × many languages, MCP server, git-hook sync.

## Verdict

**YES B− corpus (transcription batch — nominal Batch 379 window, Plan 396) / lane-intel NONE / product-coverage YES — the staged SQLite-WAL io family gains its SECOND engine instance before the family even lands.**

No benchable GOAT lead — every candidate fails criterion 1 for a domain/pattern reason (long-horizon io durability metrics, contention wall-time, policy gating), which is the Batch-136 annotation class, not a demotion: the corpus's io/sec families are built this way. No outbound-PR lane — every atom is the source's own landed decision at the pin (they measured and fixed their own 2026-11-03 store outage).

## Dedup gate (evidence per candidate, both vocabularies, live corpus this session)

| Candidate | Vocabulary greps | Verdict |
|---|---|---|
| WAL starvation / checkpoint valve | `wal_bulk\|checkpoint\|journal_size\|starvation` over corpus = only ds4-metal resume-checkpoint entries (distinct: mid-compute state, not WAL); `wal-bulk-write` STAGED (snapshot L2826), NOT landed — grep confirms | Second engine instance of the STAGED rule — feeds authoring, never a new rule id |
| io-workers derate | `available_parallelism\|io_workers\|spawn-per-call` → `thread-spawn-per-call-pool-reuse` (e72, spawn axis), `dynamic-work-parallelism-ladder` (e06, runtime ladder), `online-cost-fit-thread-ladder-reopen-hysteresis` (e62, adaptive reopen), `parallel-disjoint-pread` (e05, pread axis) — none owns a STATIC measured platform derate for open-dominated work | Distinct; face-or-rule vs e62 adjudication at authoring |
| SQLite NOFOLLOW store path | sec corpus `nofollow\|symlink` → files.rs symlink-LISTING face (L600-603, path-join-escape rider), archive-extract wave (L1354) — archive sinks owned; a DB-store-path sink is unowned | Distinct sink; adjudicate in-rule |
| Resolve-then-pin egress | `rebind\|dns\|pinned` → `browser-reachable-service-host-origin-rebinding-gate` (INBOUND handshake gate), `unvalidated-egress-url` (B342, URL-CHOICE + static allowlist/private-range on the URL string) | Distinct third arm: validate the RESOLVED ADDRESS per connection, defeating DNS rebinding on egress |
| No-op-rebuild digest gate | `early.stop\|unchanged` in rust_perf corpus = no owner; kernel-side gpu_35 key-completeness is the different-lane kin; entries_05 resume-identity is build-level identity (adjudicate in-rule) | Distinct; weakest candidate |
| test-file downrank / BM25 / spreading activation | corpus zero (retrieval-quality is not a code-fix class) | NOT a corpus rule — lane-intel only |

Source-id grep: `graphicle` in `.distill/001_mining_queue_snapshot.md` = 0 — no staged-lead conflicts.

## Ranked candidates

| # | Pattern | Source file @ pin | Grade | Notes |
|---|---|---|---|---|
| R1 | WAL starvation behind a live reader + the checkpoint-window remedy | `bench/store-load/README.md`, `crates/graphicle/src/db.rs` | **B+ second-instance FEED** | `wal_starvation.py` verbatim: "with a reader always in flight, a PASSIVE autocheckpoint never resets the WAL, so the WAL grows by everything written" — the staged codegraph rule's futility axis, measured by a SECOND engine with a dated outage (2026-11-03) and a kept reproduction harness. Remedy trio: `journal_size_limit` 64 MiB ("A write-ahead log that a checkpoint reset is cut back to this size rather than kept at its largest", db.rs:2305-2307) · `checkpoint_gate` (db.rs:1214) — a checkpoint under an ELEVATED busy timeout (`CHECKPOINT_WAIT`, db.rs:2905-2910: `busy_timeout(CHECKPOINT_WAIT)` then "restored = busy_timeout(self.busy.get())") · `wal_report()` counters (`WAL_CHECKPOINTS` / `WAL_CHECKPOINTS_BUSY` atomics, db.rs:1292-1302) · Postgres migration under sustained load (store-load records `pg_stat_wal`). Lands as adjudication + the checkpoint-window atom folded into Plan 385's pending R2 authoring, or a companion rule if the valve rule splits. UPSTREAM: their own landed decision at the pin. |
| R2 | Measured io-worker derate (open-dominated work) | `crates/graphicle-graph/src/par.rs:23-31` | B− face-or-rule | io_workers() verbatim: "On macOS, opens from more than about six threads contend in the kernel: measured on an M3 Pro, reading and hashing 11k files took 0.44 s on 4-6 threads and 0.67 s on 12-16." Law: worker count for open/read-dominated work is a MEASURED platform constant below `available_parallelism`, env-capped (`GRAPHICLE_MAX_WORKERS`), results order-invariant. Adjacent-DISTINCT from entries_62 (adaptive RUNTIME ladder vs static platform derate) — adjudication in-rule mandatory. NotBenchable-by-model (contention wall-time). |
| R3 | Egress resolve-then-pin (DNS-rebind defense on OUTBOUND fetches) | `crates/graphicle-graph/src/security.rs:241-289` | B− | `validate_url_with` resolves the host THEN checks every resolved address against `ip_policy_data::BLOCKED_V4/BLOCKED_V6` (u128 range tables, generated reference-policy data); `resolve_and_validate` returns the PINNED SocketAddr for the connection; cloud-metadata hostnames refused by name. The corpus owns URL-choice (B342) and the inbound rebind gate — THIS owns resolve-then-validate-the-address. The `Resolver` trait seam is the testability design. Suggest-only, NotBenchable-by-domain; adjudication both directions vs B342 in-rule. |
| R4 | SQLite NOFOLLOW store path (resolve the parent, refuse the link) | `crates/graphicle/src/db.rs:2311-2325` | B− | Verbatim: "SQLite's NOFOLLOW refuses a link anywhere in the path, not only at the file (macOS temporary folders sit under /var, a link to /private/var). Resolve the folder, never the file, so a file that is a link is still refused." The parent-canonicalize + `SQLITE_OPEN_NOFOLLOW` pair on a store-path sink — archive-extract and path-join-escape own different sinks. Also `secure_delete` + read-only URI mode for the read posture. Suggest-only. |
| R5 | No-op-rebuild gate keyed on output-input digests (with the stateful honesty) | `crates/graphicle-graph/src/watch_gate.rs:2-14` | C+/B− drop-or-keep | The early-stop record digests EVERYTHING the output depends on (engine build incl. grammar packs, settings, scan stats, config files, out-of-scan inputs); the honest clause is load-bearing: "A rebuild that writes a new graph records nothing and removes the record: its output depends on the stored graph it read" — an early-stop gate over a STATEFUL pipeline must invalidate on any output change. Positive instance at home: refine `index_cache` (corpus-stamp rebuild). Adjacent gpu_35 (kernel key-completeness) + entries_05 (build identity). Weakest candidate; face-or-drop at authoring. |
| A1 | test-file downrank at 0.3 in retrieval ranking | `retrieve.rs:10-17` | ANNOTATION | "Test files rank at 0.3 of their score unless tests are asked for: they repeat the names of the code under test and would outrank it." Retrieval-product decision content — routes to riir-neuron-db's RAG lanes as intel, never a refine corpus rule. |
| P1-P7 | Positive instances (record, never land) | various | — | P1 chunked-counter order-preserving parallel map (par.rs `map_with`, batch `(len/(workers*8)).clamp(1,64)`, per-thread local results) = entries_06 family + refine's own byte-prefix scan lane; P2 `Command::new("git").arg("-C").arg(root).args(args)` pinned-program args-array spawn (hooks.rs:97,636) = staged atlas rule instance; P3 bounded fetch/metadata caps (MAX_FETCH_BYTES 50 MiB, MAX_TEXT_BYTES 10 MiB, MAX_LABEL_LEN 256…) = unbounded-read/length-prefix families, plus the honesty note "METADATA_MAX_ATTRIBUTES … Upstream defines but does not enforce it" (they enforce); P4 graph-parity TRANSCRIPT digest vs the replaced implementation (graph_parity_tests.rs) = house G1 discipline; P5 ASCII fast-path + iterated casefold-NFKC fixed point with the compat-honesty note ("Rust regex \w … must not be substituted here", ids.rs) = compat-port annotation; P6 watch-gate determinism law "Its output depends on what it reads (never on the clock or the number of workers)" = box-state/determinism law independent arrival; P7 `dev` profile `debug = "line-tables-only"` + thin-LTO release = build-hygiene discipline independent arrival. |
| X1 | graph_parity_tests.rs local `fn hex` uses `format!("{b:02x}")` per byte | graph_parity_tests.rs:19-21 | NON-FINDING | The landed B369 hex-encode-lut rule's regime conditions (hot id path) correctly exclude this cold parity-transcript helper — noted as the regime gate working, never a counter-example. |

## Lane-intel axis

NONE — the source is a code-index/agent-memory tool: SQLite, tree-sitter, MCP, git hooks. Zero mechanism overlap with the league lanes (grep `sqlite|tree.sitter` over `../riir-ai/.issues/` = 0 hits). No league line owed.

TOOL-LANE (named, not filed — the atlas precedent): graphicle's incremental resolution (`incremental.rs` — content-addressed per-file hashes + conservative dependency closure + "callers depend on provider hashes, not on other callers") and the no-op watch gate are the same design class as refine's `index_cache` (corpus-stamp keyed, refuse-loud-rebuild) — intel for the index_cache incremental-rebuild lane if it ever opens; the store-load bench methodology (p50/p99 read latency under concurrent bulk write, strace fsync counts, WAL/`pg_stat_wal` size tracking) is a harness reference for riir-neuron-db's LocalKv lanes. Nothing filed from this verdict.

## Product-coverage axis

YES, modest: the SQLite-WAL io family (staged via Research 418, follow-up pool already named: SQLite WAL pragma docs, rusqlite, turso, sqlx) gains its second engine instance BEFORE the family lands — graphicle's outage + bench + remedy trio is the strongest evidence base the staged R2 authoring could ask for, and adds the Postgres-WAL face (`pg_stat_wal`). The sec DNS-rebind family gains its outbound arm candidate (R3). No new famous-lib family seeded (rusqlite is the dep surface; the family exists in staging).

## GOAT adjudication

Criterion 1 fails for every candidate: R1 is a durability-under-concurrency metric (the source's own harness is a system bench, not hostable in the Pareto validator); R2 is contention wall-time; R3/R4 are policy gates; R5 is rebuild-skip wall time on a real tree. All land suggestion-only `NotBenchable` decision content per the Batch-136 precedent — the PATTERN-weak class is NOT what failed here (each atom carries a measured basis or an exact mechanism), so the grade holds at B− rather than dropping.

## Yield estimate

2-3 rules + 1-2 faces after ~50% falsification: R2/R3 near-certain to survive authoring; R4 contestable; R5 drop-or-face; R1 lands as the staged rule's second-instance adjudication (+ possibly the checkpoint-window companion). Plus 7 recorded positive instances.

## Honest caveats

- Fast-moving: tip moved TODAY (0.1.19); re-clone + re-pin at T0 per Plan 396; quote re-verification per row (quotes above read directly at `b0bb4140` this session).
- Port discipline means most mechanisms are ALSO upstream Graphify's (Python) decisions — cite graphicle for the Rust shape, Graphify for the lineage, per the thermite precedent (cite BOTH).
- UNREAD residue (re-read rows in Plan 396 T0): `retrieve.rs` body, `serve_*` bodies, `graph.rs` full body, `vtab.rs`, `unicode15.rs` tables, `hooks_runtime.rs`, `snapshot.rs`, `cluster.rs`/`cli_cluster.rs` (Leiden), `extract*/` walker bodies, the desktop/ + sdk/ trees, `bench/` run bodies.
- 392k lines inspected by targeted reads only (~15 files) — module-level grading like thermite; the engine core files read clean and bench-disciplined, but per-file hazard claims are limited to the read set.
- `graphicle-memory` (LoCoMo-benchmarked agent memory) barely read — it is a product plane, not a rules surface, and the memory-comparison methodology is a separate research axis if ever wanted.

## Queue slot

Nominal Batch 379 window behind the staged queue (Plan 395 maya, Plan 394 colorless, Plan 393 qda, Plan 390/391 laya, Plan 389 atlas, Plan 385/371 codegraph remainder). R1's second-instance feed gives Plan 385's pending authoring priority if the owner wants the WAL family landed first.

NOTHING INVENTED — every quote read directly at the pinned clone this session; every corpus/staging claim a live grep this session.

Session: zed-m3-graphicle-distill, epoch 1795236000
