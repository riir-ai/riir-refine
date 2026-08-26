# Batch 74 — simd-lite/simd-json SWAR distill (rust_perf external)

**Status:** RECORD (batch landed — `feat: batch 74`, corpus 78→80)
**Source:** https://github.com/simd-lite/simd-json @ v0.18.1 (`main`, inspected 2026-08-26)
**Provenance:** Muła 0x80.pl SWAR articles (cited in-source) ← simdjson C++ ← the Rust port
**Plan:** `.plans/032_simd_json_swar_distill_batch74.md` · **Queue:** consumed the Research 085 §Pool-drops simd-json re-probe

## The re-probe (why this note exists)

Research 085 dropped simd-json as "repo fetch FAILED (MCP 'repo not found') — unverified,
re-probe at pool time". The re-probe found the repo alive and healthy — **the failure was
MCP zread indexing, not the repo**: direct `fetch` of github.com + raw.githubusercontent.com
worked for every file. 1.4k stars, 897 commits, MIT/Apache dual, edition 2024, active
(2026 hacktoberfest). Release profile `lto=true, codegen-units=1, opt-level=3` + criterion
benches over the classic simdjson datasets (canada, citm_catalog, twitter, github_events) —
they benchmark seriously. **Tooling lesson: an MCP repo-not-found is a tool gap until a
direct fetch proves otherwise — never grade unread, but also never drop unprobed.**

## Verdict: YES B+ → rust_perf (NOT kernel_opt)

The tempting wrong domain is kernel_opt ("SIMD library") — but the mineable content is
*portable* SWAR arithmetic + a dispatch-cost decision. The x86 SSSE3 escalation
(`_mm_maddubs_epi16`) is a citation note, not the rule.

## Landed rules (2)

