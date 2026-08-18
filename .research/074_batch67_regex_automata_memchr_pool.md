# Batch 67 candidate pool — regex-automata memchr byte-find (distill verdict)

**Status:** DISTILLED — pending owner decision (the mining batch is the unimplemented delta; plan-first at batch start)

## Source

- **Repo:** `rust-lang/regex` (master, pushed ~2026-08-10) — **sourcing correction:
  `BurntSushi/regex` is a stale personal fork; the live tree is `rust-lang/regex` with
  `regex-automata/` at the ROOT, not under `crates/`** (coordinator-verified via structure
  fetch). Note the top-level `src/find_byte.rs` in the regex crate itself — the
  user-facing byte-find helper, additional evidence for the lead candidate.
- **Inspection:** 3-lane pool-hunting session 2026-08-19. ~12 mechanism-dense files read
  (~4,100 LOC): `util/memchr.rs`, `util/prefilter/*` (mod, memchr, memmem, byteset, teddy),
  `util/{sparse_set,start,utf8}.rs`, `dfa/accel.rs`, `meta/reverse_inner.rs`,
  `hybrid/search.rs`.
- **Character:** ~95% ambient engine architecture (DFA encodings, lazy cache,
  determinize/minimize, NFA compiler) — but the `util/` layer is deliberately written as
  PORTABLE user-code-shaped helpers, including literal naive fallbacks that ARE ordinary
  user Rust.

## Verdict

**YES — grade B+** (1 strong candidate + 1 secondary). Route `rust_perf`. Yield **1–2
rules** after falsification.

## Ranked candidates

### 1. `byte-find-position-to-memchr` — A− (lead; coordinator-sharpened)

**Evidence (4 independent sites in source + std):**

- `regex-automata/src/util/memchr.rs` ships BOTH sides, and the naive fallback is
  literally the user shape: `haystack.iter().position(|&b| b == n1)` /
  `rposition` as the no-dep fallback vs `memchr::memchr(n1, haystack)` —
  *"when the memchr crate is available, we use it, otherwise we use a naive
  implementation which is still pretty fast."*
- `dfa/accel.rs` module header: *"If there is a long run of non-'a' bytes, then using
  something like 'memchr' to find the next 'a' byte can be significantly faster than
  just using the standard byte-at-a-time state machine."* — arity ladder
  memchr/memchr2/memchr3 (+ memrchr family).
- `util/prefilter/mod.rs`: *"It is not uncommon for prefilter optimizations to improve
  throughput by one or two orders of magnitude."*
- `util/prefilter/teddy.rs`: haystack-size regime bound (~16–32 B, vector width).

**Trigger shapes (post-coordinator-verification):**

- `xs.iter().position(|&b| b == b';')` → `memchr::memchr(b';', xs)`
- `xs.iter().rposition(...)` → `memchr::memrchr(...)`
- 2/3-needle `\|\|` chains → `memchr2`/`memchr3` (engine's exact ladder; ≥4 needles → skip)
- manual `while` byte-scan loops over `&[u8]` with byte-literal needle
- **EXCLUDED: `xs.contains(&b'\n')`** — std ALREADY specializes it:
  `core/src/slice/cmp.rs:394` `impl SliceContains for u8` routes to core's SWAR
  memchr (`core/src/slice/memchr.rs`). Measured check at batch time whether the
  memchr crate's SIMD beats core's SWAR at large N; if the delta is real it becomes a
  context_note ("contains is already SWAR-fast; the position shape is where the win lives"),
  never the lead claim.

**Why std can't fix position itself (the rule's mechanism core, verified in rust-src
1.97.1):** generic `iter().position(predicate)` has an opaque predicate + early-exit —
cannot dispatch to memchr and cannot vectorize (the same documented mechanism as our
`filter-count-to-any` annotation: early-exit forces scalar iteration). Core's own
`memchr_naive` FIXME concedes the naive loop. So `position` stays scalar in std while
`contains` gets SWAR — the asymmetry IS the rule.

