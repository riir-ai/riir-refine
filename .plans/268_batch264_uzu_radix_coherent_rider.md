# Plan 268: Batch 264 — the uzu #994 coherent-qualifier rider (thin-watch conversion)

**Status:** COMPLETE 2026-10-20 — T0–T4 all landed (the claim's own landing commit); T5 graded WATCH with rationale (no per-op weight-transform surface in our lane today; re-opens if one lands). Status line + lease line closed 2026-10-28 (zed-m3-idle hygiene pass): the checkboxes had said done since the landing while this header still read IN PROGRESS.
**Lease closed:** shikuwa-4090-idle (claimed 2026-10-20 epoch 1792463431, expired on landing 2026-10-22) — the batch-264 rider; landed per T0–T4.

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
- [x] T1 — rider authoring: RIDER-2 face appended to the gpu_85 `gpu_concept` in `entries_gpu_85.rs` (CRLF-preserving — python byte splice, 349 CRLF / 0 LF before AND after; quotes verbatim at the pin, read from the vendor clone `E:/git/vendor/uzu`)
- [x] T2 — needle gate: `tests.rs` `batch264_rider_landed` (verbatim needles incl. `coherent(device) ulong* selected_keys` + the count-neutral witness corpus == 923) — FIRST-RUN PASS
- [x] T3 — gates: batch264 needle gate FIRST-RUN PASS; kernel_opt lib 3193/0 (+1); default lib 2715/0; clippy −D ×2 postures clean; retrieval floors HELD 21/21 no re-pin (291.1 s, isolated CARGO_TARGET_DIR removed)
- [x] T4 — Research 300 note + `.distill/001` landing row + this plan tick + commit + push (single landing commit; binary rebuild NOT owed — kernel_opt outside the release set; freeze/score-bench NOT owed — zero clippy-corpus delta, P30)
- [-] T5 — llama.cpp #30430 shared-weight layout-transform law: graded WATCH/lane-intel, NOT authored this batch (SYCL-lane fix; our lane's Hadamard folds happen at load, before any consumer — the in-place per-op transform class has no surface today; re-opens if a per-op weight-transform lane lands)
