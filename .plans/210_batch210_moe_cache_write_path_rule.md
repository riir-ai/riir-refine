# Plan 210 — Batch 210: the llama.cpp MoE expert GPU-cache write-path regime rule

**Status:** COMPLETE — landed this session; the single 10-08 late-envelope lead (llama.cpp `d6cf9acb` #29887, merged 2026-10-07T18:07Z, ~1.5 h after Batch 209's morning poll pin `18b5f8b1`) converted as ONE new kernel_opt rule (category D: dispatch — the B94 expert-cache family's home). Gates all green; commit hash on the snapshot row.

## Lead record

- **Source:** `ggml-org/llama.cpp` @ `d6cf9acb25e6c657d6c6a4645ab7b69fce119995` (#29887, MIT) — "llama : add a GPU cache for MoE experts kept in host memory" (Aman Gupta, Assisted-by: Claude). 14 files, +761/−18; new `src/llama-moe-cache.{cpp,h}` (550 lines). **Provenance:** full `.patch` read at the pin via `https://github.com/ggml-org/llama.cpp/commit/d6cf9acb….patch` (this session's fetch — the .patch at the commit sha is the verification source, equivalent to the `.raw` clone diff convention; the ephemeral-clone law satisfied by the patch-at-pin read, no clone created).
- **The other new commit** `50a6c5cf` (mtmd cohere2 vision #30062) — model plumbing, off-lane, watch-only.
- **Envelope re-poll result:** cubecl newest `2f0d8e8df5` UNCHANGED; bevy newest `5cd69c1357` UNCHANGED; wgpu not re-polled this pass (feed 404 at B209, watch-only).

## The rule — `expert-cache-write-path-regime-gated` (D +1)

The one law no existing rule owns: **an expert cache shared by narrow and wide regimes must gate its WRITE path on the regime — plan-and-evict only for batches whose working set fits the slots; wider batches read the cache if-present but never touch the LRU, so a prefill-sized plan cannot evict the generation working set.**

Faces (all clause-level, one rule):
1. **Working-set-fit admission** — the plan refuses when the call's distinct experts exceed slots (`uniq.size() > n_slots` → refuse, fail loud); the graph-side arm declines the cache when `min(n_tokens * n_expert_used, n_expert) > n_slots`.
2. **The regime split** (`max_batch = 32`): small batches plan+write (touch/evict/upload); large batches only READ (`copy_experts` copies if-present, returns 0 otherwise) — verbatim: "large batches only read the cache, so the experts used in generation stay in it".
3. **The scheduler-seam update point** — the slot map is a HOST weight, so the scheduler's own split boundary (forced by the host-weight read) is where the copy callback runs the plan — verbatim: "the slot map is a host weight, so the scheduler starts a new split here and copies it with the copy callback" — no NEW synchronization point is created.
4. **Hit-before-evict ordering** — "hits go to the tail first, so the head can be evicted below" (the hit-touch pass precedes the miss-eviction pass within one plan).
5. **Consecutive-run merge** — sorted misses merged into single async copies ("sorted misses usually get consecutive slots, so the uploads can be merged"; `slots[e + n] == slots[e] + n` run detection on the read side).
6. **MMQ overread slot** — the bank allocates `n_slots + 1` ("one extra slot at the end, CUDA MMQ can read past the last expert").
7. **Proportional budget split** — "split the budget by the size of the experts, so each group caches the same fraction of its experts".
8. **Correctness pin** — forced-eviction NMSE test vs CPU (4 slots, 1536 KiB budget, n_ubatch=2, verbatim: "the cache has only a few slots (4 for 288 KiB experts), so the experts are evicted and uploaded again").

NotBenchable (no in-source perf numbers; the correctness NMSE test + teardown hit-rate logs are the evidence).

## Dedup (first-read, this session; the named-vocabulary greps)

- `lru|slot.?map|expert.?cache|moe.?cache|bank` + `evict|working.?set|resident` over `src/domains/kernel_opt/corpus/` — the B94 family owns: victim/eviction POLICY (`decayed-frequency-eviction-recency-tiebreak-route-protection`), one-shot REGION isolation (`scratch-bank-isolates-one-shot-operands-from-hot-cache`), regime-START admission (`fill-gate-speculation-until-working-set-resident`); `host-cache-immutable-device-weights` (B26) is the inverse direction. **The write-path regime gate is owned by NOBODY.** Full grep + distinct-from capture at authoring.

## Tasks

- [x] T1 — envelope re-poll + grade (this plan's lead record; the grading disclosure rides the research note)
- [x] T2 — dedup greps (first-read above; full both-vocabulary grep at authoring, recorded in-rule)
- [x] T3 — author the entry (entries_gpu_79.rs new slice, 1 rule) + mod.rs wiring (mod + extend + D category array + D header comment)
- [x] T4 — batch210 gate (parse both sides + needle markers + the distinct-from trio) in tests.rs
- [x] T5 — count pins: tests.rs ×4 sites (total 776→777 ×2 + Dispatch 177→178 + sum/grouped), router.rs :472, AGENTS.md ×2, README ×2
- [x] T6 — gates: kernel_opt+latent_retrieval lib (full, ~20 min) · default lib · clippy ×3 postures · retrieval floors 21/21
- [x] T7 — Research note 245 + `.research/.highwater` 244→245 + `.plans/.highwater` 209→210 (dual_allocation_gate at both)
- [x] T8 — snapshot row (.distill/001) + commit + push
