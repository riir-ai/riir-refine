# Research 120 — Layr-Labs/qwen-3.8-mtp-challenge distill verdict

**Status:** RECORD — Batch 100 landed 2026-09-02 @ the same pin `0863b06a` (8 rules + 1 B22 rider, corpus 445 -> 453; mine-time verification appendix at the end)

## Source

- **Repo:** https://github.com/Layr-Labs/qwen-3.8-mtp-challenge
- **Inspected:** 2026-09-02 (full clone at `.raw/`, deleted after this note; **pin `@ 0863b06ac16e26e48fc06e97444095b00feb66d4`** — verified via `rev-parse` on the clone; main HEAD commit dated 2026-08-23)
- **License:** MIT (Copyright (c) 2026 Layr Labs, Inc.); vendored `Vendor/mlx-swift-lm` retains upstream MLX MIT + per-file attributions; `pagedattention.metal` carries an explicit provenance header (vLLM paged_attention_v2 Apache-2.0, mistral.rs Metal port MIT, vllm-metal Apache-2.0, Apple MLX sdpa_vector_2pass MIT) and states "no lines are copied verbatim" (a re-derivation for the MLXFast metalKernel convention).
- **Re-visit trigger (user):** "more perf, bumped more % faster since the last distill" — **measured: the repo tree is byte-identical to the previous session's snapshot pin** (origin/main == `0863b06a`; zero commits between). The perf bumps live on the challenge **leaderboard** (auth-gated Yukon API, `yukon-api.fly.dev`) — participant submission scores, not mineable code. The re-visit's real yield is that THIS session completed the deep mechanism reads the previous session left pending.

## What it is

Apple-Silicon inference benchmark arena, **Swift-only**, ranked runner M5 Max 128 GB. Ranked track `qwen3.8-27b-mtp-v1` (operator-ratified 2026-08-14): Qwen3.8-27B 4-bit MLX backbone + bf16 MTP head, self-draft/self-verify MTP. Score = `dflash_decode_speedup` vs a per-prompt no-op reference, floor 0.95, ceiling 3.0. **Same team as antirez/ds4 (Batches 97/98, DwarfStar)** — `DARKBLOOM_*` env prefix, `ds4.darkbloom.ai` checkpoint host; split from `Layr-Labs/mlxfast-challenge-dev@ba5f9703`. Backbone geometry (64 layers, 4-layer hybrid repeat, vocab 248320, hidden 5120, hd 256) = the same model family as riir-ai Issue 742's qwen38 stack.

~90% of the tree is vendored Apple MLX upstream (`Vendor/mlx-swift`, out of scope — its CUDA backend is unused by the ranked Metal track). Mineable surface = fork/challenge-authored files only: `Sources/MLXFastModel/*` (notably `Qwen36MTPBlockSession.swift`, `Qwen35GatedDelta.swift`, `RuntimeStartupMemoryPolicy.swift`), `Sources/MLXFastHarness|MLXFastTrustedHarness/*`, `Vendor/mlx-swift-lm` fork deltas (`MLXSpeculative/DFlash*`, `MLXLMCommon/ContinuousBatchingV2/*` incl. `Paged/pagedattention.metal`, `Compiled/CompiledDecodeV2.swift`, `PrefixCacheV2.swift`), and the fork-authored `Vendor/mlx-swift-lm/docs/engine-v2/kernel-research.md`.

**Files read this session:** `kernel-research.md` (full), `pagedattention.metal` (header + structure), `Qwen35GatedDelta.swift` (full), `RuntimeStartupMemoryPolicy.swift` (full), `Qwen36MTPBlockSession.swift` (structure + residency/warm sections), `DFlashGreedyRound.swift` (header), `CompiledDecodeV2.swift` + `PrefixCacheV2.swift` (headers). Unverified at this pin: `QwenRuntimeMTPDriver.swift`, `DFlashBatchedEngine.swift`, `SchedulerV2.swift`, `pagedattention.metal` pass-B body — re-read at mine time.

## Dedup-gate evidence (both vocabularies, `src/domains/kernel_opt/corpus/`)

