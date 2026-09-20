# Research 195 — Batch 167: unnecessary_cast + clone_on_copy (the coverage-report re-arm pack XI)

**Status:** RECORD (batch landed — see the commit hash in `.distill/001` and
Plan 158's closing line). Corpus 86→88 (69 FIXABLE / 16 VERIFY / 3
self_evolve-only). Sources: the fresh `--coverage-report` derivation of
2026-09-21 + probe `scripts/plan158_frontier_probe.py`.

## Lead provenance

The Batch-164 re-arm is the standing pipeline: the recorded-spare queue ran
empty at Batch 163, so leads come from the coverage-report derivation. Fresh
run at develop `d8a2c0ec` (clean worktree build — the in-tree tree carried
sibling WIP): clippy 0.1.98, 822 derived / 86 covered / 736 uncovered /
**423 default-firing / 116 with default-eco evidence**, 0 cross-domain dedup
collisions, 0 fix-set-only (the corpus-lockstep law holds from the side no
test had measured, again).

## Probe verdicts (full oracle: all+pedantic+nursery, default-group arms)

| candidate | verdict | the record |
|---|---|---|
| `unnecessary_cast` (48, top non-doc row) | **LEAD** | fires on proven-ident same-type casts AND literals (`5 as u64` → `5_u64`, `5u64 as u64` → `5_u64` — canonical re-suffix). Cross-type casts fire `cast_lossless`/`needless_type_cast`/`cast_possible_wrap` — different lints; the provenance proves same-type exactly. |
| `clone_on_copy` (16) | **LEAD** | fires clean (`x.clone()` → `x`); non-Copy clones fire `redundant_clone` — different lint. Default-level pinned. |
| `needless_borrowed_reference` (27) | **REJECT** | all five natural pattern shapes SILENT under the full oracle (plan152's `mrp` posture — recorded, do not re-derive from memory). |
| `double_ended_iterator_last` (17) | **REJECT** | `iter().last()`, `into_iter().last()`, `rev().last()`, BTreeMap `values().last()` all SILENT. |
| missing_safety_doc (186) / missing_errors_doc (90) / missing_panics_doc (78) | skip | doc-AUTHORING, not a bounded mechanical fix (Research 191 adjudication). |
| len_zero (20) | **alias gap** | covered by our `comparison_to_empty` under a different name — the name-keyed derivation counts it uncovered. Derivation honesty note, report-only. |
| result_unit_err / too_many_arguments (28/28) | skip | API-shape (architectural). |

## The batch's finding — the scalar-dodge (retrieval A/B)

The first fixture spelling (u32) **crowded the `deref_addrof` eval span out
of top-1** (measured 1 → 99 in the A/B rank dump; CLIPPY_TOP1/3 floors would
have red). Mechanism: the deref_addrof eval span is u32-based while its
corpus text is u8-based — the hand-authored-variant gap — so a new u32-heavy
entry out-token-matched the truthful rule on the variant. `u64` restores the
pre-batch rankings **byte-identically** (the same fixture-dodge discipline as
Batch 163's `x + x` tail); floors HELD, no re-pin; freeze e2es re-run green
(19.2s / 19.0s). The load-bearing instrument: a per-span rank dump (temp
test, deleted) — the aggregate floors red correctly but only the dump NAMES
the casualty; that dump is this finding's whole evidence, worth re-deriving
whenever a corpus batch moves a floor.

## Instrument lessons

1. **The arxiv-walk bracket-form bug does not repeat here, but the pattern
   does:** the first uc proposal used `matches!(target.as_str(), "u8" | ..)`
   which hit E0658 on this toolchain — a const-slice `contains` is the house
   pattern (three proposers already ship it). Probe the compile, not just
   the lint.
2. **`unnecessary_wraps` is live in the clippy -D gate:** the
   `strip_numeric_suffix` helper was first written `-> Option<String>` with a
   never-None tail; the G3 gate caught it. Total-new-code clippy findings
   this batch: 1 (healed manually — semantic, not in our corpus).
3. The splice script (`plan150_splice_freeze_tables.py`) trusts the out-file
   rows verbatim — the first regen printed UNQUOTED lint names
   (`&[unnecessary_cast]`), the splice spliced them, and the lib broke 262
   errors deep. Restored from git, fixed the printer, re-spliced. The
   out-file is data; its producer is the thing to gate.

## GOAT

G1 module tests 23 (13 uc + 10 coc) + corpus suite green · G2 lib default
2060/0 + kernel_heal 2413/0 (1 pre-existing ignore) · G3 clippy `-D` clean ×2
feature states · freeze regen 88/17 + 88/21, bad sets UNCHANGED, 0 newly
excluded, solve 100.0% every k, M4 + stratification PASS · retrieval floors
HELD byte-identical (A/B-verified) · score-bench **96.4% FLAT** (147 runs;
the 2 toolchain-drift warnings pre-existing since Batch 163).