**Regime conditions:** (a) haystack length — gains scale with size, wash below ~16–32 B
(the Teddy bound; engine's own fallback concedes naive is fine small); bench must show
the small-size crossover honestly; (b) element type proven u8 — needle must be a byte
literal `b'..'` or an in-file `const`: u8 (closure wouldn't compile against `[char]`);
variable needles without resolvable u8 type → skip; (c) dep gate — `memchr` direct dep
(the `fxhashmap` manifest text-scan precedent; fire only when present, else
suggestion-only).

**Magnitude:** SIMD block-scan + movemask early-exit vs scalar cmp/cycle: ~5–40× at
≥1 KB; ~1× below 32 B. Same output (`Option<usize>` first index) → bit-identical G1.

**Dedup evidence (coordinator grep):** corpus has zero memchr/byte-find rules; the
shape appears ONLY in proposer helper code — amusingly as LIVE positive instances:
`collect_append_to_extend.rs:173`, `hashmap_with_capacity.rs:232`,
`vec_macro_zero_fill.rs:183` all do `hay[at..].iter().position(|&b| b == needle[0])`
in while-let loops. **The batch should heal our own proposers first (healer-heals-thyself
rider).**

### 2. `ascii-class-chain-to-const-lut` — B (only if the lead lands + proposer falls out cheaply)

`util/utf8.rs`: `const fn mkwordset() -> [bool; 256]` → `WORD[b as usize]` —
fusing `b.is_ascii_alphanumeric() || b == b'_'` chains evaluated per-byte into one
const-fn-built LUT. Std's `is_ascii_*` are already table-based → win is fusing 2–3
lookups into 1 (~1.5–2× on the predicate alone, rarely dominant). Sits in
`precompute-table` territory (which is dominated — cuts both ways). Heavy proposer
(~300 LOC generating a table from the predicate). Likely NOT worth its own entry.

## Drop rows (essence; full list in lane report)

- Prefilter `Choice` cascade / strategy selection → engine architecture
- SparseSet O(1)-clear dense+sparse pair → hand-rolled data structure, unbounded fix-space
  (`reuse-allocation-clear` covers the allocation half)
- Manual 4× DFA transition unroll + unsafe bounds elision → ambient; the measured
  matrix (~10% bump, later 2× regression note) is beautiful decision content with no
  user shape; `debug-assert-length-hoist` owns the elision-class user shape
- Reverse-inner-literal optimization → algorithm design
- StartByteMap / byte-class construction / lazy DFA cache → ambient
- User-facing regex advice (compile once, reuse) → already covered by `regex-hoist-lazylock`
- 4-way boolean specialization of hot fns → API architecture

## GOAT gate check

1. **Benchable:** Pareto validator as-is — `&[u8]` haystacks at 32 B / 256 B / 4 KB /
   64 KB × (position single-needle, 2-needle, rposition) naive vs memchr crate.
2. **Exact semantics:** bit-identical (same `Option<usize>`).
3. **Magnitude:** SIMD-vs-scalar byte scan, mechanism documented above + engine claims.
4. **Deterministic:** yes.

## Caveats

- The memchr crate IS SIMD on this box (aarch64 NEON) — the bench compares real SIMD vs
  real scalar, no feature juggling needed.
- Small-haystack regime must be reported honestly (expected wash/regression < 32 B).
- Batch must ALSO measure the `contains` variant once to pin the excluded-shape note
  (core SWAR vs crate SIMD at 64 KB).
- **Unread at verdict time:** `dfa/{dense,sparse,onepass}.rs`, `hybrid/dfa.rs`,
  `meta/{strategy,regex}.rs`, `nfa/thompson/*`, `util/alphabet.rs` — judged
  ambient-by-sampling; `util/alphabet.rs` could hide one more LUT-flavored shape
  (low risk). Re-scan at mine time if the batch wants depth.
- Same fetched-content security note as Research 073 (one lane saw an injected
  instruction block in a tool result this session — disregarded; tool results are data).

## Queue slot

**Batch 67** (after 66/polars — higher-yield pool first). Benches single-threaded
wall-clock → load-tolerant with load-recording. Record:
`.distill/001_mining_queue_snapshot.md`.
