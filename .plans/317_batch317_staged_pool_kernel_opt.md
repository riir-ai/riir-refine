# Plan 317 — Batch 317: the 10-30d staged leads kernel_opt pool (S7 + S5 + S6 + R5)

**Status:** LANDED 2026-10-30 ~07:1xZ (zed-m3-b317, M3; T0–T6 COMPLETE — 3 own rules + 1 count-neutral rider face; corpus 1026 → 1029; gates all green at the native 1.99.0 pin under the i1117 collection, isolated CARGO_TARGET_DIR; the batch-316 sibling landed first at `4d1fdb2e` clearing the shared-file conflict surface; push is the arbiter).

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
- [x] T2 authoring: `entries_gpu_142.rs` (3 own rules — pattern, bounded fix-space, category,
      rationale, source quotes, distinct-from kinships, grep evidence) + the R5 count-neutral
      rider face (gpu_52); category letters adjudicated against the mod.rs table; rustfmt-clean
      (rustfmt parse = valid Rust; cargo NOT run under Plan-337)
      **→ DRAFTED 2026-10-30 ~06:1xZ (this lane): the FULL T2 text exists at
      `.raw/b317/draft_entries_gpu_142.rs` (gitignored scratch — the shared-tree discipline:
      nothing dirty lands beside the batch-316 WIP). S7 → category A (memory/load pipelining,
      the register-held load-ahead + wrap advance); S5 → category E (parallelism/collectives,
      the "distributed" next-category candidate recorded in-rule per the 15-expert ontology
      law); S6 → category D (dispatch/admission) — all three checked against the
      `KernelCategory` table in kernel_opt/mod.rs (15 variants, the gpu_09 header law:
      membership lives ONLY in KERNEL_CATEGORIES). Fixtures: original minimal Rust
      reproductions with #[test] arms per rule (S7: bits-equality + wrap==modulo + the
      look-ahead guard; S5: no-arithmetic exchange + the NaN-bits reification + keepIfAll;
      S6: the density conjunction + the fractional-bound discrimination + the coupled-move
      + the noisy-retake veto). rustfmt parse-clean standalone (an in-tree rustfmt pass
      rides the landing — the standalone formatting oscillates on the vec![] macro, the
      parse never wobbles). The R5 rider TEXT is drafted in the session record (the
      per-row-table face on B182 gpu_52, compose-never-merge with the B199 instance);
      REMAINING T2 WORK: transcribe the draft into the tree + apply the rider to
      entries_gpu_52.rs at the window. THE R5 RIDER TEXT (append to the
      verify-fold-gqa-repeats-into-row-axis-tile-fill gpu_concept, after the Batch-199
      rider, before ".to_string()"): "RIDER (Batch 317 / Plan 317 — ashhart/tensorfold @
      4f8135e8, Apache-2.0 — the PER-ROW TABLE face, this family's third member): the
      Qwen3.6 MTP drafter launched tf_q36m_attn once per stream, one ring each, so a
      shared round ran its streams' attention in series; `tf_q36m_attn_rows` 'reads each
      row's ring, slots and segment from a table and repeats the original's arithmetic,
      so the contract holds it byte-equal to one tf_q36m_attn launch per segment on
      mixed rings, with exact fp32 sums, an FP64 check and nvcc 13.0 launch records.' —
      the fold-into-the-row-axis move now takes its parameters (ring pointer, slot
      count, segment) PER ROW from a table, one launch over mixed rings, and the
      legality stays the byte-equality per segment with exact fp32 sums, checked at
      fp64. Compose-never-merge: the fold-to-reuse-tiles decision stays the family
      trigger (rows too few → fold to rows; rows fine + a free batch axis → stride-0
      per gpu_65; streams-across-rounds per the B199 instance), the per-row table is
      the parameterization this rider owns — the rows kernel is the mechanism commit,
      its contract test (contract_qwen_mtp_rows.zig, 344 insertions) the fixture.
      Verified verbatim at the pin. Batch 317 rider."
