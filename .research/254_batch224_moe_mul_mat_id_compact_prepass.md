# Batch 224 — the llama.cpp MoE `mul_mat_id` compact-prepass pack (conversion tail + cycle-37 leads)

**Status:** LANDED (2026-10-12, shikuwa/4090, riir-refine idle loop cycle 38; Plan 226; claim row `1df9bcf9` pushed before authoring per the claim-marker protocol)

## What landed

**1 own kernel_opt rule (D-class, corpus 795 → 796) + 2 count-neutral riders:**

1. `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` (`entries_gpu_88.rs`) — the metal #30373 + vulkan #29998 cross-backend family. The founding defect: `kernel_mul_mm_id_map0` "assumed at most one match per token and a fixed capacity of ne21 entries per expert, which produced wrong or dropped output rows and could overflow into the slice of the next expert when the ids contain duplicates." The repair: a two-pass compact prepass (every occurrence = one entry; exclusive prefix sum with one padding slot per slice; pass 2 stores ONE UNCONDITIONAL value per token — "conditional stores of the divergent lanes are very slow" — the dummy lands on the never-read padding slot; duplicates ride a rare branch), per-tile `(expert, first entry, valid rows)` descriptors, a host-computable STATIC upper bound (full tiles + at most one partial per expert, min-clamped by n_rows — the same closed form on both backends), and a consumer that reads its descriptor and tail-early-exits. Measured (metal message): grid ~8-16x shrink ("128 tiles instead of 2048 entry tiles, of which 94% only launched to exit"), register pressure th_max 768 → plain mul_mm, pp +2% Qwen3.6-35B-A3B Q8_0 (883/883 parity). The ne02 function-constant fold measured "no measurable perf change" — the honest null, kept for kargs hygiene.
2. RIDER (rust_perf, `entries_02.rs` `channel-send-batching-unpark` name field): the cubecl #1806 CONSUMER-side park-vs-fixed-sleep half — "The server now parks instead, and the client whose task fills a buffer unparks it when it is parked. The park keeps the old step as its timeout, so a shutdown is still noticed within it, and an idle server still costs no CPU." (~20% decode, M2 Pro). New atoms: the fixed-sleep-remainder tax + park-timeout-as-watchdog + the SeqCst handshake ("either the server sees the full queue and does not park, or the client sees the flag and unparks it") + the unpark-token semantics ("An unpark that lands before the park is kept by the thread and ends the park at once"). Distinct-from: the host rule owns PRODUCER-side send batching; the rider is the consumer-side idle-loop discipline.
3. RIDER (dist, `bench-targets-shared-dependency-cluster-hint` notes): the C++ twin of the B221 rule — llama.cpp #30299 "tests : link each test against only the libraries it uses" (LINK keyword, "ggml-only tests now link just ggml", the dropped LLAMA_TEST_NAME/target_link_libraries workarounds, and the interface-leak half: "Both helpers now set cxx_std_17 explicitly, since the tests no longer inherit it from llama-common's PUBLIC interface.").

## Verbatim verification (the Batch-31 law)

