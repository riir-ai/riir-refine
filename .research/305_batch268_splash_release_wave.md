# Research 305 — Batch 268: the splash release wave (incoai/splash be3b083c..origin/main)

**Status:** RECORD — LANDED 2026-10-21 (shikuwa/4090 idle lane, Plan 273).
Corpus 926 → **937** (+11 own kernel_opt rules, entries_gpu_117: A +1, C +2, D +3, E +1,
H +2, I +1, O +1). The wave's host-side serving leads are STAGED for a rust_perf batch
(Batch 270 pool below) — freeze regen + binary rebuild is a lane of its own.

## The source

incoai/splash `be3b083c..origin/main` — the 2026-10 release wave, **205 non-merge commits**
(research/01-memory → 11-cache-quota + the model/kernel lanes). Apache-2.0. Provenance:
staged by the B267 standing-watch re-poll (Research 303 §STAGED) with range-end `1e532541`;
reconciled at T0 — the delta past `1e532541` (36 commits) carries the Spill-series leads, so
the batch scope is origin/main (the 205 count matches exactly). Prior splash owners: B236
(mid-prefill capture boundary), B247 riders (KV-ALIKE + retained-rows oracles), B237
(discretionary-cache-growth) — none overlapped this wave's rules.

## The 11 landed rules

| rule | cat | source | law in one line |
|---|---|---|---|
| apple10-narrow-grid-register-tile-threadgroups-per-core | D | `97c55fa` | decode kernel-variant selection by a threadgroups-per-core occupancy bound (<1.5 tg/core; halved for sub-4-bit formats) |
| splitk-tier-ladder-per-tier-ceiling-min-partition-inputs | E | `6198321` | split-K depth from a tier ladder (trigger, inputs floor, per-tier ceiling); ceiling 16 tier-keyed; reassociation disclosed |
| mma-lane-map-whole-run-vector-loads-split-accumulator-chains | A | `d601971` | MMA lane map re-indexed to whole 8-input runs per lane; even/odd split accumulator chains; re-measure splits after a lane-map change (16→8) |
| constexpr-unroll-capacity-bound-zero-guard-holds-array-in-registers | O | `694243c` | runtime-bound loop over a local array → capacity-bound zero-guarded `#pragma unroll` keeps it in registers |
| prefix-active-split-axis-linearized-one-dimensional-grid | D | `ee91716` | a data-dependent-prefix split axis linearizes into x (bit-identical by order preservation); up to 1.7× faster past ~32 splits |
| moe-lone-tile-register-gemv-uniform-tile-kind | H | `ed63ffb` | an expert's lone ≤2-row tile runs a lane-per-column register GEMV; ALL of one expert's tiles one tile kind |
| moe-unverified-rows-route-zero-experts-dense-shape | H | `a8c10ad` | rows past a lane's verify count route ZERO experts while kernels keep the dense shape; rows priced by expected tokens per cost (online fit, 2×/½× clamp, every 32nd cycle full) |
| block-sparse-union-lists-packed-entry-token-major-values | I | `4a16aaa` | per-tile union block lists, `(block<<ROW_BITS)\|row_bitmask` entries, token-major values, closed-form split sizing (48-entry/32-split floors) |
| same-input-consecutive-projections-one-dispatch-concatenated-tiles | C | `4944799` | N projections of ONE input share a dispatch via concatenated column-tile ranges; each keeps its own tile; bit-identical |
| consumer-prepares-consumed-table-producer-writes-plain-rows | C | `55b0071` | a table built by a row-serialized producer (one core per row) moves into the consumer's existing covering prepare phase; byte-identical |
| wide-row-tile-under-bits-per-weight-floor-one-lane | D | `b701328` | one-lane decode under 4.5 bpw takes the 16-row tile over two storage lanes (NaN-padded); rows independent so bit-for-bit |

Citation corrections applied at authoring (the read agents' draft gpu tags re-verified
against the corpus): `splitk-width-shape-need-gate` + `splitk-atomic-free-contention-threshold`
sit in gpu_37 (not gpu_56), `splitk-reduce-one-launch-slice-order-pinned` in gpu_67 (not
gpu_99), and the R3 kinship is `c-fragment-to-a-operand-register-repack` (not a-fragment).

## Dropped (with reasons, the A-slice ledger)

`582ce29` (layout-granularity enablement, no measured decision) · `2d4ed96` (loader/format
capability) · `8ed0ede` (loader support; exact-widening corollary already owned) ·
`4d8aed8` (capacity plumbing; the mixed-format one-pass face adjacent-owned by
stacked-gate-up + the fusion family — absorbable as a rider, not standalone) · `8d548f8`
(unify-refactor; its measured win rides the column tile) · `6d5b97d` (slot-table
enablement plumbing) · `e3837ba` (repo-specific GDN scratch invariant) · `8d5e05b` (thin;
adjacent-owned by the argmax-epilogue family) · `d77a61f` (build-once dedup, trivial) ·
`dc65031` (same-family substrate as expert streaming; poison-on-drift face noted) ·
`79d740f`/`0d1dec1` (planner consistency, no perf decision) · B-slice drops: `1c7e0c1`
(DRY unification, no reusable decision), `3ac750f` (enabler), `51ae046` (config honesty),
`9ee89b4` (logging-first), `cfa91f5` (test seam — the B247 rider's quoted symbol relocated
to RuntimeInstrumentation, body unchanged, provenance stays valid).

