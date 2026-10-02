# Research 141 — Batch 117: the antirez/ds4 Metal tensor-prefill delta distill

**Status:** RECORD (batch landed 2026-09-06; Plan 090; the intake lead
"2026-09-06 NEW — ds4 Metal tensor-prefill pair" consumed)

## Verdict

**5 RULEs + 8 riders**, corpus 539 → 544 (L +1, A +1, C +1, M +1, F +1).
Source: antirez/ds4 `main` @ **`9ab7053`** (MIT) — fresh clone into `.raw/ds4`,
removed at landing. The Batch 97/98 pin was `110afdd`; this is the delta batch
(~14 commits), the first mined after the opponent-watch coverage gap was closed
(riir-ai `9639e7c50` added ds4 + oMLX watch rows; ds4 was the one league-relevant
repo with zero coverage).

## Rules landed

| rule | cat | commit | mechanism (one line) |
|---|---|---|---|
| `routed-activation-pack-once-shared-operand` | L | `2ea3839` | pack the scattered routed activations ONCE per stage into a dense converted device tensor; all consumers (gate GEMM, up GEMM, all M-chunks) read it plain; f32→half exactly once at the pack; bounded grow-only scratch + derived 256 MiB refusal; bit-exact by conversion-site-only movement (memcmp-pinned upstream) |
| `device-affine-scratch-admission-with-pool-fallback` | A | `b42682f` | three-check admission tuple (ptr ∧ device-equal ∧ capacity) for persistent decode quantize staging, full ladder retained (env static → pool); device captured at SET time — the multi-GPU illegal-access guard; pool-alloc-inside-captured-graph is the motivation |
| `batch-verify-rows-single-dispatch` | C | `5806d09` | the 2–6-row speculative verify block in ONE launch; partials written straight into the TP exchange slab; the cross-rank combine epilogue hoisted out of the branch and byte-identical for both schedules — lockstep by construction |
| `row-aware-indexing-reuses-static-single-token-kernel` | M | `5806d09` | widen the existing static kernel's grid mapping (z → (token, slot)) + per-token base pointers and call the static impl VERBATIM with the axis rebased — reuse-instead-of-twin |
| `dense-fast-path-bounded-by-selector-coverage` | F | `0e9cc2d` | dense-substitution is legal exactly through `top_k + anchor_pool − 1` — the selector's own coverage, never the work-window/ctx cap; converts a tolerance into a proof while enabling the bounded GEMM |

## Riders (8)

- **B97** `tail-simdgroup-cull-barriers-preserved` += the duplicate-pad-at-materialization
  complement (tensor-pipeline consumers want a plain dense operand; `ri = min(...)`
  clamps duplicates into the pack; the scatter writes only real pairs — padded tiles
  computed-and-discarded).
- **B35** `mask-sentinel-matches-reduction-identity` += TP non-owner additive identity
  (unowned ranks write 0.0f to scattered outputs; pack skips unowned experts entirely)
  and += selector-padding sentinel (−1/out-of-range ids masked at the score stage with
  the max identity; the all-masked-row `isfinite` guard rewrites the sentinel before the
  sum phase — `expf(−INF − −INF)` is NaN).
