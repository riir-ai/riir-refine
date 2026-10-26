# Research 327 — Batch 288: the b10826 fork decode quartet + the grammar stack-walk lead

**Status:** RECORD — LANDED. Corpus kernel_opt STANDS 977 (five count-neutral riders, four hosts); rust_perf 273 → **274** (+1 own rule). Zero GPU throughout (the sibling QRR cell held the 4090; CPU-only batch posture per the B287 precedent).

**Filed:** 2026-10-26 ~11:4x–13:2x +07, shikuwa/4090, riir-refine idle loop (walk-17-continuation session). Claim `47126a91a` (push-before-authoring; LF-normalized at `ea494f79a` after the CRLF-sweep repair), plan 293 discharged by this note.

## Sources (every quote verified verbatim at T0)

- **PrismML/llama.cpp fork @ `c9c34bc9`** — the scored tag `prism-b10826-c9c34bc9`, = b10822 + EXACTLY four fork decode commits (the watch poll's compare `bf8c069f..c9c34bc9` ahead_by 4 / behind 0, merge-base == b10822; NO upstream merges — #29948/#31025 NOT in the tag, so a pp cell at b10826 still measures the pre-#29948 opponent). Read via the vendor clone `E:/git/vendor/prismml-llama.cpp` fetched to the pin (origin `PrismML-Eng/llama.cpp`) — the bout lane's own pin source, better provenance than a fresh clone; the planned `.raw/b288-llamacpp` blob-filtered clone TIMED OUT on this box's GitHub link and was abandoned (the deviation is disclosed in plan 293 T0). Pins: #395 `77a0affd8`, #397 `9f9148451`, #401 `dc21b2cf4`, #402 `c9c34bc9c`.
- **ggml-org/llama.cpp #31153 @ `48c9f08b7`** — the fetched `.patch` (1554 lines, full PR body + diff), `.raw/b288_31153.patch`.
- **SKIP (documented):** upstream `d854d98b` #30463 — CUDA VLA→std::string: init-path modernization where allocation is ADDED on an init path; the watch poll's SKIP-class verdict stands, no entry.

## The league context (why this batch is the b10826 window's companion)

The watch poll's escalation read: the b10822→b10826 delta is 2B/4B-weighted (#402's own body: 27B unaffected; #397's new arm Hopper-only) → the 27B decode/pp cells at b10826 should read ~parity vs the bf8c069 prep build; the owed bout RE-TARGETS `c9c34bc9` (re-pin + fresh MSVC rebuild + `.distill/003` tuple update BEFORE the cells run) and stays queued behind the sibling QRR cell (~14:40 +07 ETA). This batch distills the opponent's code; the bout measures it — disjoint lanes, one window.

## What landed (the five riders + one rule)

1. **#395 producer-side register + context-table** (rider on `fused-activation-quantize-for-gemv-consumer`, gpu_01) — the #30489 consumer-fold's PRODUCER twin: the fused norm+FWHT kernel writes q8_1 rows directly in the consumer's layout (AoS at 1 column, the PQ2 layout at multi-column widths); consumers find them via `ctx.fwht_q8()` and skip their quantize launch. Identity by per-32 transcription (the q8_1 writer re-executes quantize_q8_1's exact form). The admission is the extracted shared consumer scan `ggml_cuda_fwht_q8_find_uses`: every-use-is-src1-of-one-wtype mmvq matvec, the aliases[8] reshape-view walk, use-count equality, the ordered-PQ2 arch refusal, the cuBLAS padded-compute-buffer-view refusal. `GGML_CUDA_NORM_FWHT_Q8=0` disables.
2. **#402 DEFAULT-ON promotion** (same host) — the opt-in flips demote-only (`=0` restores PTQ1_0-only ≤2048); the fused quantizer learns the PQ2 layout arm; **the stale mutual-exclusion is deleted** — a promotion that subsumes an opt-in must delete the exclusions that guarded the old default. H200: 2B +5.6-5.9% / 4B +8.1-9.2% / 9B +0.4-0.7% / 27B unaffected; ~390 launches/decode removed.
3. **#401 block-diagonal truncation** (rider on `fwht-wide-block-row-per-threadgroup-exchange-ladder`, gpu_27) — the butterfly template gains `B = N`; only stages with stride < B run; each segment bit-identical to a width-B transform ("Each stage is the same add/subtract whichever level runs it"). The admission widens from width-equality to `mm->ne[0] == mm->src[0]->ne[0]` + `supported(ncols, blk ∈ {512,1024,2048,4096} ≤ ncols)` — the compiled-template-set admission on TWO axes. Bonsai-2 2B: 512-wide blocks on 2048-wide rows, +6-7% decode.
4. **#397a per-arch floor** (rider on `regime-batch-floor-force-escape`, gpu_54) — `ggml_cuda_pq2_mc_min(cc)`: Hopper 1 / elsewhere 3, env override as the escape arm; the floor comment carries its own evidence boundary ("Elsewhere it starts at 3 columns (not measured below that)") — a floor is a measurement-scope claim keyed on the axis that measured it. H200: -10..-26%/call at 1 column, +6% 9B decode; NOT bit-identical (accumulation order — disclosed, the Issue-940 law's dispatch-site face).
5. **#397b feature-axis specialization** (rider on `nvfp4-row-tile-count-compile-time-switch-dispatch`, gpu_102) — bias/gate+GLU fusion as template params: "a runtime bias check there costs the 8-column verify kernel ~12% on H200"; the runtime check happens once at the host-side launch dispatch. The count-axis host + this feature-axis face + the admission half compose the one change.
6. **rust_perf +1 `recursive-input-walk-explicit-stack-indices-not-copies`** (complexity, entries_51) — the #31153 double law: the AXIS half (a recursion depth limit must count the RECURSING axis, never a per-input quantity traveling with the data — "counted the code points of a vocab token, so any token of 256 chars or more was always rejected under a grammar"; the 512-space token of GLM-4.7-Flash class — a correctness hole wearing a perf guard's clothing) + the WALK half (explicit stack of levels, no recursion no limit; levels carry candidate INDICES, not copies — "the rejects map back to the input directly"). Source-reported: faster than master on GLM-4.7/Laguna/Qwen3.6/gemma-4, same masks. NotBenchable annotated; fixtures ORIGINAL, standalone-compiled+executed 2/2 green (the fixed arm's pop-semantics defect caught on trace + repaired BEFORE landing — survivors `std::mem::take` on advance, the splice carries rejects not survivors).

## T1 adjudication notes (dedup, 3+ vocabulary variants)

- Producer-fold: `quantize.*launch|skip.*quantize|prequant` → the corpus's fold laws are consumer-side (#30489 rider, B111 rope→cache) or the B283 priority-yield mention (`try_fwht_q8` as the deeper fusion, not owned); the producer-registers-table mechanism was UNOWNED → rider on the family host, per the watch poll's anchor.
- Block-diagonal: `fwht|hadamard|butterfly|ladder` hosts = gpu_11 (xor-butterfly reduce), gpu_27 (exchange ladder), gpu_45 (staging ladder), gpu_120 (runtime-specialize layout), gpu_128 (look-ahead + shared butterfly). The stage-TRUNCATION face (delete the ≥B stages) is unowned; the ladder host is the substrate-classifier the truncation composes with.
- Per-arch floor: `regime|crossover|dispatch-table|per-cc|hopper` → `sm90a-window-shape-gated-wgmma-dispatch` owns the exact-ARCH-WINDOW admission (wgmma fires at all), `regime-batch-floor-force-escape` owns the FLOOR+escape shape — #397a is the floor keyed per-arch → the floor host.
- Template-param: `constexpr|template-param|runtime-branch|compile-time` → gpu_117 (constexpr unroll extent), gpu_102 (compile-time switch dispatch) — #397b is the feature-axis face of the gpu_102 host's count-axis law.
- Grammar stack-walk: `recursion|explicit-stack|depth-limit` over rust_perf = NO owner (the grep hits are all kernel-side recurrence families or unrelated) → own rule; `grammar-window-draft-prune-path-local-mask` is the verify-tree prune, one layer up — cited as distinct-from, not a host.

## Gates (ALL GREEN)

- batch288 needle 2/2 FIRST-RUN (kernel riders 1/1 at `--features kernel_opt` + rust_perf rule 1/1).
- kernel_opt lib **3241/0** (1 ignored = the documented freeze-regen e2e); default lib **2734/0**.
- clippy `-D warnings` ×3 postures (default / all-features / no-default, `--all-targets`) — all clean.
- Retrieval floors HELD both postures, no re-pin: kernel+latent **22/0** (~493 s), default `retrieval_` **21/0** (~95 s).
- Score-bench #202 appended: heal **0.9636 FLAT**, parse_safety **1.0** (owd: rust_perf corpus delta).
- Freeze gen2/gen3 NOT owed (zero clippy-corpus delta, P30).
- Release-set binary REBUILT + installed: `build: ea494f79a1b7`, `--info` serves rust_perf **274** (rust_perf IS release-set surface — the B283 law; the build ran at CARGO_TARGET_DIR=/tmp/b288_release to spare the shared target dir).

## Count-pin edits (the full map, for the next lane)

rust_perf tests.rs: line-19 total 273→274 + lineage; without_spec 152→153; annotated 220→221 ×2 (serde round-trip + the drafter-propagation pin — the second caught by the FAILING-TEST ENUMERATOR, not by grep: the enumerator is the reliable list, the plan's T3 note repeats); batch283/batch284 count witnesses → 274 with lineage prepends; new `batch288_rules_landed`. kernel-side: the cross-domain pin 273→274 + the Sum comment + all four 977 message pins annotated count-neutral (977 stands). mod.rs: header 274/14 + `mod entries_51` + the extend. AGENTS.md ×2 bullets 273→274.

## En-route

- The `.plans/.highwater` + `.research/.highwater` bumps (292→293, 326→327) rode the claim commit; dual_allocation_gate 0/0/0 at claim time.
- The claim-commit CRLF sweep (the edit tool rewrote the snapshot LF→CRLF, +2001/−2003 noise) normalized back at `ea494f79a` — the ninth instance of the CRLF edit-lane hazard class recorded in the corpus docs; bash-side writes stay the safe path.
- The Rust docs bullet in AGENTS.md `## Yours` (273→274) was NOT the edit — the two domain bullets were; `--version`'s compiled feature set gained `scoped_recheck` vs the 10-25 binary (the workspace default set grew upstream; the release-set check passes).
- `.raw` artifacts this batch: `b288_31153.patch` + the two fixture .rs/.exe compile probes — swept at close.
