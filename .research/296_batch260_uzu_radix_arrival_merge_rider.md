# Research 296 — Batch 260: the uzu #991 radix arrival-merge rider (B223 host)

**Status:** RECORD — landed as Batch 260 / Plan 264 (commit on develop); count-neutral (corpus stands 920).

## Source

- **trymirai/uzu @ `56dbb91e9bd6658b397041823270f36832f9234b`** (#991 "DFlash-TfM clean up and radix top-k race fix", MIT, merged 2026-10-19T19:00Z). Read this session: the PR .diff (132 lines), the full `radix_top_k_small.metal` (219 lines) and `radix_top_k_small.rs` host (108 lines) at the pin, plus the #1861/#1864/#1865 cubecl PRs and the llama.cpp +3 window for the watch rows below. The B259-close standing watch enumerated the deltas; this note grades them.

## The conversion — ONE count-neutral rider

**Host:** `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` (B223, gpu_85) += the **MULTI-DISPATCH PASS-LOOP ARRIVAL-MERGE face**. B223 owns the device-scope fence discipline for cross-worker fan-in inside ONE persistent dispatch (bounded polling, monotone epochs). The uzu instance is the same discipline at per-row/per-partition granularity across REPEATED DISPATCHES — and with NO spinning at all (fire-and-forget arrival, last-arriver-does-the-work):

- `arrive_last` (radix_top_k_small.metal:16-22): `atomic_thread_fence(mem_flags::mem_device, memory_order_seq_cst, thread_scope_device)` → `atomic_fetch_add_explicit(count, 1u, memory_order_relaxed) == expected - 1` → **`if (last)` a second device-scope fence**. The release fence publishes the partition's prior relaxed atomics; the ACQUIRE-side fence is CONDITIONAL — only the last arriver pays it (the one threadgroup whose subsequent reads of the other partitions' partial histograms need it).
- The merge gate (:100-104): `is_last` re-broadcast through `threadgroup_barrier(mem_threadgroup | mem_device)` BEFORE `if (!is_last) return;` — every partition's threadgroup reaches the gate barrier; only the winner continues.
- THE FIX (the commit's one-kernel hunk, :127-135): the merge window's handoff barrier ordered only the THREADGROUP-memory second-level prefix (`simd_prefixes`) while the window's data flow crosses DEVICE memory (the pass-carried `ranks`/`prefixes`/`prefix_masks`, and the last-arriver identity can change per pass). Old: `threadgroup_barrier(mem_threadgroup); higher += simd_prefixes[...]; uint rank = ...ranks[row];`. New: the independent device load hoisted ABOVE a widened `threadgroup_barrier(mem_threadgroup | mem_device)`.
- THE RESET FACE (:24-27 + :152-155): `reset_arrival` OPENS with the same device-scope seq_cst fence before the relaxed store to 0, and the call site orders the winner's merged writes → `threadgroup_barrier(mem_device)` → `reset_arrival(&done_counts[row])`. This is the counter-RESET half of B223's monotone-epoch law — the half B223's own variants deliberately refuse (a reset races stragglers WITHIN one dispatch); uzu's reset is safe exactly because the pass boundary is a whole-dispatch ordering (host: `for pass in 0..self.passes { self.pass.encode(...); }` — one dispatch per pass, all scratch allocated once, radix_top_k_small.rs:80-91). Crossing the two breaks both.

**Dedup:** the barrier-SCOPE conjunct is named in `bit-exact-dispatch-fusion-boundary-emulation` (gpu_12 — fusion-emulation context) and B57's Batch-113 rider (cross-simdgroup scratch handoff); the uzu instance is a third context (multi-dispatch arrival merge) — rider, not rule. Distinct-from `loop-back-edge-scratch-war-trailing-warp-barrier` (B253, threadgroup-internal WAR) and `smem-slot-role-reversal-needs-barrier` (B57, threadgroup-memory roles).

**NotBenchable:** ordering-correctness decision content; the source ships no numbers on this axis.

## Watch rows (no corpus)

- **cubecl #1861 `8b752e1e`** (merged 10-19T15:58Z, inside the B259 tip — the "ungraded" one, now graded): "SPIR-V buffer tagging when read only" — the NonReadable/NonWritable decorations moved from the args-struct's pointer MEMBER onto the buffer struct's own member (the runtime array), plus the info struct's fields now declared NonWritable ("Info is only ever read"). Body: "In RADV this causes about a 20% performance regression on my hardware... why this didn't impact Nvidia hardware as much as RADV... more of a coincidence that the driver worked out that it is read only." → **declaration-accuracy class** (an access-mode fact must decorate the MEMORY TYPE the driver's optimizer reads, never only the pointer use; drivers that re-derive the fact mask the cliff). The #1845 precedent (shared-vs-global atomics capability split, "declaration-accuracy class, B− at most") stayed watch-only — same verdict here; lane intel rides the vendored cubecl pin-bump watch (riir-infer vendors 0.11.0-pre.2; a release carrying #1861 joins #1806/#1359 on the bump trigger).
- **cubecl #1865** "Feat/cubecl inspect" — autotune-cache introspection tooling ("what a file holds, how each autotune key was decided, where a build's time went"); adjacent to B223's autotune-fidelity family but tooling, no kernel law. NULL/watch.
- **cubecl #1866** deps bump (tracel-llvm-bundler 23.1.0-4). NULL. (#1864 = build/docs fix, its merge sha IS the B259 pin `51d84aa38` — already consumed posture.)
- **llama.cpp +3 past `0ce8cd114`**: `6fa4f6cd3` #30756 opencl get_rows/cpy i32+f32 + `1b29621ba` #30741 opencl cooperative-k dp4a gemm q4_0/q4_k/q6_k — OpenCL platform-class pair, the standing watch posture (no CUDA/Metal law; the cooperative-k dp4a shape noted for any future OpenCL lane); `4c3c5810f` #30825 hybrid dna tokenizer fixes — NULL. **New envelope pin `4c3c5810f`.**
- **uzu #992** sysinfo iOS workaround, **#993** simulator stubs / xgrammar-rs rewrite — build/infra NULL.
- **DFlash-TfM cleanup half of #991** (lane intel, no corpus host): the greedy-noise interaction moved from `make_shape` to the encode site — `prune_sigma = match sampling_method { Greedy => None, Stochastic{..} => prune_sigma }` with the removed comment "Greedy verification adds no Gumbel noise to the target logits, so pruning has none to anticipate." The law shape: a noise-anticipating parameter gates on the noise FIELD's presence, not on the sampling enum at shape time. Left unconverted — no corpus host without inventing one on the Gumbel family (B107).

## League lane note

uzu remains the M5-class prefill reference for the chunked-GDN family (B259's pack); #991's race fix touches `radix_top_k_small` (sampling/top-k), not the GDN prefill path. Detection-only; no scoring lane touched.

## Gates (all run this session)

- batch260 needle gate: first-run PASS (host id + face name + the pin + 4 verbatim fragments + Batch 260 tag + the count-neutral witness `entries.len() == 920`).
- kernel_opt lib 3176/0 (1 ignored = the documented freeze-regen e2e); default lib 2702/0.
- clippy −D ×3 postures clean (default / kernel_opt / no-default, all-targets).
- Retrieval floors HELD 21/21 no re-pin (362.74 s; blended 90.6/94.2/95.9 above floors 89.4/93.6/93.6).
- Freeze/score-bench NOT owed (zero clippy-corpus delta — the write set is kernel_opt corpus text + one test; P30/B250). Binary rebuild NOT owed (kernel_opt not in the release set).

## En-route notes

- The CRLF hazard fired INVERTED this session: `.distill/001_mining_queue_snapshot.md` was normalized LF by my claim commit's editor churn and repaired in the follow-up commit (`9d3baf735`); `entries_gpu_85.rs` is NATIVELY CRLF in the repo (349 CRLF at HEAD) — my first normalization pass created 698-line churn, caught at the diff (--stat), reverted, the rider re-spliced CRLF-preserving (final diff: 1 line). Lesson recorded: check the HEAD blob's line endings BEFORE normalizing an edited file — the B259 hazard class is "editor introduces CRLF into LF files"; the inverse ("normalizer flattens a natively-CRLF file") is the same verify-the-diff discipline, one more face.
- `.plans/.highwater` was stale at 262 (plans 263/264 exist) — bumped to 264 with this plan's allocation (monotonic, never reuse).
