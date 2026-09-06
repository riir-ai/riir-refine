# Research 140 — the Prism fork ternary + verify-batch distill (Batch 116)

**Status:** RECORD (Batch 116 landed 2026-09-06; corpus 532 → 538, A +1 / D +3 / E +1 / N +1, + 3 riders; Plan 089)

---

## Provenance

Source: **PrismML-Eng/llama.cpp `origin/prism` @ `7113f6462`** (Apache-2.0) — the
league's Bonsai-opponent lineage. Prior contact: Batch 96 mined the fork @ the
league pin `e311ed38f`; the fork has since moved **12 commits past the pin**.
Discovery: the 2026-09-06 opponent-watch run (riir-ai
`.benchmarks/rematch/2026-09-06_watch.md`, since 09-04) + the intake triage in
`.distill/001_mining_queue_snapshot.md`.

The vendor checkout stays detached at the league pin per the read-only fork
policy — every quote was extracted via `git show <pin>:<path>` and verified
byte-accurate at the named commit (both mining lanes reported the same
environment fact independently).

Commits mined: `01fd9521c` (PTQ1_0 codec), `d0ce941ec` (multi-column Q1_0 +
popcount path), `7113f6462` (exact-float rewrite + missing per-expert
instantiation), `633168fb6` (fold-transform sharing), `ca02f9b83` (FWHT
wide-block Metal+Vulkan; CUDA provenance `6f1c6908d`).

## Rules landed (6)

| rule | cat | mechanism |
|---|---|---|
| `activation-bitplane-popcount-word-dot` | N | activation int8 bit-planes consumed by AND+popcount (32 weights/instruction-pair, ~0.8 ops/weight/col vs ~3 select-per-weight); extraction-bound diagnosis (bit-test-only removal = 1.7×) |
| `scratch-sizing-shares-dispatch-eligibility-predicate` | D | tail-carved scratch: allocator size-helper and encoder share ONE path predicate; drift = benign-waste one way, OOB the other |
| `byte-owning-threads-non-positional-digit-codes` | A | non-positional multi-digit packings map threads to whole BYTES (element-span ownership = 5× byte reloads); 29.8→136.4 tok/s source-reported |
| `fwht-wide-block-row-per-threadgroup-exchange-ladder` | E | past the N/32 register wall: one row per THREADGROUP, three exchange tiers by partner distance, same host kernel names |
| `shader-shape-cutover-keys-on-resource-block-ratio` | D | subgroup-vs-staged cutover = TWO-TERM disjunction (width ∨ n/subgroup_size > cap); width alone admits the wall on narrow subgroups |
| `same-input-folded-transform-memo-per-build` | D | graph-build CSE of folded transform nodes keyed (input, rotation); one-build lifetime; 36% duplicate transforms, +3.5% pp512 source-reported |

## Riders landed (3)

- `telescoped-digit-coefficients-byte-fma-dot` (entries_gpu_11.rs) += the
  radix-general **mantissa-bound exactness criterion** — `floor(R^k·(b/2^m))` is
  exact iff `R^positions·max_code < 2^24`; the base-3 PTQ1_0 extraction
  (`g_k = floor(3^k·u)`, trit = `g_{k+1} − 3g_k`, telescoped to
  `Σ g_k·(y_{k−1} − 3y_k) + g_5·y_4`) is bit-exact per digit at 3^5·255 < 2^24
  WITHOUT any power-of-two argument — the original rule's exactness reasoning
  does not transfer to odd radices and the naive extension would have rejected
  a measured +29.5–37.8% decode win. Plus the single-digit mid-code two-floor
  form (replaces a lane-divergent recurrence loop) and the float-pipe
  motivation (the integer recurrence spent 4 int ops + a convert per trit on an
  ISA that cannot co-issue integer and float work).
- `small-batch-matvec-gemm-bridge-rung` (entries_gpu_13.rs) += the Q1_0
  multi-column family as a second external instance: register-budget sizing
  (`nr1·SW ≤ 32` staged values), exact-fit nr1=3 selection (the k=2 verify
  case), per-type mv→mm crossover override (Q1_0 holds mul_mv to 16 rows; the
  generic ext path is flat ~2.5 TFLOPS across ALL types — ALU-bound).
- `dispatcher-eligibility-predicate-lags-kernel-capability` (entries_gpu_16.rs)
  += four INVERSE-direction instances in one PR window (claim wider than
  kernel): the never-instantiated per-expert pipeline (SEGFAULT + a test run
  silently truncated to 24 cases "while still printing a clean tail"), the
  head-truncated-grep transcription ("the instantiation set is now derived from
  PQ2_0's rather than transcribed" — this rule's fix-space verbatim),
  supports_op-accepts-every-type (silent-slow), and the missing test
  enumeration ("silently skipped the new type and reported a clean pass").

## Kills (with reasons)

- **Multi-column Q1_0 as a standalone rule** — COVERAGE of
  `small-batch-matvec-gemm-bridge-rung` (same ladder, same dequant
  amortization, same per-count instantiations); rider only.
- **PTQ1_0 extraction as a standalone rule** — the telescoped-byte-FMA rule at
  radix 3; the genuinely new content (mantissa-bound criterion, single-trit
  form) rides.
- **Missing per-expert instantiation as a standalone rule** — coverage-class of
  the dispatcher-eligibility rule (inverse direction); rider.
- **half2 twin kernels** — upstream's own negative (worth 3.5–4.5%, removed
  rather than ship an fp16 accumulator with nothing enabling it).
