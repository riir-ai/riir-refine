# Plan 406: Batch 407 nominal — the vLLM GDN decode-fusion family distill (Research 441)

**Status:** LANDED 2026-11-24 (Batch 407) — rust_perf +1 (entries_97), kernel_opt +8 count-neutral faces, one commit. **R1 DEMOTED at landing: NotBenchable-by-platform** (the pre-registered bar fired: validator ParetoDominated 0 faster / 3 slower / 0 tie + the isolated probe 0.967/0.979 twice — `vec![0.0; n]` rides `alloc_zeroed`, whose zero-page path makes the deleted memset pass already absent on this platform; the TORCH win stays decision content). Plans 404/405 had NOT landed at the landing HEAD — no delta, the corpus-coordination note discharged.
**Lease closed:** zed-e7e471a3@amo, 1795473279, expires 2026-11-25T22:34:00+00:00 — closed at the landing (2026-11-24); the earlier staged-unclaimed verdict landed without a lease (the placeholder was malformed and blinded the repo-wide lease check; reshaped to a closed row by the 2026-11-24 housekeeping sweep, zed-56f3ce1e@amo).

## Tasks

- [x] **T0 — re-clone + re-pin + widen:** `.raw/vllm-gdn` blobless clone at `78f20a5a3`/`34c653886`/`92b52792d` (re-pin if the batch runs late; fast-moving repo); byte-verify every Research 441 quote at the pins; WIDEN the read — the rest of `vllm/model_executor/layers/mamba/gdn/` + `csrc/gdn/` is UNREAD (the verdict read the three-commit window only); re-grade any widened finds before authoring. DONE: all three pins resolved, every quote byte-verified; widen read the full 16-bit kernel (450 lines, end-to-end), kimi/olmo GDN variants, base.py, fused_recurrent.py, layernorm_guard.py, fused_norm_gate.py, fused_gdn_prefill_post_conv.py, gdn_attn metadata, l2norm/chunk/cumsum — no new rule-shaped atoms beyond the verdict's set (olmo's `torch.zeros` is the real second instance of R1's defect class, recorded as corroboration).
- [x] **T1 — dedup re-verification at landing HEAD:** the Research 441 §3 table re-grepped (3+ vocabularies, both corpora); the landed-batches delta check (Plans 404/405 land between verdict and batch — re-run at landing HEAD). DONE: R1 dedup clean at landing HEAD (nearest owners read in full: `vec-macro-zero-fill` inverse polarity, `scoped-init-at-consume-site` device cousin, entries_05 matvec = the accumulation anti-regime); Plans 404/405 NOT landed (no delta).
- [x] **T2 — authoring:**
  - [x] R1 `output-buffer-prezero-when-kernel-writes-all-rows` (rust_perf allocation — **DEMOTED AT LANDING to NotBenchable-by-platform**, the falsifier fired; the NaN/poison canary stays the fixture's falsification arm; the "every row written before any read" regime + the partial-write anti-regime IN-rule; adjudicated vs `vec-macro-zero-fill` (inverse polarity) + `hot-slot-recycler-flat-tables-zero-alloc-claims` + `raw-bytes-with-hit-backfill-once` in-rule both directions).
  - [x] R2 face: packed-16-bit-state store + f32 compute — folded on gpu_51 `dtype-native-source-read-with-shared-admission-predicate` (state-side polarity; the storage dtype IS the checkpoint's own — not the lossy class; the lossy-gated lane-intel line rides T8).
  - [x] F1 face on `whole-step-gdn-decode-window-one-launch` (entries_gpu_126) — the CUDA MTP 9-token + ROCm AITER instances (the third+fourth stack instances, never land).
  - [x] F2 face on `gdn-state-snapshot-per-position-verify-early-exit` — the slot-indexed per-token store instance (entries_gpu_152).
  - [x] F3 face on `gqa-zero-stride-batch-broadcast` — the weight-side stride-0 expand instance (entries_gpu_65).
  - [x] F4 face on `alignment-gated-zero-copy-view` — the contiguous-drop instance (popcorn).
  - [x] F5 face on B404 `dependency-version-known-wrong-window-refusal` — the inspect-signature variant (entries_gpu_164).
  - [x] F6 face on `xor-swizzle-smem-static-index-tables` — the integer-pad polarity face, qk_column 12-per-8 (entries_gpu_09; count-neutral, folded at authoring).
  - [x] F7 face on `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` — the consumer-constraint conjunct (entries_gpu_78).
  - [x] DROPS recorded: reduce_scatter butterfly (thin), zero-bias padding (plumbing), capability-envelope refusal as a RULE (well-owned; face only), `_encode_layer_name` (plumbing).
- [x] **T3 — fixtures:** standalone-compiled AND executed BEFORE entry (debug AND -O); R1's four-arm discipline (parity / poison-canary / anti-regime-accumulation / padding-boundary + the tamper partial-write arm — 5/5, zero warnings both postures).
- [x] **T4 — gates:** clippy -D ×postures (default + kernel_opt,rust_perf,rust_perf_bench,latent_retrieval) + lib (4144 passed / 0 failed at /tmp/plan407) + fmt bare rustfmt on the touched files; retrieval floors HELD (no re-pin needed — the corpus grew +1, floors held 29/0).
- [x] **T5 — pins_manifest --write** WITH the corpus change, same commit (rust_perf 393 rules; benchable 143→142, notbenchable 250→251 at the demotion; manifest + prose agree). score-bench NOT owed per the standing convention (Batch 401's landing recorded the same rationale: "Freeze/score-bench NOT owed (zero clippy_lints delta)" — score-bench prices the clippy-lane heal surface, which this batch does not touch); freeze tables likewise NOT owed (clippy_lints unmoved).
- [x] **T6 — count web one-landing** at landing HEAD: AGENTS head + README + `.docs/11_domains/rust_perf.md` + health_views (`perf 393` + total 1780) + the kernel-side rust_perf witness (5 hand-typed count pins + 3 annotated-count pins → 341 + serde pin), all at 393 with the demotion wording.
- [x] **T7 — records:** Research 441 Status → LANDED (with the demotion disclosed); snapshot LANDING row; dual_allocation_gate; `feat: batch 407` push; `rm -rf .raw/vllm-gdn` at close.
- [x] **T8 — lane-intel disposition:** the 16-bit-state lever recorded in the snapshot row as LOSSY-GATED lane intel (never a bit-identity arm; the lossy law's per-family retention gate governs any future decode-state-traffic cell).

## Bars (pre-registered)

- R1 GOAT: G1 exact (overwritten values bit-identical; pre-write contents never read), G2 Pareto + alloc-count (alloc+memset vs alloc-only), G3 no regression, G4 alloc-free scorer. A Tie/Slower demotes in-landing (the ungoat law).
- Faces: count-neutral, cite + never merge, in-rule adjudication both directions at authoring.
- Yield: 1 rule + 4-6 faces (the verdict's estimate; above that needs extraordinary evidence).
