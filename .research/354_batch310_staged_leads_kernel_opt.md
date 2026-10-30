# Research 354 — Batch 310: the 10-30 staged leads kernel_opt conversion

**Status:** RECORD — LANDED 2026-10-30 (Plan 315); gates GREEN on amo (the m3 stayed Plan-337 zero-cargo through the i1117 T4 collection); the en-route gate catches + the box disclosure in §Gates.

## Scope and provenance

Batch 310 converts the kernel_opt half of the 10-30 staged leads addendum
(zed-m3-leads1030, snapshot rows S1/S2/S4 + the R2 rider face): THREE own rules,
ONE count-neutral rider face, corpus 1023 → 1026. S3 + R1 + R3 DEFER to the
rust_perf tier behind the B301/plan-306 claim (to 10-31T13:41Z). The session ran
ZERO-CARGO throughout (Plan-337: the i1117 T4 128K dense collection held the box
at ~10 cores / 15 GB RSS) — T0 source verification, T1 dedup, and authoring are
text work; the G1–G3 + retrieval gate layer is DEFERRED, never skipped, and the
batch does NOT land until it runs.

Sources (all fetched as per-commit `.patch` into `.raw/b310/`, quotes
byte-verified at the pins, plan 315 T0):

| Source | Commit | License | Rules |
|---|---|---|---|
| tracel-ai/cubecl #1971 | `4549408587cf464cc2ae7b3b7bc0bcf6005b5a92` | MIT OR Apache-2.0 | S1 + S2 |
| tracel-ai/cubecl #1975 | `07cd07a2da9c4405044993c6ec2dc51b9d685508` | MIT OR Apache-2.0 | (the same #1966 series, the Metal+smem-reuse landing) |
| PrismML-Eng/llama.cpp #415 | `9d9efeadbdd1d77458d98717a96fbfa3a016e159` | llama.cpp MIT family | S4 |
| tracel-ai/cubecl #1697 | `a72eb4e83cff55ab81f9cbdf24f0d2706af07a3d` | MIT OR Apache-2.0 | R2 rider face (gpu_130) |

## The three laws

1. **`mma-accumulator-run-as-one-operand-depth-matches-instruction`** (B) — a
   self-contracting product's accumulator run is one A operand: instruction
   depth must equal the RUN depth, never one accumulator's width (m16n8k8 runs
   at half the m16n8k16 rate — "1.97 ns a call per SM for either"). The run is
   a layout fact (the manual-mma 16-bit A layout already lays the accumulators
   consecutively); a run of one stays legal. B205/gpu_75 (C-operand alias)
   kinship is compose-never-merge — the source names the boundary itself
   ("`MmaData::mma` loses its accumulator arm").

2. **`barriers-declared-before-slots-one-free-run`** (E) — SMEM declaration
   order is an allocation-fragmentation control: in a ring whose slots free
   while their barriers stay live, per-slot [data, barriers] interleaving cuts
   the freed space into pieces that fit nothing (the 64 KiB partial placed
   after the ring → 131 KiB past a 99 KiB ceiling); barriers-first leaves the
   freed data as ONE run. B234/gpu_94 owns the lifetime-disjoint SHARING axis;
   this owns the ORDER axis inside one ring's free walk.