- **256-entry trit LUT** — no decode gain ("adds a dependent load per byte");
  placement split noted (mul_mm keeps it).
- **Contiguous byte ownership / independent pow3 extraction / N_R0-N_SG
  sweeps / qh work-balance** — upstream tuning negatives, recorded inside the
  byte-ownership rule's regime note.
- **Clean-tail suite truncation as a kernel rule** — test-runner instrument
  defect (the katgpt-rs silent-zero/count-floor class), off-shape for kernel
  decision content.
- **Ratio cutover as B46 coverage / fold memo as B77 coverage / TG-staged FWHT
  as a B96-fwht rider** — all rejected by the lanes' byte-level dedup; each
  landed standalone with compose notes (the boundary-pointer-is-not-ownership
  verdict on the fwht rider is the load-bearing one).

## League-structural record (NOT consumed by our stack)

`01fd9521c` PTQ1_0: ternary at group 128, `qs[24] + qh[2] + fp16 d` =
28 bytes/128 weights = **1.7500 bpw** vs PQ2_0's 34 bytes = 2.1250 (**17.6%
fewer**), bit-identical round-trip on real folded ternary weights (the only
error either format carries is the shared fp16 scale rounding). Upstream TQ1_0
discards one of two 128-group scales per 256-wide scale group = 6.43%
MULTIPLICATIVE weight error — the reason for the distinct type id. With
`7113f6462`'s exact-float extraction the fork claims +29.5–37.8% decode and
17.6% less weight traffic together. Fork policy stands (read-only vendor
binary at the pin, never maintained): a league re-pin is an owner arbitration
with its own fairness-manifest bump. Recorded here so the next league review
sees the shape of a future opponent build.

All throughput numbers in this note and the corpus entries are
source-reported (M5 Pro, llama-bench, interleaved medians) — never cite as
ours.

## Mining-lane process notes

Two parallel lanes (A: mul_mv.metal + ops routing; B: llama-graph fold memo +
misc.metal/vulkan FWHT), both quote-verified at the pin before landing.
Coordinator adjudication accepted 6/6 proposed rules and 3/3 riders; the
corpus generator (Python, `ensure_ascii=False`, needle self-verify) caught
three needle/concept mismatches pre-landing and the compile gate caught the
missing-closing-quote generator bug plus one WGSL-isms-in-Rust pseudocode
snippet (the syn-parse gate doing its job). Count pins walked: tests.rs
(total ×2 + comment, expected array A/D/E/N, sum, grouped, batch-list line),
router.rs expert total, mod.rs (doc line, module decl, entries.extend, 4
category comments, 4 list tails).


## Landing addendum 2 (2026-09-06, the collision salvage — the concurrent M3 session)

A parallel M3 session ran the SAME batch from the same 09-06 watch entry
concurrently (the Issue-665 class, second occurrence at corpus scale). Their
landing `e895da6` (6 rules + 3 riders, corpus 532 -> 538) owns Batch 116 per
push-wins; the duplicate was yielded before commit. The SALVAGE — the content
the landed batch genuinely lacks — landed as this addendum: **1 rule + 2
riders + 2 standing numeric gates, corpus 538 -> 539 (N 35 -> 36).**

- **`swar-lane-widened-base3-dp4a-dot` (N)** — the int8 dp4a CONSUME family of
  the same base-3 codec (the landed batch owns the float-pipe extraction family:
  byte-owning rule + the B98 telescope rider). 16-bit lane widening so the radix
  multiply cannot carry between bytes; answers the packing rule's documented
  "dp4a can't consume base-3" limitation. Riders on the entry: the
  digit-LUT-vs-arithmetic per-path split (contiguous-element contracts take the
  exhaustive LUT; byte-owned inner loops take floors/widened lanes; the 142.1 vs
  143.5 dependent-load negative) + coarse-scale format retention with the FOUR-
  surface new-type registration completeness (traits / quantize dispatch / ftype
  map / row validation — four different failure modes).
- **Guard-side mirror rider** on dispatcher-eligibility: an assert widened to
  admit F16 lost its downstream buffer-layout conjunct (~3.8x wdata overrun if
  reached) — the face their 4 inverse-direction instances do not cover; plus the
  admission-unreachable-guard invisibility note.
- **`batch116_addendum_base3_mantissa_bound_exhaustive`** — turns the landed
  rider's prose criterion (floor(R^k * b/2^m) exact iff R^pos * max_code < 2^24)
  into a pinned gate: exhaustive radix-3 over all 256 bytes x 5 positions, the
  radix-7 within-bound control, AND the radix-11 falsifiable negative (violates
  the bound, MUST diverge — the gate that cannot fail is not a gate).
- **`batch116_addendum_swar_lanes_exhaustive`** — the per-lane exhaustive sweep
  (all 256 bytes x 4 lane positions x adversarial neighbors) + the exhaustive
  adjacent-pair sweep (the only carry channel).

Salvage that was NOT needed: the duplicate session's fold-CSE, ratio-cutover,
multi-column, popcount, scratch-predicate, and byte-owning candidates were all
covered by the landed batch (several more completely than the duplicate's
versions — e.g. the landed fold-memo rule carries the key-completeness
debug_assert the duplicate lacked). The yielded duplicate's full corpus text is
preserved at `target/batch116/mine/` (gitignored, ephemeral) — cited here per
the B115-addendum provenance discipline.
