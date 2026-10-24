# Research 314 — Batch 276: the mlxfast-qwen38-125b-a6b conversion (kernel_opt)

**Status:** RECORD — landed 2026-10-25 (Plan 280; the staged MLX pair part 2; B276 complete)

## Source

**Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ `59160bd`** (MIT, re-verified at the root LICENSE
this session — Layr Labs 2026; THIRD_PARTY_NOTICES.md credits third-party model artifacts and
Swift packages separately; the mine was the repo's OWN engine tree `Runner/FastModel/*` +
`Vendor/`-adjacent engine kernels, never the third-party vendor payload). Qwen 3.8 Flash Next
125B-A6B MoE (6B active); 65 promoted submissions; stock head 1.0 tok/round on every promoted
row = ZERO speculative decoding — the entire 2.11× record is kernel-side. A fresh measured-config
surface for the MoE corpus families (a LARGE MoE, ≠ our league 27B dense).

## THE PROVENANCE CORRECTION (the batch's headline finding)

The B275 landing recorded an honesty finding: "the qwen38 enumeration quotes FAILED HEAD
verification 24/37 — every failing text exists only in mid-challenge submissions later removed
(git log -S confirmed)". **That premise is REFUTED.** This batch's adjudication ran git log -S
on a distinctive fragment of every one of the 24 failing blocks: **every fragment returns exactly
ONE commit — the ADD. There are no removal commits. All 24 leads survive at HEAD.**

The 24 failures were the verification INSTRUMENT's three false-negative classes:

1. **Nested-bullet indent (15 blocks)** — quotes inside markdown bullet lists carry a 2-space
   prefix per line; the per-line substring check (`l in c`) fails for any quote of column-0 /
   low-indent file text while the text is byte-present.
2. **Label cascade (8 blocks)** — labels with no fenced block of their own grabbed the NEXT
   fence — often a different section's quote, sometimes checked against the wrong file.
3. **Enumerator reflow (2 blocks)** — the quoted form never existed in git (a hand-rejoined
   line, a re-wrapped comment); the file's own wrap is byte-stable since the add commit.

Practical consequence: the instrument (`verify_quotes.py`, ephemeral in `.raw/b275/`) needed
indent-stripping + label-anchored fence lookup. Any future enumeration instrument must verify
the label→fence binding, not just the line membership. The B275 snapshot row's claim is
corrected by this note; the correction rides the B276 landing row in the same file.

## What landed

**Corpus 953 → 955** (B 108→109, A 190→191; `entries_gpu_124`). Dedup gate verdicts: 2 NEW /
12 RIDER / 2 DROP (of 16 surviving leads), plus the two late-adjudicated leads (R1/R2) as riders.

### Own rules (2)

- **B `kahan-endpoint-peel-dead-flops-exact`** — the exactness-preserving FLOP elimination
  inside a compensated accumulation loop: the LAST i's compensation is never read after the
  block (two subtractions dead), the i == 0 step peels with the sign-of-zero case pinning the
  kept `0.0f +`, and the loop bound is untouchable because the unroll outweighs the two saved
  ops (measured −3.4 %). Kinship: `split-sign-accumulators-defer-subtraction` (different
  mechanism), `issue-latency-priced-mad-cost-model` (issue-bound as regime label).
- **A `host-context-mirror-fenced-drain-avoidance`** — a fenced host-side mirror of the rolling
  n-gram context window (seeded once from the authoritative device state, advanced from the
  tokens the step itself feeds, fenced by attention offset + state-layer identity + window
  length; dirty is the DEFAULT; invalidate-on-nonconforming beats verify-on-use) removes the
  per-call `asArray()` drain; the step's only device sync is the fed token.
  **The staged dedup grep MISSED the closest sibling** — `host-mirror-shared-scalar-advance-
  one-time-verdict` (rust_perf entries_10) — caught by the deeper sweep; the rule carries the
  mandatory `Distinct from` kinship sentence (one-scalar uniform-delta + one-time verdict vs a
  per-token-varying window + structural identity fences).

### Rider faces (14 count-neutral, 12 hosts)

