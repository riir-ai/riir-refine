# Plan 268: Batch 264 — the uzu #994 coherent-qualifier rider (thin-watch conversion)

**Status:** IN PROGRESS — claimed 2026-10-20T03:24Z, shikuwa/4090 idle loop; **Claimed:** shikuwa-4090-idle, 1792463431, expires 2026-10-22T03:24:00+00:00

Source: trymirai/uzu @ `32cd07c3f648d65e77063f3105f36c7c85bf5d81` (#994 "Radix top-k
cross-threadgroup coherency", MIT) — ONE-line fix on
`crates/uzu-engine/src/backends/metal/kernel/radix_top_k_small.metal`:
`device ulong* selected_keys` → `device coherent(device) ulong* selected_keys`
in `RadixTopKSmallCollect`.

The host: `bounded-cross-worker-barrier-protocol-polling-fan-in-ladder`
(entries_gpu_85.rs, B223) — the SAME rule that carries B260's #991
arrival-merge face. lithos's own design clause (already quoted in the host)
"Coherent pointers and device fences publish intermediate data across groups"
gains its upstream defect instance: Metal's default `device` address space is
non-coherent, so a cross-threadgroup plain-store handshake buffer must carry
the `coherent` qualifier; the atomic surface already carries explicit ordering.

- [x] T0 — claim row in `.distill/001` + this plan (this commit; dual_allocation_gate 0/0/0, `.plans/.highwater` 267→268)
- [ ] T1 — rider authoring: RIDER-2 face appended to the gpu_85 `gpu_concept` in `entries_gpu_85.rs` (CRLF-preserving — the file is 349-CRLF-native; quotes verbatim at the pin, read from the vendor clone `E:/git/vendor/uzu`)
- [ ] T2 — needle gate: `tests.rs` `batch264_rider_landed` (verbatim needles incl. `coherent(device) ulong* selected_keys` + the count-neutral witness corpus == 923)
- [ ] T3 — gates: batch264 needle gate, kernel_opt lib, default lib, clippy −D ×2 postures (default + no-default), retrieval floors re-measure (isolated CARGO_TARGET_DIR, removed after)
- [ ] T4 — Research 300 note + `.distill/001` landing row + this plan tick + commit + push (single landing commit; binary rebuild NOT owed — kernel_opt outside the release set; freeze/score-bench NOT owed — zero clippy-corpus delta, P30)
- [-] T5 — llama.cpp #30430 shared-weight layout-transform law: graded WATCH/lane-intel, NOT authored this batch (SYCL-lane fix; our lane's Hadamard folds happen at load, before any consumer — the in-place per-op transform class has no surface today; re-opens if a per-op weight-transform lane lands)