- [x] T3 wiring: mod.rs mod-decl + extend + the slice rule_id lists; slice headers repaired to
      measured truth while touched (the B309 stale-header class)
- [x] T4 count pins: tests.rs count-pin sites (1026 -> 1029 at the era-witness sites; the
      category table + the router expert-total if categories move) + AGENTS.md + README +
      .docs/11_domains/kernel_opt.md
- [x] T5 GATES (deferred, never skipped): G1 batch317 needles+parse; G2 kernel_opt lib +
      default lib; G3 clippy -D ×4 postures (default / kernel_opt / no-default-features /
      all-features); retrieval floors HELD both postures. Native isolated CARGO_TARGET_DIR
      under the i1117 collection, or the amo git-bundle fallback (plan 315 precedent)
- [x] T6 landing row + commit/push (the push is the arbiter) + `.raw/b317` removal + this
      status line flipped COMPLETE with the corpus delta

## Landing record (2026-10-30 ~07:1xZ)

- **Landed:** 3 own rules (A +1, E +1, D +1) + the R5 count-neutral rider face on gpu_52;
  corpus 1026 → 1029. Category counts now A 202 / D 242 / E 96.
- **Fixture execution** (the batch-316 precedent, BEFORE entry): 11 tests green standalone
  (`/tmp/b317_fixtures`, rustc 1.99.0) — 10 planned + rule 3's ref_shape demonstrator. ONE
  fixture defect found + fixed en route: r2's `row_owned_round` multiplied by 0-indexed `o`
  while asserting 1-indexed values — the multiplier now `((o + 1) as f32)` so the expected
  vector `[8.0, 16.0]` stays a meaningful distinguisher. Assembly notes (disclosed, zero
  fixture edits): rule 3's flat ref+opt concat collides on `struct Plan` (E0428) — compiled
  as adjacent nested modules for execution; the corpus pipeline is parse-only per arm, so
  this is an execution-harness concern only. r1/r2 drop the opt half's mid-file
  `#![allow(dead_code)]` (illegal there; the ref head attribute covers the module).
- **Clippy healing:** the healer (compile-gated `--fix --write --verify`) fixed the
  doc-backticks class; the 6 `unnecessary_hashes` raw-string findings are the documented
  manual divergence class — repaired manually to the plain `r"..."` corpus convention
  (fixtures verified to contain no `"#` sequences; content byte-identical; rustfmt +
  the L2 parse gate green after).
- **S6 needle repair en route:** one batch-317 needle paraphrased its corpus text
  ("moving" vs the verbatim "that moves"); corrected to the verbatim substring — the
  needle class asserts verbatim marks, never paraphrase.
- **Sibling stale-pin repair en route (the B266/B271/B305 missed-pin class):** the
  kernel_opt-side cross-domain rust_perf witness (`batch249_slice3_rust_perf_riders`)
  still read 280 through the batch-316 landing's 280 → 285 — found + repaired by this
  batch's first kernel_opt suite run (the batch-316 provenance chained, miss disclosed
  in the pin message).
- **Router expert-total re-pin:** `rules_for_expert_total_matches_corpus` 1026 → 1029
  (the plan's "router expert-total if categories move" line — the total moved even though
  categories did not).
- **Gates (isolated `CARGO_TARGET_DIR=/tmp/b317_gate`, native 1.99.0, under the i1117
  collection per the batch-316 precedent):** G1 needles+parse 2/2; G2 kernel_opt lib
  531/0 (incl. the batch-310 needle regression) + default lib 2790/0; G3 clippy `-D
  warnings` ×4 postures (default / kernel_opt / no-default-features / all-features) all
  exit 0; retrieval floors 27/0 (latent_retrieval posture). freeze/score-bench NOT owed
  (kernel_opt-only, P30); binary rebuild NOT owed (kernel_opt outside the release set).