## STAGED — Batch 270's pool: the rust_perf serving leads (NOT landed here)

Eleven full proposals + 4 rider faces + 13 second-tier candidates from the B-slice read,
all rust_perf-lane, all zero-corpus-owner verified by grep at read time:

- `reclaim-progress-keyed-on-refused-resource-returned` (allocation; `9ca9768`+`f6957e8`+`32cf878`)
- `refusal-wait-keyed-on-shortfall-returning-transfer` (concurrency; `fdc8f01`+`c27aaf7`+`e90b43c`+`a7406bb`+`41e8c82`)
- `restore-point-chain-copies-keyed-on-disk-state-count` (caching; `a20f344`+`65f9713`)
- `persistent-cache-teardown-needs-second-witness` (caching; `6e21cc9`)
- `hot-slot-recycler-flat-tables-zero-alloc-claims` (caching; `a7e5347`+`f3eb1a0` — 13 ns/entry vs 22)
- `cache-share-capped-at-measured-distinct-working-set` (caching; `18f9c69` — hit-flatness sizing)
- `cache-reuse-assertion-gated-on-eviction-counter` (caching; `2712e67` — the third serving-cache oracle law)
- `publication-takes-over-condemned-cache-entry` (caching; `716760d`)
- `watchdog-deadline-moves-on-command-progress` (dispatch; `8df9e95`)
- `page-miss-blocks-one-ranged-read-scatter` (io; `c7a42cc`+`c14ba73`)
- `tier-write-defers-to-urgent-reads-except-a-read-behind-it` (io; `fc6fa54`+`be30994`+`47325f9`)
- Riders: runway-refilled-by-consuming-op (`945635d`), release-unit-matches-demand-unit
  (`d26ff04`), retry-hint-from-refill-ledger (`ea1683d`), form-keyed reusable counts (`41e8c82`)
- Second tier (grade B/B−): `253506f`, `88d0e57`, `91d4ddb`, `4d17995`, `fa63d97`+`593f2a6`,
  `68b9e70`, `81ae003`, `ed1092f`+`2f5fe8d`, `dab8841`+`6fcd9b1`+`25502fd`+`7e51fcb`,
  `5ea4043`+`82b93c5`, `b7844dc`, `4fc7465`, `e2c0af2`

Landing cost note: rust_perf corpus delta ⇒ gen2/gen3 freeze-table regeneration + score-bench
run + the release-set binary rebuild, all in the landing commit (plan 137 T7.5; the
Issue-153 release feature set). Also staged: the intel rows (expert streaming ARC +
router-predicted read-ahead `99dcf73`, host steps `5117250`, residency-set partition
`c608e23`, constrained+unconstrained one command `f52d1ad` — the last measured −23.7%/−34.0%
time/token on 35B/27B mixed-lane traffic, rust_perf:dispatch candidate).

## En-route instrument lessons (this session)

1. **`cd /tmp/...` + multi-root workspace**: the terminal tool refuses `cd` outside the
   project roots; /tmp-scratch commands must run from a project root with relative or
   absolute child paths (the `.raw/` gitignored dir is the scratch surface).
2. **Slice-agent sizing**: a 6-rule authoring prompt hit the subagent stream-idle timeout
   twice (120 s no-events); 3-rule prompts complete. Split authoring slices at ≤3 rules.
3. **Watchdog-cancelled agents can still have written their file** (a2's file landed at
   10:47 before the cancel) — check the filesystem before re-running; but its opt_patterns
   were unvalidated (referenced ref_shape items) and were repaired by the coordinator:
   every corpus fixture must compile STANDALONE — inline the shared items with a
   provenance comment.
4. **Feature-gated test discovery**: `cargo test --lib batch268` filters 0 tests when
   kernel_opt is off (the domain compiles to nothing — the green-zero class, known);
   the needle gate runs at `--features kernel_opt`.

## Gates (all run this session, isolated target /tmp/b268_rel where parallel)

- batch268 needle gate `batch268_rules_landed` FIRST-RUN PASS (33 verbatim needles + count).
- kernel_opt lib **3208/0** (1 ignored documented); default lib **2726/0**.
- kernel_opt+latent_retrieval lib **3395/0** (3 ignored documented; 1994 s).
- clippy −D ×3 postures clean (default / kernel_opt / no-default); en-route the healer
  (`cargo refine --fix --write --verify`) healed 5 doc_markdown findings in the new file's
  header doc; the secret-scan's 8 suggest-only findings on the file are FALSE POSITIVES on
  kebab-case rule ids (`-mask-bit…` shapes) — synthetic documentation text, no secrets.
- Retrieval floors HELD 22/22 no re-pin (kernel self-variant top-1 green; blended
  90.6/94.2/95.3 vs floors 89.4/93.6/93.6; 332 s).
- freeze/score-bench NOT owed (zero clippy-corpus delta — P30); binary rebuild NOT owed
  (kernel_opt outside the release set).
