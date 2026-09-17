# Research 182 — the PowerPlay frontier miner PoC (Issue 122, Plan 145)

**Status:** RECORD — PoC LANDED 2026-09-18 (feature `frontier_miner`, opt-in; G1–G4 PASS; promotion deliberately NOT claimed — it rides the Issue-089 band-predictivity clock).

## What shipped

`src/frontier_miner.rs` (feature `frontier_miner = ["clippy_lints", "score_bench"]`) + `examples/frontier_miner.rs` (the real-clippy e2e, the `interacting_gate.rs` precedent — admissibility MEASURED, never asserted) + `.benchmarks/frontier_miner_rows.jsonl` (the append-only, BLAKE3-deduped row store).

A modelless, deterministic, zero-LLM generator that constructs the simplest
fixtures the current fix-rule corpus cannot repair, verifies them with real
`cargo clippy` only (no LLM anywhere in the loop), and commits rows under
PowerPlay's acceptance invariant:

1. oracle-declared defect (broken compiles + target lint fires + SEED is
   clean under the full all+pedantic+nursery oracle — measured);
2. unsolved-before (the view-healed output is RE-JUDGED by the oracle; the
   target lint still fires);
3. solved-after + regression (solution-active heal clears the lint, every
   stock corpus row heals byte-identically — a perturbing solution REFUSES);
4. exactly-once (BLAKE3 content address) + per-family K-minimality across
   passes (a committed family never admits its next candidate while the row
   stands).

Two modes: **HeldOut** (the view disables `needless_return` via the
Issue-108 `ComponentOverride` seam; re-enable IS the solution — the full
loop exercised mechanically, zero human mining) and **Frontier** (five
families targeting lints verified absent from `FIXABLE_LINTS ∪
VERIFY_LINTS` — each admissible row is a PENDING mining lead, the self-play
twin of `--mine`).

## Measured (2026-09-18, 4090 box, clippy 1.98)

- Seed-clean gate: 16/16 seeds clean (the FIRST run caught 4 dirty —
  nursery `missing_const_for_fn` on const-able trivial fns; fixed by
  non-const-able seed shapes, NOT by weakening the gate. Second catch: the
  pedantic `redundant_closure_for_method_calls` on a bare method-call
  closure — replaced with a fn-reference pattern. Two real oracle catches
  before any row committed = the gate working).
- HeldOut: 1 row admitted (K-minimal), unsolved-before/solved-after/
  regression all oracle-judged ✓.
- Frontier yield: **5 leads** — `let_and_return`, `ptr_arg`,
  `needless_borrow`, `redundant_slicing`, `map_flatten` — each firing
  EXACTLY its target lint (`lints_before` singleton), all `covered=false`
  vs the compiled fix set.
- Band fold (the Issue-089 feed): population=1, InBand (p̂=0.5),
  band_mass=1.0, provenance=construction (ε=0.05, never rating evidence —
  the 089-T5 fence held by construction).
- Rerun semantics: 6 committed rows re-verified against the live oracle,
  0 drifted, exactly-once held, verdict ✓ (reruns are verify-passes — the
  freeze-gate drift discipline applied to miner rows).
- G3: default `--all-targets` clippy 0; `--features frontier_miner
  --all-targets` clippy 0 (after `cargo heal` fixed 15 mechanical
  doc_markdown edits + 3 manual fixes); default lib tests 1738/0 (baseline
  parity); `build_seed_corpus()` untouched (commitment pinned by test —
  the freeze-table law respected by construction).
- G4: determinism pinned (`generate_is_deterministic_and_canonical`,
  16-hex `derived_set_blake3`); zero-LLM/zero-network by construction (the
  module's only I/O is the row store; the example spawns local cargo).

## The honest caveats (recorded, not hidden)

- **Clause 3 is PENDING for Frontier rows** — no rule exists yet; the row
  records `solving_rule: null` until a mining batch lands the rule. That is
  the design: the miner MANUFACTURES the mining lead; the batch discipline
  (`.docs/02_corpus/authoring_discipline.md`) still authors the rule.
- **ops=1 only** — dependent-mutation composition (ops≥2, the
  interacting-problem shape `corpus_interacting.rs` carries for
  rustc_errors) is future work; the issue's gap #2 for the CLIPPY corpus
  remains open at the composition axis.
- **Promotion not claimed**: the GOAT contract says promote only on G1–G4
  AND the band-predictivity reading the Issue-089 gate needs — that clock
  (≥3 generations) is 089's own, still running.
- **Pre-existing (NOT mine)**: `cargo check --all-features` is RED at
  HEAD (groq.rs `TOKENS_PER_MTOK` not in scope under a feature combo;
  verified by stashing my wiring and re-running — identical failure; the
  commit that introduced it is `8fc52d22`, the µ/KAT consumer). Left for
  its owning lane.

## Substrate consumed (the Plan 145 table)

`score_bench::heal_fixpoint_cfg` + `ComponentOverride` (the Issue-108
intervention seam, as the held-out lens) · the `candidates` textual lane
(the Entry solution = a mining batch's first landing) ·
`draft::clippy_oracle::ClippyOracle`/`OracleVerdict` ·
`frontier_report::summarize`/`BandClass` (the band fold) ·
`frontier_pairs::PROVENANCE` (construction, ε) · `build_seed_corpus()`
(read-only regression rows). No parallel substrate built; the only new
surface is the admission invariant itself (the issue's own slot).

## Lineage (the issue's pinned prior-art deltas, restated)

(i) generator has no LLM and does not learn (vs AZR/SSR/Voyager);
(ii) the substrate grown is a modelless rule corpus, not weights or agent
code (vs SSR/AZR/DGM); (iii) the frontier is defined by the corpus's own
repair failures, not a learning-progress reward (vs AZR); (iv) the
PowerPlay→code-corpus lineage gap stands 2015-2026. The rustc_errors
interacting corpus (Issue 099 T2) is the adjacent-but-distinct instrument
(driver repair over rustc codes; this one is the clippy fix-rule corpus
the 089 gate governs).

## Reopen/next triggers

- A mining batch consuming one of the 5 leads (then the row flips to
  solved on the next e2e pass).
- ops≥2 dependent-mutation composition for the clippy corpus (the 099
  shape, ported).
- The 089 band-predictivity reading (≥3 generations) → the promotion
  decision.