| Rule | Category | Measured (M3 Max release, load ~7.1, rustc 1.97) | Verdict |
|---|---|---|---|
| `swar-digit-chunk-parse` | arithmetic | **2.35×/1.75×/2.52× @ 8/12/19 digits** (N_RUNS=2000); **1.01× tie @ 4 digits** (the honest regime bound — the source's own README caveat); 0/0 allocs, alloc-det stable; verdict stable ×2 | **ParetoDominant** (annotated regime) |
| `runtime-feature-detect-per-chunk` | dispatch | **1.01×/1.04×/0.99× @ 16/256/4096 B/chunk — 0/0/3 ties**; the std::detect machinery const-folds to near-zero through an `#[inline(never)]` boundary on rustc 1.97 aarch64; verdict stable ×2 | **Mixed** (honest-annotated — the B68 class) |

### `swar-digit-chunk-parse` — the GOAT lead

`numberparse.rs` `parse_eight_digits_swar` + `is_made_of_eight_digits_fast`: the per-digit
loop (`val = val*10 + (b - b'0')`) is a serial dependency chain at ~6 ops/digit; the SWAR
fold replaces it with 3 and/mul/shift steps per 8 digits (magic multipliers 2561 =
10·256+1, 6553601 = 100·2¹⁶+1, 42949672960001 = 10⁴·2³²+1 — each step merges adjacent
pairs/quads/halves, byte 0 most significant), guarded by the one-test digit validation
(every byte in 0x30..=0x39 ⟺ the `0x3333` SWAR test — high nibble 3 AND low nibble +6
no-carry; Muła's construction, cited in the source comment). The chunk multiply
`val * 100_000_000 + chunk` never overflows before the final value does (prefix·10^remaining
≤ final — proven + gated).

**G1 (the ported-numeric MANDATE):** exhaustive equivalence gate in corpus tests — all
lengths 0..=19 × boundaries (all-0s/all-9s/10^k) + the exact 20-digit u64::MAX string +
10K LCG strings + non-digit terminators at every offset (value AND stop position) + the
validation matrix (8 positions × boundary bytes + 10K LCG octets) + 100K fold checks vs
the u128 reference. **The gate caught a real transcription bug pre-landing**
(`u64::from_le(*chars)` vs `from_le_bytes` — E0308 at first compile, fixed in all 3
transcription sites). Domain: digit runs fitting u64.

Composes with `unaligned-word-load-from-bytes` (B69 — the chunk load IS that pattern);
sibling of `division-by-invariant-multiplication` (B59 — magic-multiply family: radix
folding vs divisor inversion).

### `runtime-feature-detect-per-chunk` — the honest-Tie

The in-source measured decision comment in `parse_eight_digits_unrolled`: *"Only use the
SSSE3 version when the feature is statically available: a per-call `is_x86_feature_detected!`
check costs more than the SWAR fallback (cached-atomic load + two outlined `target_feature`
calls per 8-digit group)"* — detection granularity as a bounded decision (resolve once:
cached static / hoist / static cfg gate). Our bench on rustc 1.97 aarch64 measured the
per-chunk detection as **sub-0.1 ns marginal** — the feature-name match on a literal
const-folds through the inline boundary; the source's cost class survives for outlined
`#[target_feature]` dispatch, non-literal feature strings, and heavier probe libraries.
Landed as the honest-annotation record (the `varint-compose-word-single-store` B68
precedent): encodes the granularity DECISION without claiming a default-config hot-path win.

## Drops (killed at verdict, with reasons)

- **hex-LUT sentinel fold** (`charutils.rs` `hex_to_u32_nocheck` — the `DIGITTOVAL: [i8; 256]`
  LUT whose −1 invalid entries sign-extend through the fold, deferring validity to ONE
  caller check): niche (ASCII hex text); the LUT half is `precompute-table`-adjacent; the
  sentinel-propagation half is genuinely interesting but too narrow to bench honestly.
- **`get_kinda_unchecked`** (`safer_unchecked.rs` — cfg(debug_assertions) checked /
  release `get_unchecked`): the suggest-unsafe class — a parse-only healer cannot prove
  the invariant. The safe complement `debug-assert-length-hoist` already landed (B64).
- **KnownKey memoized hash** (`known_key.rs` — memoize the hash, `raw_entry().from_key_
  hashed_nocheck`): requires non-std `raw_entry` (std removed it; halfbrown/hashbrown-only)
  — no bounded std fix-space. B61 `keyword-map-to-match` owns the static-key regime.
- **Architectural kills:** two-stage tape parsing, Eisel-Lemire float parsing
  (`numberparse/{correct,approx}/`), the `impls/` arch-dispatch framework, stage2 value
  building, serde integration, `NotSoRandomState` hash-stability infrastructure.

## Dedup evidence (both vocabularies grepped, full 78-rule list enumerated)

- swar/eight_digits/parse_eight/digit-parse → 1 hit: B67 memchr's doc note about core's
  SWAR `contains` specialization (a different mechanism application — read, not coverage).
- is_x86_feature_detected/feature_detected/runtime-detection/capability → 0 hits.
- hex/nibble/DIGITTOVAL → kernel_opt GPU packed-quant rules only (different substrate:
  unpacking packed nibbles vs parsing ASCII hex text).

## GOAT

- **G1** exhaustive gate PASS (default lib 680/0) + bench-harness debug asserts + validator failed=0.
- **G2** both rules measured, verdicts stable ×2 invocations, alloc-det stable, 0/0 allocs.
- **G3** clippy 0 both states (`--all-targets`, default + rust_perf_bench) + retrieval
  floors HELD zero decay (clippy@1 89.6, perf@1 96.2 — identical to B73's recorded values;
  blend@1 93.1 structural-mode at the 101-span set, gate ok) + **full CI guard ALL LAYERS
  on the exact commit tree** (origin/develop `0b1992e` + this batch's corpus patch, run in
  a sibling-adjacent worktree — see the verification note below).
- Count pins: 80 entries / 70 benchable / 22 annotated / 12 categories (arithmetic 7, dispatch 3).
  Drive-by: the header count comments were one batch stale (76/66 vs actual 78/68 — the
  count-pin-lesson-19 class, found by inspection not by grep); fixed to 80/70.

## Verification note (the worktree dance, two traps re-hit)

The shared checkout carries a sibling session's in-flight WIP (`src/component.rs` +
`lib.rs` + `memory.rs` — issue 047's component-attributed miss map), and origin/develop
moved mid-session (`6c10d80` → `0b1992e`, docs-only). The full guard was therefore run in
a worktree on the exact commit tree — re-hitting two documented traps: (1) a `/tmp`
worktree breaks sibling path deps (`../riir-ai` unresolvable — worktrees belong beside
the siblings); (2) riir-clippy's Cargo.lock is UNTRACKED, so a fresh worktree re-resolves
a drifted graph (crates.io cubecl without the expected API — the failure masquerades as a
compile error in sibling code). Seed the worktree's lock from the main checkout's. Both
traps are now recorded here for the next batch session.

## Numbering flags for the owner (not fixed by this batch)

`.research/` 086 is DUAL-allocated (`086_dockerfile_detection_inventory_hadolint_buildkit`
vs `086_mnt_io_room_list_dod_batch73`) — same class as the known `.research/` 083 +
`.plans/` 026 duals. `.research/.highwater` was stale at 86 (087 existed); this batch
bumped it to 88.
