# Research 072: Batch 64 — the ARM Neon / CPU-SIMD / bounds-check pool (2 online-search candidates)

**Status:** RECORD — Batch 64 landed 2026-08-19 (corpus 67 → 69; 1 ParetoDominant + 1 annotated Mixed, both measured). **x86 re-gate 2026-08-19 (4090 box, quiet): `debug-assert-length-hoist` regime CONFIRMED cross-arch — 1.00× tie @1K, 0.92× @100K (milder than aarch64's 0.67-0.71×; the article's x86 headline win does not reproduce on this harness shape).** Corpus annotation updated in the same commit.
**Source:** the 2026-08-18 queue-item-11 pool (recorded in the mining snapshot with provenance + dedup + caveats; plan-first via Plan 010).

## Measured results (M3 Max, release, alloc-det stable)

1. **`loop-order-innermost-contiguous`** (simd 6→7): **ParetoDominant 7.14× / 11.32× / 13.80×** at n=64/128/256 — BETTER than the source's 9× headline, and the win GROWS with n (the column-stride cache damage compounds; the contiguous walk improves as arrays exceed cache). Sources: Arun Nagaraj matmul-simd (x86 measured, ISA-independent — memory hierarchy). Distinct from B62's `transposed-weight-register-gemv` (that repacks the DATA for GEMV; this reorders the LOOPS over an unchanged layout — the no-repack general nested-accumulation case), `simd-chunk`, `layout-reorder`, `soa-hot-field`. Float sum-order changes → rel-error gate (not bit-identity) — the plan's caveat honored.
2. **`debug-assert-length-hoist`** (complexity 14→15, annotated): **Mixed regime-split — 1.13× at n=1K / a STABLE 0.67-0.71× LOSS at n=100K** (3-run verified). Two honest findings beyond the pool's expectation:
   - **The scope finding (measured on the first harness shape)**: an assert over a DATA-DEPENDENT index array (`idx.iter().all(|&i| i < rows.len())`) can NEVER enable elision — the compiler cannot propagate a constraint through runtime index values — so the swap there is pure cost (measured 0.74-0.80× dominated). The rule's applicability is parameter/range-pinned indices only (the canonical `assert!(first + count <= rows.len())` feeding an induction loop).
   - **The large-n inversion**: even in the canonical propagatable shape, the assert side measures a stable ~45% LOSS at n=100K on aarch64 (mechanism not fully microarchitecturally explained — recorded honestly as such; the article's own aarch64 number was 1.03×, the 15% headline was an x86 unrolling secondary effect). The annotation carries the regime split + the measure-first guidance.
   The rule's VALUE: stopping the counter-intuitive release-mode pessimization (`debug_assert!` feels free — its vanishing drops the constraint elision needs) WHILE warning that the fix itself is shape- and scale-dependent on aarch64.

## Process record

- Phase 0 dedup re-verified against the CURRENT 67-rule corpus (the pool was swept at 65): both absent; B62's lead had landed — the no-repack scoping confirmed as the distinct axis.
- The pool's recorded DROPS stayed dropped (cache blocking — architectural; `& mask` bounds — correctness-first excludes; iterator retrofit — style; `#[inline(always)]` — cross-function; SME — no M4 hardware on this box).
- Generator lessons: two anchor aborts on STALE text (the B62-edited regions drifted from my memory of them) — the fix-by-actual-read pattern each time; the harness-in-file is the ESCAPED string form (raw-text anchors can never match it — replace via index-of + json.dumps re-embedding, or line numbers).
- GOAT: corpus tests 15/15 bench-state + 607 default + 659 latent; retrieval floors HELD (blended 101: 92.1/96.0/97.0 identical); clippy 0 all-targets both states; alloc-det stable ×3 (the runner's internal repeats) + the 0.67× stability triple-run.
