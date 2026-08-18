# 12 — Apple Neural Engine (ANE) substrate knowledge

> **Purpose:** reverse-engineered private-runtime ANE access knowledge,
> distilled 2026-08-19 from the Issue 726 T0 binding spike (riir-ai,
> commit `5b9fdb73c`) so it never has to be relearned. Everything here was
> **measured on this exact box** (M3 Max, macOS 26.5.2/25F84, 64 GB, h15g
> ANE) or traced to a named external source. The private API can break on
> any macOS update — re-verify the cheap probes before building on this
> after an OS upgrade.

## File index

| File | Scope |
|---|---|
| [`private_runtime_api.md`](private_runtime_api.md) | The private ObjC API contract: frameworks, classes, selectors, lifecycle, execution options, XPC serialization internals |
| [`mil_and_blob_formats.md`](mil_and_blob_formats.md) | The MIL program text that compiles on macOS 26.5 (and the variants that don't) + the binary weight-blob format |
| [`m3_max_hardware.md`](m3_max_hardware.md) | h15g hardware findings: one-fabric concurrency, instance-hint matrix, throughput, handle budget, memory math |
| [`rust_bridge_negative_result.md`](rust_bridge_negative_result.md) | The pure-Rust `objc_msgSend` bridge negative result + the working ObjC-bridge substrate pattern (build.rs + dlopen) |
| [`ecosystem.md`](ecosystem.md) | The external ecosystem map (maderix/ANE, rane, omlx, Orion paper) + the debugging methodology that cracked the failure |

## Why this lives in riir-clippy

The `kernel_opt` domain mines GPU-kernel optimization rules; the ANE is the
third accelerator in the stack's Metal/CUDA/ANE matrix, and any future
ANE-side kernel work (Issue 726's hybrid prefill line, a potential
`kernel_opt` ANE target, or distilling ANE fixes measured-GOAT-style per
Research 489 §7's revisit trigger) starts from this substrate. The knowledge
is hardware/platform reference material, not healer logic — it lives here
because this repo's `.docs/` book is the workspace's densest
reverse-engineering record and this folder is pure reference.

## Provenance

- **Primary artifact:** `riir-ai/crates/riir-poc/src/ane_t0.rs` (the spike —
  contains the pure-Rust bridge kept as the format reference) +
  `riir-ai/crates/riir-poc/objc/ane_t0_bridge.m` (the working ObjC bridge).
- **Issue tracking:** riir-ai `.issues/726` (T0 record, 2026-08-19).
- **Research notes:** katgpt-rs `.research/489` (omlx PR #2756 distillation
  + T0 update), `155` (ANE backend verdict), `377` (ANE architecture).
- **External anchors:** see [`ecosystem.md`](ecosystem.md).

## See also

- [`11_domains/kernel_opt.md`](../11_domains/kernel_opt.md) — the GPU kernel
  mining history this complements on the accelerator axis
- katgpt-rs `.research/377` — the 302-page ANE architecture distillation
  (dispatch floor ~0.23-0.28 ms, 2 MB SRAM working set, int8 formats)
- Bench 053 (katgpt-rs) — the `ane_npc` GOAT FAIL that established the
  dispatch-floor economics this folder's workload regime finally amortizes