- **donation / slice-update / copy-on-write / functional update** → **ZERO hits** (kernel_opt). Closest neighbors all distinct: B37 `kv-window-device-resident-ring-append` (transfer axis), B63 `reset-state-in-place-fill-never-realloc` (reset), B39 in-place weight slots (training weights), B21 pool-buffer aliasing (aliasing hazard). The donation-eligibility mechanism is unclaimed.
- **fence** → hits are CUDA device fences (`fence.acq_rel` in B92's semaphore rule; the compiler-fence in B97's bit-exact fusion) — NOT the MLX "fence output array chained through a group as hazard-tracking substitute" contract. Novel.
- **residency** → hits are register/operand residency (B29 epilogue, B34 dual-layout). **Metal resource residency-set wiring: zero hits.**
- **head split / register-budget accumulator** → B44 splits the reduction axis; B07 splits plane ROLES; B33 owns output threads. The "split query heads across threadgroups when `heads·D/32` exceeds the register budget, trading KV re-reads" formula is unclaimed.
- **command-buffer byte/op budgets** (`MB_PER_BUFFER` / `max_ops`) → zero hits.
- Spec-decode surface → covered by B71 (`rotated-ring-undo-via-rebind-reference-snapshot`), B81 (`virtual-tail-packed-tree-verify-over-paged-kv`, `staged-append-rollback-lifo-retry-blocks`), B84 (snapshot coverage contract) — confirmed by reading `DFlashGreedyRound.swift` (evidence-ledger design, not new perf mechanism).

## Ranked candidate table

| # | Candidate rule | Source | Grade | Category | Notes |
|---|---|---|---|---|---|
| 1 | **`functional-cache-versioning-loses-donation-under-kernel-read`** — a large shared device buffer versioned through functional/slice updates (copy-on-write) loses the runtime's buffer-reuse/donation eligibility when ANY custom-kernel dispatch reads it in the same or in-flight submission window (the runtime pins input `Data` handles until command-buffer completion) → every subsequent update silently falls back to a FULL-BUFFER copy. Two-regime: pure write chains donate fine (measured linear, zero transient, even chained 24 deep); write-chain interleaved with kernel reads degrades to `O(slab)·layers` copies per token. Fix = stable buffer written IN PLACE by a kernel (the vLLM `reshape_and_cache` shape); "any fix keeping per-layer slice_update on multi-GiB shared slabs is dead on arrival." | kernel-research.md §3 (measured: 25,470 → 13.6 ms/step, ≈370 GiB/token of copies at 23×{K,V}×8 GiB — 24 layers total, first read pins, subsequent 23 copy) + pagedattention.metal header | **A− (GOAT)** | A | Measured 3 orders of magnitude; mechanism-level arithmetic; deterministic; measured-GOAT distillation shape (B49/B52/B54 precedent — the fix landed, `bc358e2`, with ablation tables) |
| 2 | **`fused-kv-write-single-writer-reads-newest-from-input`** — a KV write fused into the consuming attention kernel is race-free WITHOUT cross-threadgroup primitives iff (a) exactly ONE threadgroup per byte is the writer (hgroup-0 of the row's last partition) and (b) NO threadgroup in the same dispatch reads that slot from the slab — the newest position is always loaded from the `knew`/`vnew` input operands (bit-identical bytes). Ring windows cannot alias: the overwritten slot held a position ≥ ring·S older than the query, outside every attendable window. | pagedattention.metal HAS_WRITE arm | **B+** | A/D | Tight bounded contract; composes with #1 as the decode arm of the in-place contract |
| 3 | **`in-place-write-through-input-emits-fence-consumer`** — when a framework's encoders are concurrent with hazard tracking at DECLARED-buffer granularity, an in-place kernel write through an "input" declaration is INVISIBLE to the tracker: every subsequent reader of those bytes must consume an explicit fence token (fence = prev + 1, chained through the group; gathers consume the latest fence) to get both scheduling order and the memory barrier. Applies to bulk writes (prefill/prefix adoption); decode rides graph edges instead (step N+1 consumes step N's outputs). | pagedattention.metal bulk-write arm + header safety argument | **B** | D | The fence-token-as-ordering-currency is distinct from all CUDA-fence corpus content |
| 4 | **`residency-set-wired-once-post-warm-persistent-footprint`** — a Metal residency-set capacity is ZERO until wired: without a one-time post-warm `setResidencyLimit`, the driver re-establishes residency for the whole weight tower on later command buffers. Wire ONCE after the input-independent warm, capacity = live post-warm persistent footprint + small page-rounding allowance only — later scratch FAILS the fit test and stays on the commit-free unwired path; never end/shrink the ticket (shrinking evicts resident weights). | Qwen36MTPBlockSession.swift L202-218 | **B** | D | Bounded, mechanism-level; Metal-specific (corpus has Metal precedent) |
| 5 | **`register-budget-head-split-gqa-reread-trade`** — when per-thread accumulator floats (`heads_per_tg · D / 32`) exceed the ISA register budget (128), split a kv head's GQA query heads across `GQA/HPT` threadgroups (HPT chosen so `HPT·D/32 ≤ 32`; d512→HPT=2, d256→HPT=4, d64/d128→HPT=GQA unchanged) — trading KV re-reads (vLLM RFC measured ~30% loss at g=8) for static eligibility, with the smem formula (`(G·D + NSG·G·(D+2))·4`) modeled exactly in host eligibility code and pinned by tests. | pagedattention.metal HPT template + kernel-research.md §4 | **B** | E | Formula-bounded two-regime (full-GQA when D small; split when eligibility, not optimality, gates) |
| 6 | Command-buffer referenced-byte commit threshold — `MLX_MAX_MB_PER_BUFFER` 50 MiB default < individual 4-bit projection payloads → one commit per weight-sized op; raise to 512 MiB to group adjacent kernels, explicit async-eval groups stay the outer boundary; low-memory profile halves it to bound transient in-flight memory. | RuntimeStartupMemoryPolicy.swift | **C+** | D | Config-knob-shaped (env var), single framework — mine only if the batch has room; the "memory-gated feature-disable silently skips ranked code paths" note in the same file is a process lesson (our gate-coverage class), not a rule |

