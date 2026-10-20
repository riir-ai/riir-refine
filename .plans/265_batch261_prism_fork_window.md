# Plan 265 — Batch 261: the PrismML fork prism-b10787 window (3 own rules)

**Status:** IN_FLIGHT — claimed 2026-10-20 ~07:4x +07 (shikuwa/4090, riir-refine idle loop), claim row pushed `a38e4dd94`, expiry 2026-10-22 ~07:4x +07.

The standing-watch fork leg FIRED: the prism branch moved `1180918489` → `73bfe6cc4` (+4 commits) and **the tip carries a NEW SCORED TAG `prism-b10787-73bfe6c`** (the claim row's initial ls-remote grep `prism-b[0-9]+$` missed it — the tagging convention appends `-<sha>`; **CORRECTED here: the decode re-cut trigger FIRES**, the bout queues behind a verified-idle 4090 (StartLux resident to 13:30Z), recorded in the snapshot landing row). The kernel-opt mining lane converts the window: 3 own rules (A/D/B), zero riders (Slice B's kinship find: the decode-identity/tail/alignment atoms were already consumed by B180/B217 — disclosed in-rule as consumed components, the novel atom is the warp-topology re-shape).

## Verified source facts (read this session, pre-claim)

- `25f68a4` #330 (fattn-common.cuh:53-64,1019-1022,1044,1078 + fattn-mma-f16.cuh:2133,2148,2151-2152 + fattn.cu:583-587): `GGML_CUDA_FA_PREFILL_F16=N` — prefill-sized batches (Q->ne[1] > 8) over a quantized K/V cache of ≤ N cells convert K/V to f16 in POOL memory sized by the ACTUAL KV length and run the f16 MMA kernel; decode keeps the in-place quantized read; nothing reserved at load ("the reserved f16 copy would be n_ctx cells"); the alloc accounting treats pool-backed as "nothing to reserve beyond dst"; `kv_native_supported` REFUSES when the f16 route armed (disjoint by the query-count conjunct). Off by default.
- `ec1ee3a` #307 (fattn.cu:492-505): the quantized branch of the Ada kernel-choice ladder lacked the F16 branch's GQA>4 ratio rule (fattn.cu:495 `!(gqa_ratio > 4 && K->ne[1] >= 8192)`); fix lands `Q->ne[1] <= 2 && (gqa_ratio <= 4 || !ggml_cuda_fattn_mma_kv_native_supported(dst))` — parity + the IN-PLACE PRECONDITION ("Otherwise it converts the whole cache to f16 on every call, so keep the vector kernel"). Measured (Ternary Bonsai 2 27B PTQ1_0, GQA 6, q4_0 K/V, head 256, RTX 4070): 44.8→54.2 @32k, 25.3→41.2 @120k, mtp 53.3→84.0 / 23.2→68.5 (**computed range 1.21–2.95×** — the slice agent's honest correction of the task text's 2.71×). Measure-only switches removed in-PR.
- `483bdab` #368 (ggml-cuda.cu:1822-1921,1968-1978): PQ2_0 mat-vec 1-4 cols on Pascal (sm_61) — ONE warp keeps the generic mmvq's summation order (chunks lane + 32*w + 128*i, the ((v0+v1)+v2)+v3 fold + butterfly) for a whole row, R=4 rows per warp batched, NO smem, NO block barrier — "bit-identical to the generic mmvq there"; the decode-free PQ2 identity sum(q*(c-1)) = sum(q*c) − sum(q); clamped rows "computed but not stored"; 2-byte-aligned block loads; the arch-BAND admission (≥PASCAL <VOLTA, ne11 ≤ 4, exclusions).
- `73bfe6c` #321 server reasoning-effort flags — NULL watch row (Bonsai-2 template ops intel only).

## Tasks

- [x] T0 Claim row pushed (`a38e4dd94`).
- [x] T1 Source read at the pin: all 4 commits in full (ephemeral clone `.raw/b261/fork`); line numbers pinned; the scored-tag correction recorded (prism-b10787-73bfe6c EXISTS — the re-cut trigger FIRES, bout owner-gated behind the verified-idle 4090).
- [x] T2 Dedup greps: Slice A (`pool|prefill|f16 conv|dequant once|amortiz` → zero owners of the route choice; lossy-route B242 distinct-from) + (`gqa|parity|branch` → B103/B208 distinct-from, zero parity owners); Slice B (`bit-identical|butterfly|dp4a|sum(q` → B251 sibling distinct resource axis; **B180/B217 consume the identity/tail/alignment atoms — disclosed**); both in-rule with grep evidence.
- [x] T3 Slices authored (2 parallel subagents, disjoint files): Slice A `entries_gpu_113.rs` (2 rules, 4 fixtures / 12 tests 0 failures, committed `6a293263b`); Slice B `entries_gpu_114.rs` (1 rule, 2 fixtures / 10 tests 0 failures). All quotes verbatim at the pin (Slice B re-verified 25 fragments programmatically against the pin blob).
- [x] T4 Coordinator wiring: mod.rs (2 mod + 2 extends + A/B/D category comments 186→187/104→105/220→221), tests.rs (11 count pins 920→923 with the Batch-261 note prepended + 3 category tuples + the `batch261_rules_landed` needle gate), router.rs expert pin 920→923.
- [x] T5 Gates: batch261 needle gate FIRST-RUN PASS + kernel_opt lib 3190/0 (the gate caught 2 wiring misses mid-flight — 6 inline count pins + the KERNEL_CATEGORIES slices — repaired before landing) + default lib 2715/0 + clippy default/no-default clean + retrieval floors HELD 21/21 no re-pin (283.4 s, isolated target removed). Freeze/score-bench NOT owed (kernel_opt-only, P30/B250); binary rebuild NOT owed; ZERO-GPU throughout.
- [x] T6 Research note 297 + snapshot landing row (claim discharged + the trigger correction) + `.docs/11_domains/kernel_opt.md` Batch-261 row + AGENTS.md count pins 920→923 ×2.
- [x] T7 Commit `feat: batch 261: ...` + push; `.raw/b261/` + `/tmp` fixture scratch removed at close.

## Defers

- [-] T-defer the decode re-pin bout (prism-b10787 SCORED TAG, carries #369): the re-cut trigger FIRED per the 10-19 intake row's rule — the bout (fork-vs-ours decode/pp at the new pin tuple) is the riir-ai league lane, gated on a verified-idle 4090 (StartLux to 13:30Z today + exclusivity check); run at the earliest idle window, never through contention (Bench 649).
- [-] T-defer #321 server reasoning flags — NULL (server feature; Bonsai-2 template ops intel recorded in Research 297 only).
