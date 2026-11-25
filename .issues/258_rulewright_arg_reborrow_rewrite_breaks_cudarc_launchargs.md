# Wrong heal: `&*x` → `&x` rewrite broke the CUDA-only cudarc LaunchArgs (6× E0277, 4090 lane)

**Status:** OPEN
**Claimed:** zed-a4d02c1f@amo, 1773519057, expires 2026-11-27T03:00Z

## The wrong heal

Batch `aad54860` ("rulewright mechanical batch — 253 kept edits across 113
files, compile-gated") rewrote `.arg(&*grad_q)` → `.arg(&grad_q)` (6 sites,
2 hunks × 3 args) in `riir-train/crates/riir-train-gpu/src/cudarc_backward_kernels.rs`.

With `grad_q: &mut CudaSlice<f32>`, `&grad_q` is a `&&mut CudaSlice<f32>`,
which does not satisfy cudarc's `LaunchArgs`/`DeviceRepr` bounds:

```
error[E0277]: the trait bound `&mut CudaSlice<f32>: DeviceRepr` is not satisfied
  = note: required for `LaunchArgs<'_>` to implement `PushKernelArg<&&mut CudaSlice<f32>>`
```

Result: `riir-train-gpu` lib **unbuildable on the CUDA lane** — caught only
when the 4090 probe cell compiled (job `01a20fc6-3ad8-71c1-80a0-76dc37a6e9a8`,
exit 101, 2026-11-25T00:36Z; the 709 T3 eval cell also died in the same
window). The mac compile gate cannot see the break: the module is
`#[cfg(all(feature = "ternary_backward_cudarc", not(target_os = "macos")))]`
— the gate's macOS posture compiles the module body to nothing.

## Why the rulewright should refuse this class

The rewrite `&*x` → `&x` is semantically identity ONLY when `x` is an owned
value. When `x` is already a reference (`&mut T`), `&x` re-references
(`&&mut T`) while `&*x` reborrows (`&T`) — different types, and for
kernel-launch arg lists the difference is a compile break (or worse, a
silent D2D-copy change if a match on `&&mut` ever resolved). The proposer's
pattern should either (a) refuse when the operand's syntactic type is a
reference (not statically knowable pre-typeck — so the safe posture is
refuse-on-uncertainty), or (b) restrict the rewrite to sites where the
operand was BOUND as an owned binding in the same body (`let x = ...;`).

Adjacent evidence the batch's other `&*` rewrites were lucky rather than
right: `b.arg(&*idx_dev)`, `blas.gemm(gemm_sq, &*p_cur, &*p_cur, p_sq)`,
`.arg(&*p_cur)`, `.arg(&*p_sq)`, `.arg(&*pw2)` in the same commit's
diff — those survived only because their operands happened to be owned
`CudaSlice` bindings, not because the rewrite is sound there.

## Remedies carried

1. riir-train fix landed: `740cbebc` (develop) — restores the exact
   working-since-`2d0eab54` reborrow spelling on the 6 grad sites.
2. This issue = the rulewright improvement-queue row. Proposed rule name:
   `arg-reborrow-to-ref-not-identity-on-ref-typed-operand` (or fold as a
   divergence class: never rewrite `&*x` where `x`'s binding form is not
   proven owned).

## The blind spot that let it ship (record, not the rulewright's fault)

macOS compile gates never see `not(target_os = "macos")` module bodies.
The standing lane for this class is a non-macOS typecheck
(x86_64-pc-windows-msvc died in a cc build script on this box —
`ml64.exe` absent — so the 4090's own build remains the only working
gate). A future windows-gnu typecheck lane would close this; filed here
as context for the lane discussion, not as a refine deliverable.