**Drop rows:**
- `Qwen35GatedDelta.swift` — the recurrence **deliberately delegates** to upstream `MLXLLM.gatedDeltaUpdate` ("intentionally the recurrence source of truth"), i.e. a positive instance of Batch 95's mlx-lm `gated_delta.py` distillation. Zero novel content. KILL.
- DFlash/MTP spec-decode orchestration (draft/verify/rollback/paged-KV/tree state) — covered by B71/B81/B84. KILL (verified, not assumed).
- MTP session micro-patterns — positive instances: depth-1-serial-through-the-SAME-forward (B30 `zero-init-side-branch-frozen-base-path`), next-round primary argmax from the SAME batched readout (B25 `device-side-argmax-minimal-download` / B26 `batched-downloads-single-sync-cycle`), lazy hidden-state retained for the next graph (B27/B36 deferred-download family), warm-all-legal-widths at startup (ambient warmup discipline). KILL (riders at best).
- DFlash evidence-ledger / verifier-pricing design (top-2 work binders, price-the-rejected-tail, hiddenDigest SHA-256 self-consistency-only because accumulation-order frame divergence makes cross-build bit-identity impossible) — benchmark doctrine, not perf; the digest caveat is a nice EXTERNAL CONFIRMATION of B95's `reduction-restructure-needs-source-owned-comparator`. Rider candidate.
- Window-as-start-offset-arithmetic (never mask arrays) — standard FA2 idiom via ring page tables; low decision content beyond the page-table design. KILL.
- Sinks-fold-into-denominator-at-merge (vLLM `s_aux` / SGLang `deno`) — GPT-OSS-specific, C−. KILL unless the batch wants a micro-rule.
- `simd_sum(f32)` ~14.5 cycles vs shuffle ~2 / quad 2× cheaper (§1.1) — **B22 rider candidate** (`warp-shuffle-intrinsics-for-sub-warp-reduction` gains a Metal measured-cost note).
- Memory-gated feature-disable eval-parity lesson — process lesson (our gate-coverage class). Not a rule.

## GOAT candidate test (#1)

