# Plan 406: Batch 407 nominal — the vLLM GDN decode-fusion family distill (Research 441)

**Status:** STAGED-UNCLAIMED — staged behind the live m3 leases (Plan 404 ARTEX exp 2026-11-25T18:00Z, Plan 405 coreutils exp 2026-11-25T07:05Z); the first free mining lane claims this per the queue order. Verdict: Research 441, YES B− corpus, face-heavy.
**Claimed:** —

## Tasks

- [ ] **T0 — re-clone + re-pin + widen:** `.raw/vllm-gdn` blobless clone at `78f20a5a3`/`34c653886`/`92b52792d` (re-pin if the batch runs late; fast-moving repo); byte-verify every Research 441 quote at the pins; WIDEN the read — the rest of `vllm/model_executor/layers/mamba/gdn/` + `csrc/gdn/` is UNREAD (the verdict read the three-commit window only); re-grade any widened finds before authoring.
- [ ] **T1 — dedup re-verification at landing HEAD:** the Research 441 §3 table re-grepped (3+ vocabularies, both corpora); the landed-batches delta check (Plans 404/405 land between verdict and batch — re-run at landing HEAD).
- [ ] **T2 — authoring:**
  - [ ] R1 `output-buffer-prezero-when-kernel-writes-all-rows` (rust_perf allocation, the GOAT lead — benchable; the NaN/poison canary is the fixture's falsification arm; the "every row written before any read" regime + the partial-write anti-regime IN-rule; adjudicate vs `vec-macro-zero-fill` (inverse polarity) + `hot-slot-recycler-flat-tables-zero-alloc-claims` + `raw-bytes-with-hit-backfill-once` in-rule both directions).
  - [ ] R2 face-or-rule: packed-16-bit-state store + f32 compute — fold decision pre-registered: face on gpu_51 `dtype-native-source-read-with-shared-admission-predicate` (state-side polarity) unless the uint4-pair packing axis carries bounded decision content of its own.
  - [ ] F1 face on `whole-step-gdn-decode-window-one-launch` (entries_gpu_126) — the CUDA MTP 9-token + ROCm AITER instances (never land; record beside the owner).
  - [ ] F2 face on `gdn-state-snapshot-per-position-verify-early-exit` — the slot-indexed per-token store instance.
  - [ ] F3 face on `gqa-zero-stride-batch-broadcast` — the weight-side stride-0 expand instance.
  - [ ] F4 face on `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` / `alignment-gated-zero-copy-view` — the contiguous-drop instance.
  - [ ] F5 face on B404 `dependency-version-known-wrong-window-refusal` — the inspect-signature variant.
  - [ ] F6 face-or-thin-rule: the 12-per-8 padded smem column map (`qk_column`) — fold decision vs `xor-swizzle-smem-static-index-tables` at authoring.
  - [ ] DROPS recorded: reduce_scatter butterfly (thin), zero-bias padding (plumbing), capability-envelope refusal as a RULE (well-owned; face only), `_encode_layer_name` (plumbing).
- [ ] **T3 — fixtures:** standalone-compiled AND executed BEFORE entry (debug AND -O); R1's four-arm discipline (buggy/fixed/all-safe/tamper — the tamper arm plants a partial-write read to prove the regime condition is load-bearing).
- [ ] **T4 — gates:** clippy -D ×postures + lib + fmt at an isolated `CARGO_TARGET_DIR`; retrieval floors HELD expected (rust_perf +1 → re-derive per the standing conventions at landing HEAD).
- [ ] **T5 — pins_manifest --write** WITH the corpus change, same commit; score-bench per the P30 law (rust_perf delta → RUN); freeze tables NOT owed unless clippy_lints moves.
- [ ] **T6 — count web one-landing** at landing HEAD: AGENTS head + README + `.docs/11_domains/rust_perf.md` + health_views + router witness; kernel count UNCHANGED unless R2 lands as a rule (re-derive, never hand-type).
- [ ] **T7 — records:** Research 441 Status → LANDED; snapshot LANDING row; dual_allocation_gate; `feat: batch 407` push; `rm -rf .raw/vllm-gdn` at close.
- [ ] **T8 — lane-intel disposition:** the 16-bit-state lever recorded in the snapshot row as LOSSY-GATED lane intel (never a bit-identity arm; the lossy law's per-family retention gate governs any future decode-state-traffic cell).

## Bars (pre-registered)

- R1 GOAT: G1 exact (overwritten values bit-identical; pre-write contents never read), G2 Pareto + alloc-count (alloc+memset vs alloc-only), G3 no regression, G4 alloc-free scorer. A Tie/Slower demotes in-landing (the ungoat law).
- Faces: count-neutral, cite + never merge, in-rule adjudication both directions at authoring.
- Yield: 1 rule + 4-6 faces (the verdict's estimate; above that needs extraordinary evidence).
