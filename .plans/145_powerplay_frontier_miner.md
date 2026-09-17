# Plan 145 — Issue 122 PoC: the PowerPlay frontier miner (modelless generator at the capability frontier)

**Status:** COMPLETE — PoC landed 2026-09-18; GOAT G1–G4 PASS; issue closed (record: Research 182 + HISTORY 2026-09-18).

---

## What this lands

A **modelless, deterministic, zero-LLM generator** that constructs the
simplest code fixtures the current fix-rule corpus cannot yet repair,
verifies candidates with the real clippy oracle only, and commits admissible
rows (unsolved-before / solved-after / prior-rows-green / exactly-once) with
band evidence for the Issue-089 admission gate. Feature `frontier_miner`,
opt-in; **never touches `build_seed_corpus()`** (freeze tables unmoved).

Issue: `.issues/122_powerplay_frontier_miner.md` (the pinned claim + GOAT
contract). Research lineage: `.research/178` (Schmidhuber RSI essay), prior
art POET/MCC/AZR/SSR with deltas pinned in the issue.

## Substrate check (substrate-first skill, Mode 1 — 2026-09-18)

- Searched for: generator/composer/mutator (`corpus_gen`, `compose_multi`,
  `build_seed_corpus`), interacting pairs (`corpus_interacting` — EXISTS,
  Issue 099 T2, rustc_errors domain only), acquisition/candidate-search
  (`gp_acquisition` — GP σ² ordering, Issue 048; different criterion, cited
  not consumed), oracle (`clippy_oracle` auto-oracle, `draft::clippy_oracle::
  ClippyOracle`, score_bench `OracleVerdict`), intervention seam
  (`score_bench::ComponentOverride` + `heal_fixpoint_cfg` — CONSUMED as the
  held-out view + the solution lanes), band arithmetic (`frontier_report::
  summarize`/`BandClass` — CONSUMED for G2), provenance (`frontier_pairs::
  PROVENANCE` = "construction", ε=0.05 — CONSUMED), coverage steering
  (`corpus_coverage` fix-set vocabulary — CONSUMED for the uncovered report).
- Found: every ingredient exists; **no PowerPlay-admission loop exists**
  (zero `frontier_miner|powerplay` hits). The interacting corpus
  (`corpus_interacting.rs`) closed the issue's gap #2 for the rustc_errors
  driver AFTER 122 was filed — the CLIPPY fix-rule frontier (this issue's
  population) remains uninstrumented.