1. **Benchable** — measured-GOAT distillation shape (B49/B52/B54/B55 precedent): the fix landed (`bc358e2` in-place writes, `5087663` head split) with same-harness ablation tables (§3.2 profiler sweep + §5 post-fix: 236.4→11.4 ms/step at 24 layers/2 GiB; 25,470→13.6 ms/step GPT-OSS B=1; 1,999→14.3 B=4; real-weights B≥4 paged BEATS contiguous). kernel_opt validation = syn-parse standing gates + these cited measurements.
2. **Exact semantics** — orchestration rule; no bit-identity surface (the in-place contract's bit-identity argument — "newest position loaded from inputs, bit-identical bytes" — is quoted as decision content, not gated numerically here).
3. **Magnitude story** — 370 GiB of GPU copies per decoded token ≈ 1.9 s/token at copy bandwidth; measured 3 orders of magnitude. Mechanism arithmetic, not "probably faster".
4. **Deterministic** — yes; ablations same-harness, same-run comparisons only (the doc itself flags cross-report contention caution).

## Yield estimate

6 candidates in, ~50% coordinator falsification → **3–4 rules land** (R1 near-certain; R2/R3 strong as one in-place-contract family; R4/R5 each carry falsification risk — R4 could be judged Metal-API-boilerplate, R5 could dedup against B44/B07 on coordinator re-read). R6 + the B22 rider are opportunistic.

## Honest caveats

- **The tree did NOT move since the previous session's pin** — the "perf bump" is leaderboard-side (auth-gated). Anyone mining should re-pin at mine time anyway (beta-quality fast-moving repo).
- **Swift/Metal substrate** — rules land as substrate-agnostic pseudocode (oMLX=Python and ds4=Metal precedents). The MLX donation mechanism is framework-specific; R1's generalization ("copy-on-write versioning of hot device state + a reader that pins the source in-flight → per-version full copies") must be written carefully or the coordinator may falsify it as MLX-only ambient.
- **Unread at verdict time — NOW READ at mine time** (Lane B above): `QwenRuntimeMTPDriver.swift`, `DFlashBatchedEngine.swift` (honest zero), `SchedulerV2.swift`, `pagedattention.metal` pass-B body, `CompiledDecodeV2.swift` body. Findings folded into the appendix.
- **Challenge-mechanics noise**: `editablePaths` access-level lesson, scoring contract, anti-cheat ledgers — explicitly out of corpus scope.
- League overlap (separate lane): this repo IS the M-series opponent lane for qwen3.8-27B — route to perf-rematch opponent-watch, not the corpus.

## Queue slot

**Batch 100 candidate** — plan `.plans/068_qwen38_mtp_challenge_mine.md` (created with this note, BEFORE any corpus edit). Lead with `functional-cache-versioning-loses-donation-under-kernel-read`.

---

## Mine-time verification appendix (Batch 100 landing, 2026-09-02)

Re-cloned at mine time: pin `0863b06ac16e26e48fc06e97444095b00feb66d4` — **byte-identical to the verdict pin** (tree static since 2026-08-23). Two parallel verification lanes (the Protocol-A step-3 discipline):

**Lane A** (`pagedattention.metal` 500 lines IN FULL + `kernel-research.md` sections 1-6):
- R1 CONFIRMED with one numeric correction: the ~370 GiB arithmetic is **23 layers** x {K,V} x 8 GiB (24 layers total — layer 1's read pins, the subsequent 23 `slice_update`s copy); the table cell above is corrected in place. Two riders folded into R1's rule text: the memory-limit-throttle serialization (25,470 ms vs the 1.9 s pure-copy arithmetic, ~13x superlinear — `get_active_memory() > get_memory_limit()` -> serialize) and the >=2-layers-sharing fixture corollary (per-layer tests + small-pool parity suites stay green by construction).
- R2 CONFIRMED all three halves, code-anchored: the writer guard is `hgroup == 0 && part == (attend_len - 1) / PTOK` (L261); the input-path read fires for ALL hgroups of the last partition, not just the writer's (L299-315); the ring-alias impossibility quote is header L105-107.
- R3 CONFIRMED with one sharpening: the fence obligation is scoped to readers of the SAME bytes, "directly or transitively" (kernel-research.md L244-253). Bonus consumer-side evidence: `PagedAttentionKernel.swift` L285-290 (the decode kernel literally takes `wfence` as an input — the graph edge IS the fence consumption) + `PagedKVBackend.swift` L141-144.
- R5 CONFIRMED with three precisions: threadgroups are per (kv head, PARTITION) (`SPLITS = GQA / HPT`, metal L220-221); the smem formula lives in BOTH the doc and host Swift (`partThreadgroupBytes`, L234-241); the ~30% re-read loss is **vLLM RFC #15351** at g=8; the test file EXISTS and pins the HPT table (`Tests/MLXLMTests/CBv2PagedEligibilityTests.swift` — the challenge repo's own `Tests/` find was misleading; it is in the vendored tree's test target).
- B22 rider CONFIRMED byte-accurate (§1.1 L57-60): `simd_sum(f32)` ~14.5 cycles throughput / ~28 latency vs ~2 for a shuffle; "Quad reductions are ~2x cheaper; a shuffle tree can beat the builtin".
- Header provenance quoted (4 upstreams: vLLM Apache-2.0, mistral.rs MIT, vllm-metal Apache-2.0, Apple MLX; "no lines are copied verbatim").

**Lane B** (the 5 files the verdict left unread):
- `DFlashBatchedEngine.swift`: **honest zero** — ambient serving plumbing (single queue, idle-sleep, detokenizer lifecycle; vLLM-cited bookkeeping).
- `QwenRuntimeMTPDriver.swift`: challenge-mechanics (ledger/verifier) + one B42-family positive (serial reference walk vs quadratic re-prefill). No novel rule.
- `SchedulerV2.swift`: 2 novel candidates surfaced (span-snapping + chained-decode overlap) — DEFERRED, see below.
- `CompiledDecodeV2.swift` body: the `compile(inputs:outputs:)` in-place-write aspect is R3's content (confirmed); 3 novel candidates surfaced (container-rebind, hidden-footprint ledger, dtype priming).
- `LagunaDFlashBlockSession.swift` (never read before): B71 positive instance confirmed (the saturated-ring-cannot-trim contract + the Amendment-7 emitted-vs-KV-row off-by-one); rest is challenge-mechanics + benchmark protocol.

**Landed (8 rules, corpus 445 -> 453; A +3, D +3, E +2):** R1 `functional-cache-versioning-loses-donation-under-kernel-read` (A — category corrected from the verdict's tentative D: its siblings `append-cache-rewind-overwrite-in-place` + `kv-window-device-resident-ring-append` live in A), R2 `fused-kv-write-single-writer-reads-newest-from-input` (A), R3 `in-place-write-through-input-emits-fence-consumer` (D), R4 `residency-set-wired-once-post-warm-persistent-footprint` (D), R5 `register-budget-head-split-gqa-reread-trade` (E), + 3 novel: `batch-composition-invariant-grid-early-return` (E — per-row summation order bit-identical regardless of batchmates + early-return unread slots + consumer-side extent recompute), `never-gqa-multiply-full-d-merge-smem` (A — the source doc's own extracted rule: the 3-scheme merge menu, GQA-independent smem evidence), `container-rebind-over-retrace-for-churn-survival` (D — the FOURTH graph-churn mechanism beside B58 drop-or-survive / B81 retain+rebuild / B92 device-M / B97 static-sibling). Plus the B22 rider (Metal measured-cost note).

**Deferred with reasons (the honest queue):**
- Indivisible-attention-span chunk snapping + one-shot starvation guard (SchedulerV2) — multimodal-instance (bidirectional image blocks); no in-tree chunked-prefill consumer of the generalizable half. Reopen: a multimodal or span-atomic chunked prefill consumer.
- Hidden fast-path footprint charged to the admission ledger + refuse-on-half-budget (CompiledDecodeV2) — solid + generalizable, batch was full. Reopen: next qwen-3.8-mtp-challenge-adjacent batch or a paged-KV admission consumer.
- Chained-decode lazy-token overlap (SchedulerV2 + CompiledDecodeV2, SGLang-MLX-cited) — medium confidence, spec-decode scheduling lane.
- Prime host-syncs off the traced region + per-layer K/V dtype harvest (CompiledDecodeV2) — small; pairs as a B58 rider candidate.
- Scratch-lane batch padding with recycle-before-capacity — rider-scale on the pad family (B84/B92-padding).
- Warmup-past-the-wrap-seam — measurement methodology; the warmed-arm half is already encoded in riir-ai Bench-749/831 lessons.
- R6 command-buffer referenced-byte commit threshold (`MLX_MAX_MB_PER_BUFFER` 50 -> 512 MiB) — SKIPPED per plan (config-knob/env-var shaped, single framework; anchors verified only in `RuntimeStartupMemoryPolicy.swift`, NOT in the metal/kernel-research files — Lane A grep-confirmed).
- MLX dispatch-batching economics note (~50 ops/buffer, single-digit-us dispatches) — external confirmation of the B80 dispatch-overhead class; note-grade.

**GOAT:** kernel_opt lib 1262/0/1i (all pins + 3 new gates) + default lib 1012/0 count-identical + clippy exit 0 both states (`--no-deps`; one doc-markdown round on the new `//!` header — the B37 lesson again) + retrieval floors 14/14 HELD (clippy@1 89.6 / perf@1 96.2 / blend@1 94.7 — identical bands). Count pins walked: tests.rs corpus total + expected-array A/D/E + sums + grouped + router.rs expert total + mod.rs doc line + 3 category count comments + the batch22 full-concept pin (moved WITH the deliberate B22 rider — the gate caught the change exactly as designed).

**Tooling hazards recorded for future `.raw` verification passes:** the search tools cannot index the `.raw` clone (gitignored — silent "no matches"); several multi-command terminal invocations dropped all but the last command's output (single-command invocations reliable); grep output on very long lines gets elided — use `cut -c1-N` or read_file.
