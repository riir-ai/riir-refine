# Research 257 — Batch 227: the lithos-metal wave-2 pack (2 rules + 2 riders)

**Status:** RECORD — landed 2026-10-13 ~10:1x +07 (m3-max idle loop; Plan 229; corpus 801 → 803)

## Source (all quotes verified verbatim this session)

lithos-ai/lithos-metal @ repo pin `5485ad6` (HEAD main at the clone), Apache-2.0 — the four
rule-grade PRs staged by the 08:2x intake envelope, read in FULL from the fresh `.raw` clone
(`.raw/lithos-b227`, rm at close):

- **#35** `7781e93` "kernels: the matrix tile on Apple9 GPUs (M3, M4)" — gemm_tile.metal +264,
  3 gemv arms, monolith wiring, contract tests.
- **#36** `087db24` + `182d016` + `4effa4a` — the exact-prefill-attention trio: kernel chunking +
  the boundary refusal + the M3 Ultra serving posture.
- **#37** `5842639` "prefill: keep a large scratch slot for a value of its size" — arena.py +13.
- **#38** `258ef6e` "formats: decode FP8 E4M3 two codes per half2" — fp8_e4m3.py +14.
- WATCH (per the envelope grading, not converted): #39 kernel-choices sidecars, #48 mlx-4bit M5.
  NULL: the ollama-serve series (#43), CI/runner infra (#44-46).

## The conversions

### NEW A — `coop-tensor-layout-measured-per-chip-family` (#35, entries_gpu_90.rs)

A SIMD-group cooperative tensor's register layout is a per-chip-family hardware contract
measured with the device's own introspection (coop_layout), never assumed inherited: Apple9
(M3/M4) lays cooperative tensors as 8×8 matrices ("a thread holds the reduction slots kr + 8*jj
of the row pairs nc + e + 8*s (measured with coop_layout), so every product was wrong there and
the autotuner kept the tile off, leaving every T > 1 projection on the shader GEMV — a DSpark
verify of 8 rows at 8.8x a T = 1 pass"). The law's sharpest edge: **a correct autotuner is not a
correct pipeline** — the defense worked (the tile was kept off) and the cost was still an 8.8×
regression on every multi-token projection. The repair ports the LAYOUT (COOP_A9: slot
permutation over twice the rows / half the columns so the decode/pack machinery is unchanged,
epilogue reads row pairs, row GEMVs read four slots 8 apart, TK≥128 for 4-bit), with in-family
compile guards (`#error "two token blocks, staged weights and the split fill assume the Apple10
cooperative layout"`) and skip-with-reason test discipline for the layout-gated paths. Measured
(60-core M3 Ultra, source-reported): NVFP4 8 rows 8.80×→2.50× a T=1 pass (32 rows 5.81×); 27B
prefill of 43 tokens 11.8→0.65 s.

Kin: mma-fragment-layout (B221 rider) is the within-ABI coupling; this is the cross-family
contract — the same class one level up.

### NEW D — `exact-pass-rows-take-their-sub-pass-chunking` (#36, entries_gpu_90.rs)

A shape-dependent scheduling parameter (chunk width derived from the pass's context) silently
breaks an N-exact reproduction promise ("a 512-row prompt pass summed a row's keys in a
different order than the 128-row pass N-exact promises to reproduce"). Three faces + the
admission: (1) per-row sub-pass chunking (EXACT_SUB — each row uses the chunking of the sub-pass
shape it belongs to) + the cache-append ownership guard (`rg == 0 && key >= qpos0 + s0`);
(2) the merge restores per-row context (`ctx_t = ctx - T + min((t / EXACT_SUB + 1) * EXACT_SUB, T)`);
(3) the boundary law — exact chunks are MULTIPLES of the sub-pass size, refused loud at config
("a 192-row chunk would put rows 128-191 of a 384-token prompt in a sub-pass of the wrong
context"); (4) the fold-chain admission — the per-rows variant is cloned only when the whole
decode+merge chain is chunked (STEP_STATE=1, no DRAFT/LM_MODE/STEAL/FIXED_CHUNK/ADAPTIVE_CHUNK;
fixed-chunk kernels and shared workspaces left alone). Serving posture: M3 Ultra 512-row exact
passes, 2846-token Qwen3.8-27B prompt first token 9.2→8.6 s; **1024 rows measured no better**
(the honest plateau, in-rule).

Kin: `batch-composition-invariant-grid-early-return` (gpu_14) owns order-independence BY
CONSTRUCTION via the grid; this repairs a broken promise by restoring per-row parameters — the
derived-parameter face. Compose, never merge.

### RIDER on B223 `attention-partition-two-knob-shared-partial-workspace` — the ARENA-ADMISSION face (#37)

The cross-layer shared workspace is defeated by the arena's free-slot policy: "a later small
value took their slot and the next layer's partials opened a new one. A Qwen3.8-27B prompt
program at 32K context held five copies of the partial workspace: 18.7 GB of scratch for
128-row passes on the M4 Pro and 34.9 GB for 256 rows, past its working set (a GPU page fault).
A value now takes a free slot at most twice its size: 5.8 and 9.1 GB." The one-line bound:
`s['end'] < start and s['size'] <= 2 * size`. Sharing policy (host rule) and slot admission
(rider) are one law's two halves.

### RIDER on B207 `minifloat-normal-code-shift-bias-diff-decode` — the LANE-PAIRING face (#38)

The decode family's fifth member (gather-LUT → field-arithmetic → bit-placement → SWAR →
pairing): when the decode consumer is ALU-bound, pair two codes per half2 (even bytes → one
half, odd bytes → the other) — "Placing the sign and magnitude of two codes in a half2 and
correcting the exponent bias in float decodes every finite code to the same value as before, at
fewer instructions: the M3 Ultra's T = 1 FP8 projections 514-562 -> 672-675 GB/s, the M4 Pro
+2-4 %, the M5 Max +3-6 %." The trigger is the BINDING RESOURCE (a bandwidth-bound consumer
gains nothing); the default flipped on the measurement (FP8_DECODE 0 → 2, "0 and 1 remain").

## Dedup check (Protocol A step 2, done before authoring)

- #37 slot-size-class: ZERO owners (the tight grep found no slot-admission-by-size law; the
  B223 host owns the sharing policy, the rider adds the allocator half — same failure surface).
- #36: the order-independence family owns construction-level invariance; no owner of the
  derived-parameter break + sub-pass restoration; `pick_chunk`/`sub-pass`/`N-exact` all zero.
- #35: "cooperative tensor" appears only as consumer prose (gpu_09/60/66); `coop_layout`,
  `Apple9`, `8 x 8 matrices` all zero. NEW.
- #38: pairing is the throughput half of B207's own placement decode — rider, not new rule;
  B116 (SWAR base3) and the B218 SWAR rider are different mechanisms (compose).

## Gates (ALL GREEN)

- **G1**: `batch227_rules_parse_and_needles_landed` — 2 rules × (parse both halves + 9/8
  verbatim needles) + both rider needle sets (7 + 7) — PASS.
- **G2**: kernel_opt lib **2948/0** (2947+1 ✓; 1 ignored = the documented freeze-regen e2e);
  default lib **2505/0** (unchanged ✓). Corpus 801→803 at ×5 pin sites (build corpus, Plan-003
  sum, round-trip, batch116 running pin, router expert total) + per-category A 163→164,
  D 187→188.
- **G3**: clippy `-D warnings` ×4 postures clean; retrieval floors HELD (kernel self-variants
  100.0% ×3, blended 91.2/94.7/95.3 — no re-pin; 21/21 tests).
- **Freeze/score-bench NOT owed** — zero clippy-corpus delta (P30).

## Session lessons (process)

- **The 148-line pin replacement**: replacing the batch116 running-count assert by searching
  for `");` swallowed ~148 lines / 30 test functions because the message's true terminator was
  far past where a naive search stopped — recovered by `git checkout` of tests.rs and redoing
  all edits with **unique-anchor `count==1` verification before each replace**. The lesson for
  the next corpus batch: never free-search for a message terminator in this file; anchor on
  unique message PREFIXES and verify anchor uniqueness first.
- The write_file escaping trap (all `\"` delimiters escaped) cost one compile cycle on
  entries_gpu_90.rs — the fix pattern: `: \"` → `: "`, `\".to_string()` → `".to_string()`,
  inner quotes stay escaped. Fixture style: Rust-lookalike stand-ins (usize/let/fn), never
  C-style — the parse gate enforces it.

## Provenance

- Quote verification: the fresh clone walk covered every commit in full (the diffs were small —
  4 PRs ≈ 450 lines with tests); no patch-endpoint needed, the clone IS the pin (HEAD == 5485ad6).
- Count pins: 803 at all five sites verified by the suite (any missed pin fails the run — the
  two missed pins WERE caught by the suite, the system working as designed).
- `.raw/lithos-b227` removed at close.