MEASURED ROW-COUNT OPTIMUM + GATHER-QMM INPUT-STAGE (gpu_01 ×2 faces), PREFETCH-ROW-COUNT SIGN
FLIP (gpu_56 — the prefetch-benefit × row-count sign flip: −0.5 % @2 vs +8.4 % @4), HPT +
BATCHED-COMPONENTS EPILOGUE (gpu_01), INJECT-4 LATENCY-CHAIN UNROLL (gpu_01), ROUNDING-REGIME
ADMISSION (gpu_07), GPU-BUILT BF16 SIGMOID TABLE (gpu_57), IN-KERNEL DEFERRED-STATE JOURNAL
(gpu_08 — journal + replay both inside the consuming kernel), EXPERT-RUN TILE OWNERSHIP
(gpu_18 — run-aligned tile boundaries + the atomics-free two-pass prefix-scan builder),
BF16-STORAGE FP32-MATH (gpu_22 — bf16 operand read through the float-GEMV walk unchanged),
DECODE-STEP BUILD-CHUNK (gpu_50 — asyncChunk every 12 layers, the first chunk right after the
step's one host sync), SIMDGROUP-PARTITION SPLIT-K (gpu_67 — with the honesty note: the shipped
default is `split = 5` while a stale source comment says "four partitions"; the comment is named
stale, never quoted as the value), PREFILL M-TILE RESIDENCY (gpu_117 — the prefill dense-GEMM
M-tile halving 80 → 160 threadgroups, bit-exact), SIMD-NATIVE PAIR SELECTION (gpu_20 — the
unpacked simd_max/simd_min pair with the 0xffffffff sentinel electing the lowest index on ties),
LOW-RESIDENCY PIPELINE (gpu_75 — 40–80 resident threadgroups ⇒ software pipelining is the only
latency hider).

### Honest drops (2)

- **N12 `declaration-only-header-unreached-instantiation`** — authored-grade NEW C+ at dedup:
  real but MLX-Swift-host-specific (source-string custom-kernel path), size-not-time evidence,
  single-vendor cost model — the Batch-30 profile. Dropped.
- **R13 constant index-table memo per shape** — DROP-ADJACENT: covered outright by rust_perf
  `mask-memo-hoist-to-owner-lifetime` (a pure-function-of-geometry index table memoized at the
  owner's lifetime). The NSLock and the per-layer host copy are implementation notes.

## Authoring shape

Four slices with disjoint write sets: slice-124 (the 2 own rules, one agent, rustc-validated
fixtures before landing), rider slice 1 (gpu_01 ×4 faces + gpu_56 + gpu_07 + gpu_57), rider
slice 2 (gpu_08/18/22/50), rider slice 3 (gpu_67/117/20/75). Every face: deterministic python
splice anchored on the unique `rule_id: "<host>".to_string(),` line + the next `gpu_concept: "`
line, insertion before the line-final `".to_string(),`; uniqueness asserted before and after;
every quote grep -F-verified at the pin; intro commits derived via `git log -S` per face
(`1b0ee62`, `9f6a0ac`, `7df55c7`, `709404c`, `8981cef`, `d51c662`, `b054c53`, `bff43f7`,
`f70c17d`, `6dc576e`, `f8e8ad5`, `0594455`).

Count pins via ONE shape-asserted python pass (the B265 law): **29 corpus-count sites in
tests.rs** (10 single-line + 17 multi-line batch re-pin witnesses + 2 needle-gate sites — the
needle-gate sites needed BOTH their message and their value line; v1 of the pass fixed only the
messages and the gate caught the values), router.rs expert-coverage assert, the Plan-003
per-category expected table (A 190→191, B 108→109), the Plan-003 totals + grouped-corpus pins,
2 category header comments + 2 category lists in mod.rs. Plus the **full-string concept pins**:
the batch18/batch23 `concepts_covered` tests pin the EXACT full gpu_concept text of two hosts
(row-tile-amortize-input-load, gqa-kv-head-sharing-bandwidth-reduction) — a rider face breaks
them; the fix is a source-level literal replacement (old concept literal → new literal), and the
other 10 hosts have no full-string pins (needle-style asserts only).

## Gates

ALL GREEN: batch276 needles + parse 2/2 (after one needle's trailing-space fix — the needle must
be a verbatim substring, punctuation included); kernel_opt lib 3221/0; default lib 2729/0;
clippy −D ×3 postures (default / kernel_opt — one `too_many_lines` on the new gate test, allow
added; / no-default); retrieval floors HELD 22/22 at the kernel_opt+latent posture (433 s) and
21/21 at the default posture (95 s) — no re-pin. Freeze/score-bench NOT owed (kernel_opt-only
corpus, P30/B250). Binary rebuild NOT owed (kernel_opt outside the release feature set).

## Queue after

Batch 277 = the Humming + Chord pair (claimed by the sibling session, Plan 281, gated on this
landing). Walk #17 opens Mon 2026-10-27 00:00Z (claim-first). nnethercote ~11-01. `.raw/b275/`
(both clones + staging) removed at this batch's close.
