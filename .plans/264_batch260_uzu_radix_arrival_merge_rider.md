# Plan 264 — Batch 260: the uzu #991 radix arrival-merge rider (B223 host)

**Status:** IN_FLIGHT — claimed 2026-10-20 ~04:3x +07 (shikuwa/4090, riir-refine idle loop), claim row pushed `5e77b5041` (+ LF normalize `9d3baf735`), expiry 2026-10-22 ~04:3x +07.

Thin-watch batch: ONE count-neutral rider, ZERO own rules, ZERO count moves. Source: trymirai/uzu @ `56dbb91e9bd6658b397041823270f36832f9234b` (MIT) — #991 "DFlash-TfM clean up and radix top-k race fix". Host rule: `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder` (B223, entries_gpu_85.rs).

## Verified source facts (read this session, pre-claim)

- `radix_top_k_small.metal` at the pin: `RadixTopKSmallPass` — one threadgroup per (row, partition) via `GROUPS(rows* partitions)`; the fix hunk moves `uint rank = pass == 0 ? k - 1 : ranks[row];` ABOVE the barrier and widens it `threadgroup_barrier(mem_flags::mem_threadgroup)` → `threadgroup_barrier(mem_flags::mem_threadgroup | mem_flags::mem_device)` before `higher += simd_prefixes[lid / METAL_SIMD_SIZE];`.
- `arrive_last` (lines 16-22): `atomic_thread_fence(mem_flags::mem_device, memory_order_seq_cst, thread_scope_device)` bracketing the `done_counts[row]` fetch_add — release+acquire pair at device scope.
- Merge tail: winner writes `prefixes[row] |= …; prefix_masks[row] |= …; ranks[row] = rank;` then `threadgroup_barrier(mem_flags::mem_device);` then `if (lid == 0) reset_arrival(&done_counts[row]);` — the reset is fenced AFTER the merged writes.
- Host `radix_top_k_small.rs` `encode()` (lines 80-90): `for pass in 0..self.passes { self.pass.encode(…) }` — ONE dispatch per pass into one command buffer; all scratch (histograms/prefixes/masks/ranks/counts/done/keys) allocated once before the loop.

## Tasks

- [x] T0 Claim row pushed (`5e77b5041`) + snapshot LF-normalized (`9d3baf735`).
- [x] T1 Source quotes verified at the pin (kernel + host + `arrive_last` + merge tail read in full; the diff hunk byte-read from the PR .diff).
- [x] T2 Dedup greps: barrier-scope law named in gpu_12 decision content (fusion-emulation context) + B57 Batch-113 rider (cross-simdgroup scratch) — uzu is a THIRD context (multi-dispatch pass-loop arrival merge), distinct-from B253 (threadgroup-internal WAR back-edge) and B110 (spin-barrier co-residency — uzu's arrive_last spins not at all); B223 is the family host.
- [x] T3 Rider authored: B223 rule += the MULTI-DISPATCH PASS-LOOP ARRIVAL-MERGE face (scope-widening + independent-load hoist + reset-fence faces; 3 decision contents). Count-neutral. LANDED — final diff 1 line in entries_gpu_85.rs (the CRLF-churn detour reverted; the file is natively CRLF).
- [x] T4 batch260 needle gate in `corpus/tests.rs` (host id + face name + pin fragment + Batch 260 tag) — FIRST-RUN PASS, incl. the count-neutral witness entries==920.
- [x] T5 Gates: kernel_opt lib 3176/0 / default lib 2702/0 / clippy −D ×3 postures clean / retrieval floors HELD 21/21 no re-pin (362.7 s). Freeze + score-bench NOT owed (no clippy-corpus delta, P30); binary rebuild NOT owed (kernel_opt not in the release set).
- [x] T6 Research note 296 + snapshot landing row + `.docs/11_domains/kernel_opt.md` Batch-260 row (count-neutral note). AGENTS.md count UNMOVED (920 stands).
- [x] T7 Commit `feat: batch 260: ...` + push; clones/scratch removed (C:\t_b260 scratch + /tmp target). LANDED `8c98f76b1`.

## Defers

- [-] T-defer cubecl #1861 (`8b752e1e`) SPIR-V read-only tagging — declaration-accuracy class; the #1845 precedent stays watch-only; rides the vendored cubecl pin-bump lane as lane intel. NOT converted (watch row + research note).
- [-] T-defer DFlash-TfM greedy-noise face (prune_sigma=None under greedy) — lane intel in Research 296 only; no corpus host without inventing one on the Gumbel family.
