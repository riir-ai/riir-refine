# Research 126 — the fa-vec offline swept launch-table system (ggml-org/llama.cpp)

**Status:** RECORD (Batch 104 landed 2026-09-05, same cycle — Plan 074 all tasks `[x]`; the
queue line + this note flipped together at landing).

## Source

- Repo: https://github.com/ggml-org/llama.cpp (MIT).
- Infra commit: `efeda76b948f59ee52ea20db640bc4cf3dfe8ac1` (2026-08-24) — the whole fa-vec
  machinery: `ggml/src/ggml-metal/ggml-metal-tuning.{h,cpp}` (runtime table + lookup) +
  `tools/tuning/{main,fa-vec,bench}.cpp` (the offline sweep harness).
- Table pin: `8963a9bdcdf312abc9aab2e662e525c06c9964ec` (2026-08-28, #27863) — adds M3 Max
  (F16 + Q8_0; 91 rows), M5, M5 Pro rows. **This is the league-relevant SKU** (our pinned
  opponent binary is PrismML fork @ `e311ed38f`, which does NOT carry fa-vec — upstream-only).
- Harness provenance: the `tools/tuning/` tree is byte-identical from the pin to HEAD
  `d230ddd76` (b10793) — verified by empty diff; only the table's ROW COUNT moved post-pin
  (1,293 → 3,420 rows across 20 SKUs in two weeks — `9cc33944f` M3, `8887a48f0` M2 Pro,
  `be789c344` A18 Pro, `d08c7872d` M2 Max, `458681e1d` M1 Ultra): the sweep harness is being
  run per-SKU by the community through discussion #27668.
- Clone: `.raw/llama-upstream` (`--filter=blob:none`), removed at batch landing per the
  `.raw` discipline.

## What the system is

The Metal flash-attention vector kernel's optimal launch config is NON-MONOTONIC across five
axes: device SKU, KV dtype, (dk, dv) head dims, KV length (ne11), and query count (ne01).
`Q>1` reuses one K/V load across query rows "so it only pays off once ne01 aligns with Q";
the Q>1 crossover is itself head-size dependent ("small dk crosses late, large dk wins even at
short KV"). A single hand-tuned threshold cannot express that surface. The system:

1. **A standalone sweep binary** (`ggml-metal-tuning fa-vec`) that measures the PRODUCTION op
   path (graph construction "mirrors test_flash_attn_ext::build_graph for the subset this
   tuner sweeps"; K/V are views of a 2×-tall parent "as they are of the KV cache in
   production") over a derived grid, and prints **pasteable table rows** — "stdout carries
   nothing but table rows, so the whole stream pastes into fa_vec_tuned_table"; diagnostics
   on stderr; "never a pass/fail test".
2. **A constexpr runtime table** keyed by an 8-byte packed struct
   (`{device_id: i8, dtype: i8, dk: i16, dv: i16, ne11_b: i8, ne01_b: i8}`,
   `static_assert(sizeof == 8, "tightly packed for memcmp")`), looked up by flat memcmp.
3. **A lookup ladder**: exact `(ne11_b, ne01_b)` bucket → ne11-collapsed domain default
   (`ne11_b == FA_VEC_NE11_DEFAULT`, `ne01_b ∈ {DECODE=0, BATCH=1}`) → retry under the
   family's representative SKU (`gpu_family == 9` → M4 Max rows) → template baseline.
4. **Short-axis skip**: `ne11_b == 0 → return baseline` — "short KV: attention is a small
   slice of the step, left to baseline".
5. **Consumer-side device-limit validation**: "fall back to baseline (Q=1) if the tuned
   config exceeds threadgroup memory" (`FATTN_SMEM(nsg) > props_dev->max_theadgroup_memory_size`)
   — offline data is never trusted at runtime against device limits.
6. **A shared legality constraint**: `fa_vec_legal_ne(dk, dv)` (NE ∈ {1,2,4} legal iff
   NL = 32/NE divides dk/4 and dv/4) is "single source shared by the offline tuner and
   test-backend-ops", and `fa_vec_baseline_ne` is a "hand-maintained mirror; keep in sync
   with those instantiations" — the tuner can only emit configs the kernel actually
   instantiates (399 vec-kernel templates at the pin, `q{1,2,4}_ne{1,2,4}` × 10 shapes).
7. **Axis split, swept vs heuristic**: only Q (queries/threadgroup) and NE (head elements
   per thread) are table-driven; `nsg` (simdgroups/threadgroup) stays a runtime `while`
   heuristic (double up to 4 while `2*nwg*nsg*ncpsg < ne11`) — the axis that is
   shape-DERIVABLE stays a formula; the axis whose optimum is empirical and non-monotonic
   gets swept.

## The measurement methodology (tools/tuning)

- **Anchor + cooldown drift gating**: every 4 candidates re-measure the BASELINE as an
  anchor; if drift > `--cool-drift` (10%) from the coolest anchor seen, discard everything
  since the last clean anchor, sleep with exponential backoff (2s→32s) until the anchor
  returns within `--cool-eps` (3%), re-measure ONLY the dirty window; retries exhausted →
  the cell is **dropped as untrusted** ("Nothing measured, nothing proven: a bucket with no
  surviving sample admits baseline only").
- **Keep gates**: keep a row iff the best candidate beats baseline by ≥2% pointwise
  (`r.t[best_i] < base_t * 0.98`); tune a bucket at all iff aggregate speedup ≥ `TUNE_THETA`
  (1.05); ride a ne11-collapsed domain default iff pointwise regret ≤ `TUNE_TAU` (0.05) at
  EVERY point the bucket covers.
- **The no-harm rule** ("A config may represent a bucket only if it is no slower than
  baseline at every point that bucket covers"), with the aggregate-gate warning: "The
  aggregate gate below sums absolute times, so it can pass on the aligned and deep points
  while a misaligned ne01 pays the mod-Q padding."
- **The padding-witness sampling rule**: "The padded-row waste ceil(n/Q)*Q/n is largest at
  the smallest ne01 of each residue class mod Q, so one of a bucket's first Q values carries
  the worst padding it can ever see, and that value has to be sampled. Otherwise the bucket
  bounds nothing: a config picked on the aligned ne01=8,16 says nothing about ne01=9. This
  covers the padding term only — the per-row cost varies with ne01 too — so it is a floor on
  the evidence, not a proof."
- **Worst-case-regret pick**: the bucket target minimizes `worst[i] = max over points of
  t[i]/bestt`, tie-break smaller Q then smaller NE — never the best-average.
- **Grid derived from runtime semantics**: "nsg is a pipeline specialization constant (1 up
  to ne11=2048, 2 up to 4096, 4 above), so ne11 bucket 1 takes two samples to cover both of
  its regimes. Bucket 0 is not sampled at all: the runtime leaves short KV at baseline, so
  no measurement there can reach the table" — measure where the runtime can consume.
- **Fixture realism is load-bearing for timing**: the mask carries ~20% -INF/zero blocks —
  "the -INF blocks are what drives the kernel's skip-INF path, so this pattern is
  load-bearing for the timings, not just for numerics".
- **Candidate-order shuffling** per cell (seeded by shape) to decorrelate thermal drift;
  op replication to ~50 GFLOP/graph (cap 512 nodes) to amortize dispatch overhead; median
  of odd reps; per-cell RNG seeding mixed with shape (`fa_cell_seed`) for reproducibility.
- **Refusal reporting**: when the no-harm rule refused the oracle pick, print WHERE it loses
  ("+%.2f%% vs baseline at ne11=%d ne01=%d") — "a sweep on another machine then shows where
  the winner loses, instead of just emitting a smaller table".
- **Split-and-concat shards**: `--dtype`/`--dk` filters partition the grid with per-shard
  outputs independent ("the emitted rows for one (dtype, head size) do not depend on the
  others"); "Concatenating the shard outputs in the order the full grid would visit them
  gives the same rows a single run prints".
- **Exception-row sparsity**: per (dtype, dk, dv, ne01-domain) emit ONE ne11-collapsed
  default (chosen to minimize #rows, then total achieved time, then Q, then NE) plus exact
  bucket rows ONLY where the default's pointwise regret exceeds TAU or the default is not
  admissible for that bucket. The shipped table stores exceptions over defaults, not the
  full cross-product.

## Ranked candidates

| # | Pattern | Source | Grade | Corpus home |
|---|---|---|---|---|
| 1 | `swept-shape-bucket-launch-tables` — offline-swept per-(device × dtype × head-dim × axis-bucket) launch-config tables with fallback ladder + short-axis skip + consumer device-limit guard + shared legality + exception rows | tuning.{h,cpp} + ggml-metal-ops.cpp consumer | A− (GOAT lead) | kernel_opt K |
| 2 | `no-harm-bucket-representative` — worst-case-regret pick + pointwise-regret exception trigger + the padding-witness sampling rule + untrusted-cell drop + anchor/cooldown gating | tools/tuning/fa-vec.cpp + bench.cpp | B | kernel_opt K |
| 3 | sample-where-consumable (grid from runtime specialization boundaries; bucket 0 unsampled) | fa-vec.cpp ne11_rep comment | B− | fold into rule 1 decision content |
| 4 | harness-as-shipped-binary ("never a pass/fail test"; stdout = rows only; post rows + log for review) | tools/tuning/main.cpp + README | C+ | fold as rule 1 content (process half) |
| — | mask-pattern-load-bearing-for-timing note | fa-vec.cpp fa_init_kq_mask | C | cite in rule 2 (B97 owns the mask-block-classification mechanism) |

## Dedup evidence (both vocabularies grepped over src/domains/kernel_opt/)

- `anchor-shape-bucketing` (L, entries_popcorn.rs:718): bucketing the autotune CACHE KEY so
  similar shapes share an entry — a LEAD; no swept table, no ladder, no guards. Rule 1
  composes (bucketing the key is step one of the contract) and cites it.
- K family (`panic-safe-autotune-candidate`, `multi-priority-tune-groups`,
  `vector-width-pow2-sweep`, `shape-kind-dispatch`): RUNTIME autotune mechanics — no
  offline-swept shipped table, no methodology.
- B77 `batch-width-routed-dual-tile-twins`: compile-time tile-shape TWINS + "sweep, never
  interpolate" for tile shapes — different instantiation axis; the swept-table machinery
  (ladder, admissibility, device guard) is absent. Compose-cite.
- B103 `shape-class-keyed-capability` / `request-shape-decline-mirror-completeness`:
  capability-VERDICT keying + decline-branch mirroring — adjacent doctrine, different object.
- B97 `proven-shape-static-sibling-identical-results-fallback`: the FALLBACK-LADDER concept
  exists for identical-results shape proofs — different contract (bit-identity vs tuned
  throughput), different guard (shape check vs device-limit check).
- Zero hits: `worst.case|regret|no-harm|no slower than|admissib|padding witness|residue class`
  (except an unrelated superset-scratch rule); `tuned table|launch.table|lookup ladder|
  fallback ladder|pasteable|offline sweep` over kernel_opt — all clean.

## Caveats

- Upstream moves fast (1,293 → 3,420 rows in 2 weeks); all quotes pinned at `8963a9bdc` +
  `d230ddd76` (harness byte-identical pin→HEAD). The M3 Max rows were measured on a 64 GB
  MBP in LOW-POWER MODE — the table is per-box by design, which IS the rule's point.
- Mechanism-grade magnitude: rows only survive ≥2% pointwise / ≥5% aggregate wins, and the
  community replicated the sweep across 20 SKUs — the wins are real and repeated. No
  in-OUR-stack measured number; the in-stack contrast (tiled-flash m16/m32 two-point length
  gate, Bench 805; split-decode 64-split cap, Bench 831; runtime gemv_autotune, Issues
  712/714) is qualitative, not a measured regression.
- Fixtures are ORIGINAL pseudocode (license hygiene); suggestion-only rules — no
  `--fix` path (architectural, the B77/B92/B103 precedent).
- Category K grows 5 → 7 (both rules); the router's K centroid gains 2 members (annotation
  only — the floors re-check covers it).

## Queue slot

Landed as **Batch 104** (Plan 074), same cycle as the verdict. Mined-by: riir-clippy idle
loop 2026-09-05.