Every quote fetched at the pins via the GitHub commits API this session (two parallel subagents; patch completeness reconciled +/- against the API `stats` block — none truncated):
- `073b74034b8bfe652e410ec8bd1e0d95a7d46fea` (metal #30373; parent `77757a346...`; +182/-59 across 5 files)
- `966baae76b578d5d3dd4c179a75a35c80ff75577` (vulkan #29998; parent `62a6f74ba...`; +200/-404 across 9 files)
- `ccdbe290533f68154ede46e0618622eebcb6c070` (#30299; parent IS `073b74034` — sequential)
- `6bba5fdf04206d9c3710dfbc46aa9850450ab405` (tracel-ai/cubecl #1806; parent `987d2a7ad...`; +57/-9, one file)

**Cross-backend honest-mapping corrections the verification surfaced** (now carried in-rule):
- The "conditional stores of the divergent lanes" comment, the padding-slot dummy mechanism, the th_max 768 note, the ~8-16x/94% figures, and the ne02 null are METAL-COMMIT-ONLY.
- The vulkan commit carries NO measured numbers; its scatter appends via atomicAdd cursors (duplicates by construction — no rare branch); its descriptor is 2 words `(expert, valid rows)` vs metal's 3; it chunks the count pass past `EXPERTS_PER_CHUNK 1024` (replacing the removed `MAX_EXPERTS 1024`); it moved the u16 packing constraint to op-support time; and it DELETED the subgroupBallot streaming scan (the "looping" the title retires — the vulkan twin of metal's round loop).
- The metal commit credits the vulkan: "follow the approach of the vulkan implementation (#29998)".

## Dedup (re-run at authoring, rule-slug level)

Core atoms UNOWNED (zero corpus hits): `tile-descriptor`, `launch-to-exit`, `launch to exit`, `static upper bound`, `unconditional store`, `divergent lanes`, `entry tile`, `compact list`; rust_perf side: `park_timeout`, `fixed-sleep`, `idle server`, `PARK_STEP` all zero. Neighbors adjudicated in-rule: `degenerate-selection-falls-back-to-identity-uniform` (B110 — SELECTION-side duplicate-id AVOIDANCE vs THIS consumer-side TOLERANCE; composed neighbors), `moe-per-expert-mixed-dispatch-worst-case-workspace` (workspace vs tiles), `tile-picker-input-from-dispatch-grid-partition`, `indirect-device-dispatch-tile-map-moe` (device-side indirect vs host static bound), `moe-router-weight-fold-into-reduce-never-materialize` (B220), the two `prepass` rules (different purposes).

**The walk conversion-set audit** (this batch's T0): #29435 LANDED (B205, `entries_gpu_75` `work-splitting-requires-work-shape-homogeneity`) and the B208-F16 rider LANDED (the F16-scores watch face inside `entries_gpu_78`, "revert reason unstated — never adopt") — both walk-set legs were already converted; only #1806 was live. The cycle-14/25/28 "conversion set" phrasings were stale on those two legs.

## GOAT

- G1 parse: the `batch224_rule_parse_and_needles_landed` gate (14 verbatim needles + syn-parse of both fixture halves) ✓
- G2 lib: kernel_opt 2944/0 (was 2943/1 at first run — the router.rs expert-total pin updated 795→796, the B223 T7 pattern); default 2504/0 ✓
- G3 clippy: clean at default AND `--features kernel_opt` after 2 en-route heals (my `MoE` doc_markdown backtick + a PRE-EXISTING too_many_lines on batch223's 149-line test fn — no allow above it; fixed with the annotated `#[allow]`, the 5-site house pattern in that file) ✓
- Retrieval re-measure (Issue 017): 21/21 passed; kernel self-variants **100.0%** (30 rules); blended 171 at **91.2/94.7/95.3%** — every floor held, NO re-pin (the batch costs zero retrieval decay)
- Freeze-table/score-bench: NOT owed (zero clippy-corpus delta — the B223/P30 kernel-batch precedent)

## Watch (disclosed, unconverted)

- **vulkan switch→ternary #30164** (`08ffba21e`): GLSL→SPIR-V origin; transferability to WGSL(naga/tint)/MSL UNVERIFIED — the intake row's own gate ("grade against a real wgpu shader at conversion or watch-only"). Stays watch until a wgpu shader-lowering probe lane exists.
- **F16C proxy-detection #30270** (`1cf77677`): runtime proxy-feature class (AVX2 ⇒ F16C, false on odd CPUs); our arms are compile-time `target_feature` — the shipped_target_feature rule owns a different class. Re-open only if a runtime-detection lane lands.
- **League intel (not rules)**: Bonsai-2-27B UPSTREAM in llama.cpp master (#29600) — the perf-rematch league's next bout gains a native-serve opponent arm (the intake row; the league doc's bout operator owns it). cubecl 0.11.0-pre.2 vendored-pin lane intel rides the #1806 rider text.

## Numbering

Plan 226 (`.plans/.highwater` 225→226); Research 254 (`.research/.highwater` 253→254); Batch 224 (last = 223); `entries_gpu_88.rs` (max was 87). Dual-allocation gate at claim: 0/0/0.
