# Research 378 — trymirai/uzu decode-path distill (riir-infer Issue 1178 T0)

**Status:** DISTILLED — pending owner decision

## Source

- **Repo:** `trymirai/uzu` — Rust/Metal LLM inference engine (workspace `uzu-engine` + legacy bindings; MIT).
- **Pin:** `1866d189f91a5083dbb1394eeaee07ff8ecdfb6d` (2026-11-05, "Decode Trellis weights without tail padding (#1076)"). Cloned shallow into `.raw/uzu`, removed at close. The prior queue pin `69c64cb` (11-04) was one window older; all quotes verified at THIS pin.
- **Inspection date:** 2026-11-06 (M3 box, zero-cargo session — clone greps + file reads only).
- **Context:** riir-infer Issue 1178 T0 — the first task of the uzu decode-path close (Bench 1167's 0.23× dense-decode datum: ours 12.05 tok/s ≈ 24% of serial roofline vs uzu 52.9 ≈ 94%).

## Verdict (per the distill skill format)

**YES B− corpus / LANE INTEL CRITICAL.** Domain `kernel_opt` (host-orchestration-around-GPU + Metal kernel classes); not `rust_perf`.

### Provenance chain

uzu is an independent first-party engine (trymirai/Mirai); its kernels are original `.metal` sources behind a Rust macro DSL (`VARIANTS`/`CONSTRAINT`/`KERNEL` in `common/dsl.h` + the `uzu-engine-macros` crate), not a port of a named upstream. Where its atoms coincide with public technique classes the corpus already owns (magic-anchor dequant, half-mantissa code embedding, int8 KV), those coincidences are instances of OUR corpus's laws, not provenance links.

### Dedup-grep evidence (both vocabularies, `src/domains/kernel_opt/` + `rust_perf/`)

| atom | source term | corpus-operator term | hits | verdict |
|---|---|---|---|---|
| in-flight decode chaining | "in-flight decode", "pipelined step", "forward pass pending" | same | 0/0/0 | NOVEL |
| access-tracked barrier elision | "barrier elision", "access track", "read write overlap", "RangeSet" | "encoder barrier", "range set" | 0/0/0/1(the rangemap dep row — read, unrelated) | NOVEL |
| MTL4 argument table | "argument_table", "ArgumentTable" | "bind once" | 0/0 | NOVEL (API face) |
| commit-feedback completion | "commit feedback", "feedback handler" | "completion callback" | 0/0/2(read, unrelated) | thin novel face |
| magic-anchor dequant | `0x4B400000` | — | 3 (all gpu_106 B251) | INSTANCE — gpu_106 owns |
| half-mantissa code embedding | "half mantissa", "w4_pair" | "packed half", "exponent bias" | 0/0/1/2 (gpu_09's `bits \| 0x6400` f16-exponent class) | ADJACENT — gpu_09 class |
| int8 KV in-place | "int8 KV", "quantized KV", "packed KV" | — | 1/3/1 (gpu_34/69/99) | INSTANCE |
| act-quant → next-GEMV format | "activation quant", "int8 activation" | — | 5/7 (gpu_01 + B335 anti-face) | INSTANCE |
| weaver/trie/DFlash | "trie", "draft tree" | — | 232/5 (B299's own family) | INSTANCE |
| GEMV tile policy | "fp_tile", "gemv policy" | "split-K" (35), "tile policy" (3) | — | INSTANCE + 1 face |
| fused residual-into-norm | — | "one encoder" (7: gpu_02 family) | — | INSTANCE |

## Ranked candidate table

| # | pattern | source file @pin | grade | notes |
|---|---|---|---|---|
| 1 | **decode-step-inflight-chaining** — the decode state machine encodes forward-pass N+1 while N executes; the CPU-GPU sync lands ONLY at the acceptance/trie-readback boundary (`ForwardPassChaining::InFlight`, the `pending` vec, `copyout` future); a fresh `AllocationPool` per forward pass double-buffers backing memory so the in-flight pass's buffers are never reused while executing; grammar bitmasks force a split-buffer resync (the one named sync point) | `crates/uzu-engine/src/engine/language_model/stream/stream.rs:380-810` (+ `ForwardPassChaining::resolve` :43-88) | **A−** | THE 4.4× mechanism answer; feeds riir-infer 1168's measured class ("per-token host-sync serialization is the measured deficit"). NotBenchable-by-domain (needs a serving validator) — dial9 class (1) |
| 2 | **access-range-tracked-barrier-elision** — ONE compute encoder per decode step; every kernel declares its byte-range accesses; a barrier is emitted ONLY when the dispatch's ranges overlap the running `RangeSet<u64>` of prior reads/writes, and the sets clear on emit — data-flow-precision barriers instead of encoder-wide ones. In-source TODO "more fine grained barriers" names the coarseness ceiling | `crates/uzu-engine/src/backends/metal/command_buffer.rs:108-131` | **B+** | zero owners; Rust-shapeable (rangemap); the corpus's barrier rules are all barrier-REQUIRED (gpu_269) — this is the elision pole |
| 3 | argument-table-bind-once (MTL4 `MTL4ArgumentTable`, max 31 buffer binds, set once per encoding; per-dispatch binds eliminated) | `command_buffer.rs:76-88` | B | Metal4-API face; the wgpu pole is owned by `prebuilt-bind-groups-fixed-buffer-combos` (gpu_02) — cite-kinship, no standalone rule for a wgpu-first lane |
| 4 | commit-feedback completion (MTL4 `add_feedback_handler` → mpsc; `wait_until_completed` is a future on the callback, no polling) + `command_allocator_cache` (reset+pool of MTL4CommandAllocators) + the 256 KiB constant bump allocator per encoding | `command_buffer.rs:296-330, 76-99` | B− | host-side Metal4 mechanics; wgpu analogs exist; faces |
| 5 | int8-symmetric KV + Q with per-tile scales consumed in-place (the QUERY also int8 at projection output; `kv_layout.h` per-TILE_KEYS scale indexing keyed to the MMA lane geometry) | `attention/vector.metal`, `attention/kv_layout.h`, `int8_quantization.h` | face | gpu_34/69/99 instances + the Q-quantized extension |
| 6 | PackedHalf — W4 codes embedded as half-mantissa PAIRS (two 4-bit codes share one half's mantissa bits 10+; per-position exponent-bias correction terms `w4_pair_bias_value`); the dequant multiply absorbed into the half format | `matmul/common/quant_unpack.h:37-76`, `matmul/common/packed_half_layout.h` | face | gpu_09's f16-exponent class, wider variant |
| 7 | GEMV policy tables (`fp_tile` m/n/k shape table; quantized lane-sliced tiles by bits/group; `cap_k_split_to_complete_fp_k_blocks` — k_split capped to COMPLETE vectorized K blocks) | `matmul/gemv/policy.rs`, `gemv/policy/quantized.rs` | face | split-K family (35 hits); the completeness cap is the one crisp face |
| 8 | magic-anchor W8 dequant (`W8_MAGIC_FLOAT_OFFSET`, `magic_code_float`, `magic_dequant_bias = -12582912·scale`) | `quant_unpack.h:52-56` | drop-instance | gpu_106 (B251) owns the anchor; uzu = the accumulator-subtract pole instance |
| 9 | weaver/DFlash draft tree + trie-arg attention + `fuse_select` admission | `encodable_block/weaver.rs`, `dflash.rs`, `mixer/attention/mod.rs:497` | drop-instance | B299's own family (mined from uzu #1022) |

## GOAT candidate identification

Candidate 1 (in-flight chaining) tested against the four criteria:
1. **Benchable?** No validator in the corpus hosts a decode-serving loop — criterion 1 fails for a DOMAIN reason → per-rule ANNOTATION (`NotBenchable`, `bench_spec: None`), the Batch-136 precedent. Not a demotion.
2. Exact semantics: n/a (host orchestration; correctness = the token stream, verified by the consumer lane's G1).
3. Magnitude: mechanism-level — uzu's measured 94%-of-roofline serial decode vs our 24% with 1168's host-sync triage naming the deficit class; the roofline arithmetic (52.9 tok/s × 16.35 GB ≈ 770 GB/s) bounds the prize at ~4.3×.
4. Deterministic: the chaining is state-machine-deterministic (the acceptance boundary is the only readback).

Candidates 2-4: same domain-class annotation; decision content is the placement/admission law.

## Lane-intel axis (CRITICAL — this is the verdict's primary output)

- **Mapped lane:** riir-infer dense-qwen38 decode close. Bench 1167 (2026-11-05, bmo): uzu serial 52.9 tok/s (94% roofline) vs ours 12.05 (24%) — 0.23×. Issues: **1168** (host-sync triage — same class, Bonsai lane; shared mechanisms consume once), **1177 item 2** (dense-f16 decode GEMV price cell — leased to zed-83e79a9e@amo to 2026-11-07; coordinate, don't duplicate), **1178** (this distill's parent; T1-T3 pricing/bench legs UNCLAIMED).
- **Transferable:** candidates 1+2 (the chaining posture + the barrier elision) — they decompose the 4.4× into (dispatch cost floor) + (sync boundary placement), exactly the decomposition Bench 937 did for prefill.
- **Not transferable verbatim:** the MTL4 API mechanics (argument table, commit feedback) — wgpu/CubeCL equivalents differ; the law transfers, the plumbing doesn't. The int8-KV/Q axis is a WEIGHT-FORMAT change (mirai-m vs our GGUF) — a pricing-lane question, never a corpus rule.
- **Offered follow-up:** the 1178 T1 decode frame-census (attn/GDN/FFN/embed/small-stages/dispatch-gap shares on bmo) prices candidates 1+2 against measured shares; T2/T3 price cells + GOAT gate per the issue's own plan. The corpus rules from candidates 1+2 ride the NEXT kernel_opt batch after T1 confirms the shares (plan-first when the batch opens).
- **Absolute-numbers caveat:** uzu's 94% is their box/model/quant (M5 Ultra, mirai-m 4-bit); never headroom evidence — only same-box league cells own comparisons.

## Product-coverage axis

Does not fire: uzu is an engine, not an ecosystem lib; its patterns' home (Metal serving runtimes) is exactly our workspace's own surface. No famous-lib family gap opened.

## Honest caveats

- **Unverified surfaces:** the `slang/` and `webgpu/` backends, `matmul/gemm/common/schedules/` (the small-batch schedule tables), `moe/`, `sampling/gumbel+prng`, and the `generated/` kernel-variant machinery were NOT read (re-read at mine time if a batch opens). The `encodable_block/` surface (weaver, dflash, embedding, normalization, linear, mlp) WAS read.
- uzu moves fast (one-day pin delta at clone time); quotes byte-verified at `1866d189`; a mining batch re-pins.
- The chaining candidate's wgpu port carries a REAL risk class: wgpu's `on_submitted_work_done` + map_async readbacks are coarser than MTL4's feedback — the T2 price cell must prove the port's sync boundary is as clean before any adoption claim.
- Zero default-build surface claim: uzu's chaining is their production posture (not env-gated), strengthening the law's weight.
- The owner decision this note pends: whether candidates 1+2 join the next kernel_opt batch (the distill verdict is YES; the batch is a separate plan-first act).

## Queue slot

Staged in `.distill/001_mining_queue_snapshot.md` (the 2026-11-06 delta row). The riir-infer 1178 T0 row carries this note's number as its verdict pointer.