3. **`warp-per-row-small-blocks-cta-under-load-law`** (D) — blocks-per-work-unit
   < threads-per-CTA idles warps 1..N in a whole-CTA block-stride loop; the
   warp-per-row re-map (warp w owns row row0+w) is admitted only while
   `blocks_per_row <= warp_size` (one block per lane keeps the butterfly).
   Bit-identity is a construction property: with ≤1 block/lane the generic
   path's per-lane accumulation IS one block, so both paths compute the same
   butterfly over the same lane-to-block map. gpu_49 (the plain-matvec
   crossover), B180 (the planar layout the block-dot call consumes), B261 (the
   opposite resource corner) are kinships. The league leg (this is the b10831
   scored-tag tip's headline commit) rides the riir-ai watch — never this corpus.

## The rider face (R2)

gpu_130 `autotune-candidate-priority-orders-try-first` += the QUEUE-ONLY face
(cubecl #1697): a tune cache PENDING or COMPILED-with-kernels-queued under a
CompileOnly override is the THIRD consumption surface beside the measurement
round and the recorded-tune replay, and it must consume candidates in plan
order ("so the best ranked candidate that launches runs rather than the first
declared"); the repair extracts `execute_in_plan_order` over the new
`TunePlan::order()` whose contract ends "last in declaration order, in no
batch". One plan order, three consumers — declaration order is never one of
them.

## Deferrals and the R3 home correction

- **S3** `metadata-count-query-builds-no-structure` (rust_perf) — T0 found a
  NEW measured datum the staged row lacked: "cubek's attention prepare ...
  goes from 12.7 to 9.6 us a call on an RTX PRO 6000 host (10.3 before that
  change)". Rides the deferral for the next rust_perf slot.
- **R1** `allocation-window-resolved-at-stream-birth` (rust_perf/concurrency,
  cubecl #1963) — the staged "rider-or-small-own, home adjudicated at claim"
  adjudicates rust_perf (a host-side stream/window stack law).
- **R3** the B200 tool-turn face — **HOME CORRECTED AT AUTHORING**: B200
  `prefix-cache-entry-one-token-early` lives in the RUST_PERF corpus
  (`rust_perf/corpus/entries_13.rs`, PerfRule, Batch 200/Plan 198); the T1
  grep hits in kernel_opt's entries_gpu_79/82 were distinct-from COMMENTS,
  not the rule home. The staged row's "serving tier" hint was right; the
  claim row's "kernel_opt confirmed" is WRONG and the landing row discloses
  the correction. All three defer behind B301/plan-306 (to 10-31T13:41Z).

## En-route repairs

- The D/E slice header counts in `corpus/mod.rs` sat STALE (239/90 vs the
  measured 240/94 — the Research-353-flagged drift class); repaired to
  measured truth (241/95 after this batch's +1 each) with the miss disclosed,
  the B309 B-slice repair precedent.
- `.plans/.highwater` was stale at 313 while plan 314 existed (the B309
  session's miss) — repaired to 315 by the claim commit (`fdcbd8da`).

## Count pins moved (authoring-time; gate-verified at landing)

corpus 1023 → 1026 at 38 witness sites; category table B 116→117 / D 240→241 /
E 94→95; the sum-comment + the count-witness comment; the B/D/E
KERNEL_CATEGORIES rule_id lists + slice headers; AGENTS.md + README +
`.docs/11_domains/kernel_opt.md` ledger row. `batch310_rules_landed` (needles
over all three rules + the gpu_130 rider) and `batch310_pseudocode_parses`
(all six fixtures through the L2 verdict) are appended beside the batch309
gates.

## Gates — GREEN at landing (ran on amo; the m3 stayed Plan-337 zero-cargo)

The i1117 T4 collection still held the m3 at the authoring close, so the gate
battery crossed to amo (M5 Ultra, same pinned 1.99.0 toolchain, same
aarch64-apple-darwin target) via a git bundle m3→amo direct — the unverified
corpus never touched origin. Box witness at launch: amo load 1.74/1.96/3.41,
899 Gi free; the m3 witness at the same hour: the collection at ~10.2 cores /
15.8 GB RSS (Plan-337 honored — zero cargo ran on the m3 the whole session).

- G1 batch310 needles+parse: PASS
- G2 kernel_opt lib: PASS; default lib: PASS
- G3 clippy -D ×4 postures: default PASS; kernel_opt PASS; no-default-features
  PASS; all-features PASS
- Retrieval floors: R1 (kernel_opt,latent_retrieval) PASS — no decay, no
  re-pin; R2 (default latent_retrieval) PASS
- Freeze/score-bench NOT owed (zero clippy-corpus delta, P30); binary rebuild
  NOT owed (kernel_opt outside the release set)

First-pass catches, fixed en-route (the gates doing their job):
1. The router expert-total pin sat at 1023 — the authoring T4 bumped the
   tests.rs witness sites but missed the router.rs site (the B309 "two sites,
   not one" precedent, now fired twice); fixed 1023→1026 + the B310 message.
2. Five clippy findings in entries_gpu_141 (3× unnecessary raw-string hashes,
   2× doc-backtick items) — fixed; rustfmt-clean re-verified.
3. G3c environment note (amo-only, pre-existing, not corpus-caused): the
   non-interactive ssh PATH lacks /opt/homebrew/bin, so sentencepiece-sys's
   cmake build script failed at the first pass; proven at develop (identical
   failure on e1c614e8 in a second worktree) and PASSED with the PATH fix.

## Session

zed-m3-b310, epoch 1793317704 (2026-10-30 ~06:48 +07). Claim `fdcbd8da`;
plan-correction push `971fce68`; authoring in the detached worktree
`/tmp/wt-b310` (the shared checkout untouched after the claim). Claim expiry
48 h (2026-11-01T~23:48Z); the push is the arbiter.
