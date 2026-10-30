# Plan 317 — Batch 317: the 10-30d staged leads kernel_opt pool (S7 + S5 + S6 + R5)

**Status:** IN PROGRESS — claimed 2026-10-30 ~05:5xZ (zed-m3-b317, M3; T0/T1 DONE PRE-CLAIM — all ten TensorFold pins resolved in `.raw/b317/tf`, every staged quote byte-verified, dedup greps ruled in-row; authoring T2–T4 STAGED behind the batch-316 sibling lane + the gate window; the claim row is the queue snapshot's head).

## Scope

The 10-30d staged leads addendum (zed-m3-leads1030d, snapshot STAGED LEADS ADDENDUM),
converted **kernel_opt-only** under the B293 cross-domain precedent: rust_perf/sec rule
counts ZERO this batch (S3/R1/R3 stay behind B301/plan-306 to 2026-10-31T13:41Z;
batch-316 owns the rust_perf slot in the sibling lane). S10 (splash `a56988e`) is NOT in
this batch — its home is the splash staged pool (Research 345's lane-B ANE/host five);
handed to that lane. S8/S9 stay in the serving/spec-dec tier (untouched).

Box posture (Plan-337): the i1117 128k decode-gate collection holds this box (PID 19526,
~10 cores, 880% CPU since 18:42Z 10-29, hours remaining) and the batch-316 sibling lane
holds the shared corpus files (`entries_01/mod/tests/entries_55`) — **T2–T4 (authoring +
wiring + pins) touch `mod.rs`/`tests.rs`, shared conflict surface, so they wait for the
sibling's landing**. T0/T1 (patch fetch + quote byte-verification + dedup greps) are
git/grep-only and were run pre-claim. NO unverified corpus reaches develop (the amo
10-30 incident class); the gate layer is DEFERRED, never skipped — native isolated
`CARGO_TARGET_DIR` when the window opens, or the amo git-bundle fallback (plan 315's
precedent). Gates at the native 1.99.0 pin (the bump landed 10-29; no RUSTUP_TOOLCHAIN
override). freeze/score-bench NOT owed (kernel_opt-only, P30); binary rebuild NOT owed
(kernel_opt outside the release set).

## Candidate slate (staged grades; T0 re-verified this session)

| # | Lead | Source | Grade | Disposition |
|---|---|---|---|---|
| S7 | `ring-scan-grouped-load-ahead-wrap-step-same-fold-order` — THREE atoms: (a) GROUPED SCALAR LOAD-AHEAD on a ring scan (`AHEAD = 8`; `uint4 kn[AHEAD], vn[AHEAD]` register groups via `__ldg` "read-only path, as a __restrict__ ring's"; the next group issued while one folds — `if (end - pos >= 2 * AHEAD) load(kn, vn)`); (b) WRAP-STEP REPLACES THE PER-SLOT 64-BIT MODULO (`slot = slot + 1 == w ? 0 : slot + 1` — works for non-pow2 widths; the modulo was half the named per-slot cost); (c) SAME-TERMS-SAME-ORDER bit-identity as the legality contract (trees equal chains, batched equal serial on-GPU: mixed depths, picked rows, a gap, reused contexts, 17 streams). Measured: ring attention 1.6 ms → 0.24 ms (6.7×), single-stream decode 90.6 → 103.2 tok/s on GB10 (+13.9%), source-reported | TF `6428050c` (the helper, 10-28) + `1ff930d9` (the batched==serial contract, 10-27) + `2c10cfc1` (the stream-batching infra, "with the same bits as alone", `TF_MTP_BATCH=0` kill-switch, 10-27) | B STRONG DISTINCT-ish | OWN rule; category adjudicated at authoring against the mod.rs category table (the B307 letter-correction precedent — staged "A/B"; the load-ahead face reads A-family, the wrap-step face reads D/Dispatch); kinships (all hit-read, none owns the scalar ring-scan form): `cpasync-multistage-tile-pipeline` (gpu_09 — global→SMEM S-stage), `register-double-buffer-operand-pipeline` (gpu_09 — MMA-fragment lookahead), the B273 fetch/store-split rider (Metal loader pipeline), `cluster-dsmem-shared-head-one-row-decode-attention` attention_ahead (gpu_111 — tile copies-in-flight), `gather-signed-index-wrap-semantics` (gpu_72 — the NEGATIVE-ID wrap sense, distinct-from at authoring). The B256 `mtp-ring-rebase-covers-rollback-window` (G) owns the ring's REBASE floor, not its scan. Chronology recorded: rows-kernel `4f8135e8` (10-27) → stream-batch `2c10cfc1` → contract `1ff930d9` → load-ahead `6428050c` (10-28) |
| S5 | `row-owns-output-k-full-reduce-exchange-carries-no-arithmetic` — the DISTRIBUTED-PARTITION axis law: partition by OUTPUT rows (rank r owns 2,560 of the 5,120 output columns — 8,704 of 17,408 for the down projection; reduces the full K locally), never by K halves (every exchange would carry PARTIAL PRODUCTS: extra reduce rounds + cross-rank float error); `pair_gather` copies whole rows in bf16 "without doing any arithmetic, so no exchange rounds or sums a partial product across ranks" — the four kernels "carry no sums and no accumulator's error". TWIN-LAW (fault reification): `pair_argmax` reports a NaN/±Inf row "as NaN bits with bit 31 set in its id word", `pair_pick` turns the flag into the id sentinel — "both Macs then refuse a non-finite round where one Mac would"; corroborated wire-side by `fccb46ac` (the multirank probe vote is keepIfAll — "one rank's mismatch drops the wire on every rank"; the per-rank-only-vote negative fails the test). Five planted controls each go red on a named gate; "A broken link surfaces as bf16 NaN 0x7FC0" | TF `e06a6eef` (step 1 of 6, the two-Mac 27B series, 10-27) + `fccb46ac` (fabric/tp corroboration, 10-28) | B STRONG DISTINCT | OWN rule; category: nearest existing home at authoring (parallelism/collectives — the E-family per the mod.rs table), the "distributed" NEW-CATEGORY candidate RECORDED in-row (the router's 15-expert ontology does not grow in a batch — a separate owner-visible decision); kinships (hit-read): gpu_04 output-ownership (SINGLE-device work distribution), `attention-partition-two-knob-shared-partial-workspace` (gpu_51 — split-K partials ON one device), B119/B249 collective faces (comm-buffer shapes), gpu_30 `sentinel-readiness-over-separate-flag` (readiness sentinel — different sense), gpu_01 `clamp-tile-indices-to-sentinel-discard-at-write` (OOB aliasing — different sense). ZERO corpus owners of the rank-partition axis / keepIfAll / NaN-bits id-word reification. NotBenchable (2+ devices; the partition + reification contract is fixture-testable in logic) |
| S6 | `routed-density-pairs-per-item-keys-tile-admission` — a MoE routed-prompt kernel's tile admission keys on PAIRS-PER-ITEM (experts-per-token density), NOT item count alone: "wins or loses by the pairs a plan holds per item, which its items alone cannot tell"; low density → the K-parallel tile, full → the GEMM tile; landing row "takes plans of up to 507 items and 3.5 pairs an item, from served runs on Qwen3.6 and Nemotron" with the fill-fraction disclosure (x0.53–x1.00) and "every reply keeps its bytes". RIDER faces (956dff6a + fa45d4be): the admission bounds are COUPLED knobs — "Moving a routed prompt's items bound alone cannot both give Qwen3.6 the K-parallel tile and keep Nemotron's full plans off it" — the search moves the bounds TOGETHER, scored with the per-cell veto on each whole table; "the tuner times both sides of every place its proposal changes a choice, and retakes a noisy timing, so one noisy cell no longer vetoes a whole bucket by chance"; the fractional bound (threshold 3.43–3.75) is an f32 compared exactly in f64; "when kp3 refuses a plan, kp1 takes it first" (the admission ladder); flag-off byte-identical ("No table sets the field yet, so every choice stays the same") | TF `0c4c0613` (the bound field, 10-28) + `956dff6a` (the joint search, 10-28) + `fa45d4be` (fractional + both-sides + noisy-retake, 10-29) + `140ab0ff` (the landing row, 10-29) | B− DISTINCT | OWN rule, category D (Dispatch/admission — the B307 letter note); the coupled-search face adjudicated at authoring (compose in-row vs count-neutral face on the B284/B286 autotune family — lean compose-in-row: the search is the admission rule's own tuning half); kinships: `small-batch-matvec-gemm-bridge-rung` rows_min tables (gpu_13 — keys rows/type/device; the DENSITY axis NEW), `moe-tokens-per-expert-block-m-cap` (entries_gpu_133, B297 — caps block SIZE by routing load, not tile CHOICE), B284/B286 autotune family (bounded rounds + candidate priority, not coupled-bound joint search with per-cell veto). ZERO corpus hits on pairs-per-item/routed-prompt/fill-fraction/per-cell-veto. NotBenchable (gfx1201 HIP; the density-crossover shape is fixture-testable) |
| R5 | per-row RingRow TABLE face — "tf_q36m_attn_rows reads each row's ring, slots and segment from a table and repeats the original's arithmetic, so the contract holds it byte-equal to one tf_q36m_attn launch per segment on mixed rings, with exact fp32 sums, an FP64 check" — the streams-folded-into-row-axis family's TABLE parameterization | TF `4f8135e8` (10-27; predates the load-ahead; 6428050c then rewrote the rows variant onto fold_ring) | RIDER B− | count-neutral face on `verify-fold-gqa-repeats-into-row-axis-tile-fill` (B182, entries_gpu_52 — owner confirmed) beside its B199 streams-across-rounds instance; compose-never-merge |

## Tasks

- [x] T0 sources verified at the pins — fresh blobless clone `.raw/b317/tf` (ashhart/tensorfold),
      all ten short shas resolved (`6428050c 1ff930d9 2c10cfc1 e06a6eef fccb46ac 140ab0ff
      956dff6a fa45d4be 0c4c0613 4f8135e8`), every staged quote byte-verified against the
      commit bodies/diffs this session (S7: the 380-ns/slot sentence, the eight-slots helper,
      the wrap line, the `__ldg` comment, the next-group guard, the 0.24 ms / 103.2 tok/s
      reads, the contract body; S5: the whole-row-exchange body, the no-sums sentence, the
      NaN-bits id word, the five planted controls, the 0x7FC0 broken-link line, the keepIfAll
      body + its per-rank negative; S6: all four bodies including the fractional-bound +
      noisy-retake sentences the staged row understated; R5: the RingRow-table body verbatim)
- [x] T1 dedup at origin HEAD — S7: zero owners of the scalar ring-scan form (gpu_09 pair +
      gpu_111 confirmed the owned set; wrap hits ruled — gpu_72 owns the signed-id wrap
      sense; gpu_34/96/127 hits unrelated); S5: zero hits on rank-owns/keepIfAll/probe-vote,
      "output columns" hits incidental (gpu_71/gpu_122), NaN hits ruled (gpu_30/gpu_01
      different senses); S6: ZERO hits on all four vocabularies (gpu_13 rows_min + gpu_133
      moe-cap confirmed the kin set); R5: B182 gpu_52 owner confirmed (named first-member in
      the gpu_61/65/97/99 kinship notes)
- [ ] T2 authoring: `entries_gpu_142.rs` (3 own rules — pattern, bounded fix-space, category,
      rationale, source quotes, distinct-from kinships, grep evidence) + the R5 count-neutral
      rider face (gpu_52); category letters adjudicated against the mod.rs table; rustfmt-clean
      (rustfmt parse = valid Rust; cargo NOT run under Plan-337)
- [ ] T3 wiring: mod.rs mod-decl + extend + the slice rule_id lists; slice headers repaired to
      measured truth while touched (the B309 stale-header class)
- [ ] T4 count pins: tests.rs count-pin sites (1026 -> 1029 at the era-witness sites; the
      category table + the router expert-total if categories move) + AGENTS.md + README +
      .docs/11_domains/kernel_opt.md
- [ ] T5 GATES (deferred, never skipped): G1 batch317 needles+parse; G2 kernel_opt lib +
      default lib; G3 clippy -D ×4 postures (default / kernel_opt / no-default-features /
      all-features); retrieval floors HELD both postures. Native isolated CARGO_TARGET_DIR
      under the i1117 collection, or the amo git-bundle fallback (plan 315 precedent)
- [ ] T6 landing row + commit/push (the push is the arbiter) + `.raw/b317` removal + this
      status line flipped COMPLETE with the corpus delta