- **B23** `hardcoded-fast-path-jit-specialization` += derived BYTE-SIZE static pins
  (2176/4456448/1088 — byte sizes imply the 32×17 MXFP4 block layout, catching
  quant-format/stride drift a dims-only check misses) and += the topology-axis widening
  contract (measured (device, quant-pair) schedule extends to a new topology only when
  the machinery is shared; invariance stated: "No sampling decision is changed by this
  dispatch").
- **B57** `smem-slot-role-reversal-needs-barrier` += single-pass two-phase softmax +
  LayerNorm twins (read `reduce[0]`/`scratch[0]` → barrier → phase-2 writes the same
  slots), fixed across FOUR backend implementations in one commit; 20×-repeat memcmp
  determinism harness as the race detector.
- **B58** `captured-graph-bakes-buffer-addresses` += the remove-the-alloc complement
  (the pool-alloc-inside-capture hazard: the alloc node re-addresses at replay while
  the kernel's pointer was baked at capture).
- **B103** `shape-class-keyed-capability` += clamp-to-stay-on-bounded-gemm (slice > 256
  → 256 so the selected-row GEMM keeps the batch on the fast path instead of silently
  falling to the scalar fallback).

## NOT landed (recorded honestly)

- **MXFP4 packed-prefill arm** (`5806d09`'s `kernel_mul_mm_id_mxfp4_mpp_packed`
  enablement): the source's own QA refuses attribution — "adjacent controls suggested
  roughly 3-5% faster prefill, but both paths slowed during sustained runs; 16K results
  were variable", and the pre-existing control-path nondeterminism flag
  ("investigate reproducibility before attributing a difference to a new kernel",
  max 0.163/0.167 at 8K/16K, unchanged argmax). Non-uniform: 2K +9%, 4K +2.8%,
  8K −2.3%. Template-instantiation of an existing generic = no decision content.
- Seed-schedule TP widening as a rule (policy, not kernel decision content — the
  portable half is the B23 rider above).
- qk_rope zero-width guards, comparator/`-ffast-math` instrument hardening, TP
  transport record-keeping — NO-RULE (process/instrument, not kernel decisions).

## Measurement-honesty notes (carried in-rule)

1. The `5806d09` QA table (+11–23% TP DSpark) CONFLATES three changes — batching +
   seed schedule + MXFP4 arm. Cited whole-commit only.
2. All numbers source-reported (M5-class Apple silicon) — never ours.
3. The pack gain is modest (+1.7–3.2% Q2 arm) and the source's own gate boundary
   ([512, 4096] tokens) is gate-asserted, not bench-derived — stated in-rule.

## En-route process findings

- **19th count-pin site**: the Batch 116 test carried its OWN corpus-count pin
  (`batch116_rules_present` asserting 539) — missed by the pin walk until the
  kernel_opt lib run red it. The count-pin-site inventory keeps growing; grep
  `entries.len()` across the whole tests file, not just the named pins.
- **B22 full-string-pin lesson re-proven**: `batch23_attention_cubecl_concepts_covered`
  pins the ENTIRE concept string of `hardcoded-fast-path-jit-specialization` (all
  riders concatenated). Appending the Batch 117 rider required regenerating the
  needle (+996 chars). Riders on full-string-pinned rules must regenerate the pin
  in the same commit.
- Quote line-drifts from the lanes corrected by coordinator spot-verification:
  pack kernel `kernel_mul_mm_id_mpp_packed` at moe.metal:9015 (not 8987 — that is
  `kernel_moe_pack_rhs`), the packed `tB` read at :9045 (not 9051), sum6 `out[]`
  at :6693-6695 (not 6693-6694).

## GOAT

- batch117 gates 3/3 (rules_present / concepts_covered / pseudocode_parses — the
  parse gate forced 3 entries' pseudocode to Rust-shaped rewrites: no `kernel fn`
  Metal syntax, no pointer arithmetic, no `...` varargs)
- kernel_opt lib **1389/0**; default lib **1086/0** unchanged; combined
  kernel_opt+latent_retrieval **1504/0/2i** with the retrieval-precision floor
  gate PASSED explicitly (15.8s run)
- clippy 0: kernel_opt lib + kernel_opt `--all-targets` + default `--all-targets`
  (one doc-markdown backtick fix en route — `DSpark` in the new `//!` header, the
  B37 lesson's 14th strike)
- score-bench run #45 FLAT (heal 96.2 / created 1.9 / safety 100 / decline 100)
- count pins walked: tests.rs corpus count + 5 category counts + sum + grouped +
  exhaustive doc + the batch116 test's own pin; router.rs expert total; mod.rs
  exhaustive doc + 5 member lists + 5 category headers.

Batch 117 / Plan 090. Pin `9ab7053` — `.raw/ds4` removed at landing.
