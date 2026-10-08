# Research 246 — Batch 211 conversion notes: the 4-lead mini-batch (M3)

**Status:** RECORD

The 2026-10-08 intake row (`3091c00a`) + remediation (`d5178621`) + upstream
sweep (`63cacd06`) staged four conversion leads; this batch converted all
four in one session. What follows is the conversion-time verification record —
every quote re-fetched and read at the pin THIS session (patch + PR body via
GitHub API/fetch), not inherited from the intake rows.

## 1. The i8mm arm validates; the alias-back lifts (PrismML #290 @ 4fbda1292)

The Batch-204 ISA-boundary arm carried its third arm (i8mm 8-bit-lane decode +
SMMLA 2×2 tiling) from a THIRD-PARTY branch (karusrus arm-neon-ptq1_0,
"unverified by the PR author who had no i8mm hardware"). #290 is the in-tree
resolution and it CONFIRMS the arm: arch-fallback.h's four alias-back lines
are DELETED; the validated kernel ships in arch/arm/quants.c.

The new mechanism atom is the division-free 8-bit-lane trit identity:
`((q + (q >> 1)) >> 1) >> 6 == (3q) >> 8` for every byte value (checked
exhaustively in-source). The halving add `vhaddq_u8(q, q>>1)` computes
`(q + (q>>1)) >> 1` in-lane — no u16 widening anywhere. The transferable law:
**a packed-digit decode's lane width is chosen by the CONSUMER's native
width, never by the multiply's overflow bound alone.** dp4a-class consumers
need widened lanes (the overflow bound is real there); sdot/SMMLA consume
int8 lanes natively, so the widening was never the load-bearing half.

The measured A/B lands on the SAME device family (Snapdragon 7 Gen 4) that
produced the plain-NEON negative — pp64 1.22→2.33 t/s, tg16 1.09→1.31 t/s vs
generic — so the pair isolates the i8mm arm. Exactness disclosed: greedy
output matches the generic path.

## 2. The CUDA FWHT ladder lands upstream (llama.cpp #29100 @ 7d9305fb)

The B116 Metal ladder's CUDA side is now the shipped upstream form
(`fwht_cuda_block<N, 256, T>`). Two decision contents converted:

1. **The cross-vendor width-gate datum** — three gate shapes, one crossover
   family: Metal post-#29602 = 512–8192 TG-gated unconditional (64–256 stays
   simdgroup); CUDA = warp ≤512 / block 1024–8192 (with an honest HOLE at
   512–1023 where neither CUDA kernel serves and the generic matmul path
   runs — a hole Metal does not have); the tetherto twin = arch-keyed
   rows==1 && Blackwell at 512.
2. **The superset-admission fall-through law** — the width-blind shared
   predicate (`ggml_cuda_op_mul_mat_use_fwht`) is a deliberate superset:
   "any width it admits that ggml_cuda_op_fwht can't serve already falls
   through correctly to the cuBLAS path." This COMPOSES-WITH-TENSION the
   B176 shared-predicate law; the reconciliation: a superset admission is
   sound IFF the dispatch's refusal path is a safe fall-through to a correct
   generic kernel — never an assert/refusal-after-admission (B176's crash
   shape). Payoff: admission stability across kernel-side width extensions
   (#29100 added four widths with zero predicate edits).

## 3. The dtype axis of the aliasing conjunct (llama.cpp #28205 @ 6ed24bce)

One-line diff, one PR body sentence each. The webgpu flash_attn supports_op
overlap guard rejected differing K/V types only when BOTH were non-quantized
(`!q(a) && !q(b)`), which admits every MIXED pair — exactly the Q8_0/F16 case
it existed to reject (the shader defines V as K when KV overlaps, so a dtype
divergence is undefined access). The fix negates the COMPOUND:
`!(q(a) && q(b))` — differing types rejected unless BOTH quantized.

The law (rider on the B208 FA admission rule, which owns the WIDTH conjunct):
an aliasing admission conjunct must cover every representational axis the
consumer reads through one view — width, stride, and now dtype — and a guard
whose escape hatch is a compound condition must negate the compound, never
distribute it. De Morgan is load-bearing in guards.

## 4. The register-partition law (bevy #26042 @ a144d280 + rust #163916 @ aaa57c9a)

The first rust_perf RULE of the batch (layout 13→14). The PR body's mechanism
paragraph is the cleanest statement of the store-to-load-forwarding failure
class this corpus now owns: EntityLocation loads as (4, 8, 4) registers but
is stored 8+4+4; the first store chunk joins a field with HALF of the second
register, so LLVM round-trips both registers through the stack and the 8-byte
read spans two pending stores — forwarding fails. `repr(align(8))` tips the
partition to 4+4+8 and the join happens in-register. The 16-row table is
honest on both ends: world_entity/50000 x0.26 (259.52→233.33→66.48 µs), the
get_entity_mut_slice/20 x1.49 regression the layout change caused is
RECOVERED to parity (102.2→152.1→100.9 ns), and one shape
(observer_custom/10000_entity) stays worse with the reason unstated — the
attribute removes ONE identified stall, it is not a universal win.

The rider face (#163916, rustc) is the SAME lever opposite direction:
usize→u32 drops PartialRes's alignment, shrinking the (NodeId, PartialRes)
map pair 32→20 bytes ("We store a *lot* of those") — max-rss −102 MiB (1.84%)
on aws-sdk-ec2. One law, two faces: field width/alignment is ONE decision
with TWO downstream costs — the register partition (codegen) and the
container footprint (size at N=millions). Mismatch in either direction is a
defect.

## Conversion integrity

- Every quote in the landed text was read at its pin this session; the four
  sources fetched: two .patch files (PrismML, llama.cpp #29100), two PR
  bodies + one patch via the GitHub API (#28205, bevy #26042, rust #163916).
- Riders are count-neutral; count pins moved only for the rust_perf rule
  (183→184; layout 13→14; NotBenchable 84→85).
- Score-bench + freeze regen NOT owed (zero clippy_lints-corpus delta, P30).
- Landed under the live Plan-337 census (nice-19): the battery is
  correctness gates, no GOAT bench owed — the cargo-quiet precondition's
  measurement-validity intent is satisfied by the census's yielding posture;
  the census pays wall-time only (bounded CARGO_BUILD_JOBS=8, nice 5).
- The M3-Max GOAT for the new rule is a candidate for a later quiet-box slot
  (the B204 → Plan 204 T11 pattern: rule lands source-reported, the GOAT
  prices the mechanism later); queued in the snapshot row, not owed here.