- Decision: **build new, consuming substrate** — the module composes
  `heal_fixpoint_cfg` + `ComponentOverride` + `ClippyOracle` +
  `frontier_report`/`frontier_pairs` vocabulary; the only new substrate is
  the admission invariant itself (the issue's own slot).
- Architectural rules: modelless mandate respected (no LLM, no network, no
  training — G4 pins it); corpus/data evolution only, never healer-source
  self-modification (the Gödel boundary the issue pins).

## Design (pinned before code)

- **Two admission modes:**
  - `HeldOut` — the view disables a COVERED rule (ComponentOverride);
    generated fixtures that only that rule heals are unsolved-by-the-view;
    re-enabling = the solution (exercises the FULL invariant loop
    mechanically, hermetic).
  - `Frontier` — families target lints OUTSIDE the compiled fix set
    (checked against `FIXABLE_LINTS ∪ VERIFY_LINTS`); clippy fires, nothing
    heals → a genuine mining lead row (the self-play twin of `--mine`).
- **Generator:** table-driven clean seeds + 1-op mutation families (the
  `corpus_gen` parametric pattern — no RNG, byte-stable). Canonical order
  `(ops, blake3)` = MDL simplest-first, content-hash tie-break. K(c)
  minimality: per family, the FIRST admissible candidate at the lowest op
  count commits; a family with an ops=1 admission never takes ops=2.
- **Admission invariant per row** (the issue's four clauses):
  1. oracle declares ≥1 clippy lint on the broken form AND no error-level
     diagnostic (compiles) AND the SEED is clean (seed-clean gate — measured,
     the corpus_interacting lesson);
  2. `heal_fixpoint_cfg` under the view leaves the target lint's fixture
     unhealed (unsolved-before);
  3. solution-active re-heal clears it (solved-after, HeldOut mode) AND the
     regression arm holds: every stock corpus row heals byte-identically
     with the solution active (a solution that re-breaks/perturbs a prior
     row is REFUSED — the adversarial G1 arm);
  4. exactly-once by BLAKE3(fixture source) against the committed jsonl.
- **Row store:** `.benchmarks/frontier_miner_rows.jsonl` (append-only;
  `{ts, sha, family, ops, lint, blake3, mode, solving_rule, seed_blake3}`;
  dedup on load). Rows carry provenance = `frontier_pairs::PROVENANCE`.
- **Band evidence (G2):** admitted HeldOut rows → `FrontierRow { rule,
  r:1, f:1, n:2 }` → `BandClass::InBand` (p̂=0.5) via `frontier_report::
  summarize` — the synthetic feed, ε-priced, never rating evidence (the
  089-T5 fence). Frontier rows are BelowBand (r=0) = the mining signal.
- **Two-tier verification** (the freeze-gate / interacting_gate precedent):
  hermetic lib tests (mock oracle verdicts + compiled proposers) + the
  real-clippy e2e as `examples/frontier_miner.rs` (spawns `ClippyOracle` on
  temp fixtures; the authority for admission claims).
- **Freeze-table law:** `build_seed_corpus()` untouched; G3 asserts the
  corpus BLAKE3 commitment is byte-identical pre/post.

## Tasks

- [x] T1 — `Cargo.toml` feature `frontier_miner = ["clippy_lints",
      "score_bench"]` + `lib.rs` gate + module skeleton `src/frontier_miner.rs`
      (types: `Family`, `Candidate`, `CorpusView`, `Solution`, `AdmittedRow`,
      `MinerConfig`; doc header with the pinned claim + substrate table).
- [x] T2 — generator: seed + mutation family tables (≥1 covered-lint family
      for HeldOut; ≥3 uncovered-lint families for Frontier — landed 1 + 5),
      canonical ordering, per-family K-minimality; determinism tests (G4: same inputs →
      byte-identical set, BLAKE3 over derived set).
- [x] T3 — admissibility core with injected oracle verdict fn: unsolved-
      before / exactly-once / minimal-K; hermetic tests incl. the seed-clean
      skip and compile-error skip.
- [x] T4 — solution + regression: `Solution::Reenable` / `Solution::Entry`;
      solved-after; prior-rows byte-equality regression; the adversarial
      refuse arm (a broad textual solution that perturbs a stock row's heal
      output must REFUSE) — G1. Landed the cross-pass K-minimality repair
      (`KnownSet { blake3s, families }` — a committed family never admits
      its next candidate; a rerun is a verify-pass).
- [x] T5 — band evidence + row store: FrontierRow/BandClass fold, jsonl
      append + dedup load, provenance tag; G2 hermetic test (HeldOut rows
      InBand; Frontier rows BelowBand; organic feed untouched).
- [x] T6 — `examples/frontier_miner.rs`: the real-oracle e2e (seed-clean
      gate, both modes, regression, band summary, jsonl append, uncovered
      steering report vs the compiled fix set). RUN for the GOAT record —
      VERDICT ✓ twice (fresh: 6 rows; rerun: 6 re-verified, 0 drift). The
      oracle caught two real seed defects pre-commit (nursery
      missing_const_for_fn ×4, pedantic redundant_closure_for_method_calls
      ×1) — repaired in the tables, gate never weakened.
- [x] T7 — GOAT sweep: G1–G4 evidence recorded (Research 182); `cargo
      clippy` 0 at default AND `--features frontier_miner` (`--all-targets`);
      lib tests 1738/0 at default (baseline parity) + 13/13 hermetic;
      corpus commitment unchanged; freeze tables untouched. ⚠ Pre-existing,
      NOT fixed (unrelated): `--all-features` RED at HEAD — groq.rs
      `TOKENS_PER_MTOK` combo break from `8fc52d22` (verified pre-existing
      by stash-and-rerun; its owning lane's guard will catch it).
- [x] T8 — docs: AGENTS.md feature row; issue close-out; research note
      (`.research/182`, RECORD); `.distill/001` post-mining intake pointer
      (the 5 leads); HISTORY.md entry; plan checkboxes.
- [x] T9 — commit `feat: Issue 122 PoC — frontier miner ...` + push; issue
      file removed per the noise rule.
