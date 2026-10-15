# Auto-research Contribution Rails — CrocSwap/integer-mult-bounds Distill

**Status:** RECORD — Track A (the integer-multiplication math) PASS for the workspace · Track B (the agent-contribution rails) Gain → katgpt-rs Issue 943
**Date:** 2026-10-15
**Source:** github.com/CrocSwap/integer-mult-bounds @ `0fdd7d82de03e43c9e5a1000dfbcd380d1b687d2` (Apache-2.0; 2,224 commits; 1,147 `research/` packages; ~104 source PRs in the selected frontier; read via a read-only `.raw/` clone, removed at commit time)
**Context:** the user shared a post about a "geometric self-attention" project that names this repo as its rails model ("testing auto-research.md rails now"). That project is NOT distilled here — its four techniques (basis-curve weight sampling, hierarchical state vs KV, forward-pass perturbation optimization, expert bundling) each need their own run once it publishes.

## TL;DR

Douglas Colkitt's (CrocSwap/Ambient) community auto-research farm grinding κ in
`T(n) = O(n (log n)^{1-κ})` for integer multiplication — the most complete
public assembly of agent-contributed research rails: seeded task allocation,
pre-claim overlap search, machine-verifiable claim packages, independent
second verifiers, certificate chains, and ONE selected-frontier record with
per-PR attribution. The math is conditional on an unreviewed OpenAI preprint,
asymptotic-only, and has zero consumers in our stack — **PASS**. The rails are
a working reference implementation of the multi-agent research-collision
problem this workspace has paid for repeatedly (ai-1071, katgpt-rs Issue 825 —
both SAME-ISSUE collisions, see Fusion) — **Gain**, one issue filed. Everything else in the rails validates machinery
we already ship under different vocabulary.

## Track A — the math: PASS

**The claim** (selected-result.json at the pinned sha): κ =
836330098538459/500000000000000000 = 0.001672660197076918, i.e.
`T(n) = O(n (log n)^{1-κ})` — BELOW the Harvey–van der Hoeven O(n log n)
record (Annals of Mathematics 193(2), 2021, arXiv:1802.07932). Selected PR is
#1094 (Rohan Arun), priority revision 4, 2026-10-14, composing ~104 source
PRs. The construction recursively replaces width-m work with ranked children
(a Harvey–vdH-lineage reduction); a candidate "bill"
`Σ n_r (r/m)^(1-a) / W` plus contract charges must sit strictly below one.

**Honest status — treat as unverified:**
- Everything is "conditional on the original OpenAI #109 framework"
  (github.com/openai/math family #109, 2026-10-06: κ₀ = 2⁻¹⁸², produced by an
  unreleased internal model, NO Lean formalization; OpenAI withdrew 3 of 722
  preprints within 24 h and revised 14 more).
- The repo says it plainly: "This is **not a complete Lean multiplication
  theorem**" — all-size/compiler, realization, hosting, precision/tape and
  analytic obligations remain open. Only the stated FINITE contracts are
  replayed and certified.
- Model note: NOT word-RAM/SWAR — the "packing" is bit-circuit/tape-frame
  level on a multitape Turing machine. No super-linear lower bound is known in
  that model, so the claim refutes the Schönhage–Strassen optimality
  conjecture rather than any theorem — if it survives review, which it has not
  (Ben Recht: galactic constants, "an exploit of the book-keeping"; mixed HN
  reception; Fortnow curated with the caveats attached).
- κ ≈ 1.7e-3 is an EXPONENT saving, not a runtime result — invisible at any n
  a physical machine can represent. The repo itself forbids the conflation.

**Verdict: PASS.** No bignum consumer exists anywhere in the 27-repo stack;
the certified discipline (not the math) is the transferable part, and it is
already our convention. PASS-Redirect line filed at the closest shipped
cousin: riir-chain/.research/004 (LatCal fixed-point bridge, Lean-checked).

## Track B — the rails: Gain

The rails inventory (all in-repo, all working, all Apache-2.0):

1. **Entry contract** — root `autoresearch.md` (516 lines) + `AGENTS.md`:
   priorities, task selection, overlap rules, stopping criteria. Rohan Arun
   has proposed standardizing root-level `auto-research.md` so agent
   harnesses can discover repos fleet-wide.
2. **Bounded sessions** — ~60 min default; "a PR is not mandatory"; "a
   handoff is a valid outcome"; negative results are first-class ("useful
   partial/negative results do not need a new κ to merit a focused PR").
3. **Deterministic seeded allocation** — `choose_research_task.py`: task
   catalog → weighted track → 128-bit OS nonce → task cards; `--seed`
   reproduces; "a deterministic seed is a reproducibility aid, not novelty".
4. **Pre-claim overlap search** — `find_overlap.py` over PR bodies +
   discussions + local package notes, `--offline` fallback, coverage +
   freshness disclosed, and the honesty line printed in AGENTS.md: **"no
   matches is not proof of novelty."**
5. **Claim package shape** — CLAIM.md carries: exact base sha, scoped object,
   hypothesis, FINITE method/admit class, deliverable, expected finish + hard
   handoff deadline, seed provenance, and explicit overlap adjudication vs
   named predecessor PRs ("#739 covers power-of-two-only reuse and explicitly
   excludes odd-numerator transitions; this class differs").
6. **Verification ladder** — the claim's own `verify.py` + an independent
   `oracle.py` + an `independent.py` REIMPLEMENTATION + VALIDATION/CERTIFICATE
   jsons + dual NAMED human auditors per checkpoint.
7. **Certificate chain** — `certificates/selected-result.json` is the ONE
   frontier record (κ, source PR list, source_manifest, certificate sha256);
   frozen checkpoints; PR ledger + CONTRIBUTORS.md attribution retained even
   for non-advancing contributions.
8. **Loop hygiene** — "refresh upstream at each research checkpoint": fetch
   canonical main, PIN FETCH_HEAD immediately, compare priority_revision,
   isolated worktree per round, frozen evidence untouched, local-only
   evidence labelled, scope changes declared.
9. **Positive control** — `research_baseline.py`: a cheap source-bound
   orientation check, explicitly "not a full circuit replay".
10. **Claims taxonomy** — "Distinguish a finite certificate, a scoped proof,
    a search failure and a complete multiplication theorem. They are
    different claims."

## Fusion — the workspace cousin map (signal-diff per row)

| Rails mechanism | Our shipped analog | Signal-diff | Delta |
|---|---|---|---|
| Overlap search before claim | Mining-queue claim markers ("no file overlap with any live lane"), `dual_allocation_gate.py` (numbers), substrate-first skill (new systems) | Theirs: SEMANTIC topic-level search over every landed claim, pre-allocation. Ours: file-level + number-level, or vocabulary-grep scoped to substrate systems | **GAP WE PAID FOR — but the paid collisions are SAME-ISSUE, not semantic** → the lease (katgpt-rs Issue 943); semantic search deferred to v2 pending a real semantic fixture |
| Verification ladder | GOAT gates, verdict ping-pong, `cargo refine --verify` compile gates, distill quote re-verification at pins | Same shape; their `independent.py` ≈ our verdict reviewer + arm-reach mutation | None worth an issue |
| Certificate chain | `.benchmarks` verdict records, BOUNDARY drift ledger, HISTORY.md, per-repo current-status sections | Same shape (single curated truth + sha-pinned records) | None |
| Loop hygiene | Protocol H (sync both boxes), mining-loop upstream refresh, frozen pins | Same law; their "pin FETCH_HEAD immediately" is the same lesson as our mlxfast wrong-base artifact class (named in the mining queue) | None |
| Claim deadlines | Mining-queue claim expiry 48 h | Identical | None |
| Claims taxonomy | Full-gate axes ("A green test_gate.sh is a claim about the DEV profile only") | Validation, not delta | None |
| Seeded allocation | Hand-curated mining queue + dated leads | Theirs serves unassigned donor fleets; ours serves one operator's priority order | Minor; not worth an issue |

The named failures that justify Issue 943 — verdict-round-1 correction: BOTH are
SAME-ISSUE collisions (no in-flight marker on the issue before the second
session started), neither is a semantic-vocabulary duplicate:
- **ai-1071** (2026-10-15): ONE issue file
  (`riir-ai/.issues/1071_hero_knock_slide_no_wall_stop.md`), implemented by
  BOTH boxes the same day; the M3's push `f2447c873` landed first and the
  4090's push was rejected mid-flight after 800+ lib tests were already
  green. The claim-marker yield was correct — but it only ran AT PUSH TIME,
  after the duplicate work. Residue `88dc3220b` landed instead.
- **katgpt-rs Issue 825**: two sessions, ONE issue, four hours apart; the
  second recorded a NEGATIVE result against the first's in-flight state.

So the actionable delta is an ISSUE-LEVEL CLAIM LEASE (a `Claimed:` line on
the issue/plan file, pushed before implementation starts, checked as the
session's first step) — extending the mining queue's claim-marker substrate
to the `.issues/`/`.plans/` surfaces concurrent sessions actually allocate
against. The rails' SEMANTIC overlap search stays DEFERRED (Issue 943 v2):
no fixture with different issue numbers AND different vocabulary exists, and
building the semantic class without one is exactly the §1.55
"validates-our-design" trap.

## What NOT to import

1. **The math.** No consumer; do not route a mining batch at the packing
   packages without a `distill`-skill pass — and that pass must run the
   kernel_opt corpus dedup grep first (the SWAR face already ships as
   `lut-free-bit-spread-vadd4-unpack` + the Batch-204 NEON PQ2_0 rule; the
   repo's packing circuits are cousins, not novelty).
2. **The κ number.** An exponent saving in a contested conditional framework —
   never quote it as a runtime result or a multiplication benchmark.
3. **"Any repo with auto-research.md is a mining target."** The distill
   skill's license + decision-content testing + corpus dedup rules still
   apply per source; the rails make CONTRIBUTING easier, not vetting
   optional.

## Verdict

- Track A (math): **PASS** — recorded here; PASS-Redirect filed at
  `riir-chain/.research/004_LatCal_Fixed_Point_Bridge_Lean4_Proof_Guide.md`.
- Track B (rails): **Gain** — one actionable delta (an issue-level claim
  lease before implementation; the rails' semantic overlap search is its
  DEFERRED v2, gated on a genuinely semantic fixture) →
  `katgpt-rs/.issues/943_claim_overlap_search_gate.md`; the rest of the
  rails is independent confirmation that our gates, certificates,
  attribution and loop-hygiene machinery are the right shape — validation is
  not actionability, so no further files.
- Routing: note here (riir-refine) per the fusion ladder's #2 — the rails'
  primary consumer is this workspace's own multi-agent loop (mining queue,
  idle loop, claim markers); the instrument lands in katgpt-rs/scripts next
  to `dual_allocation_gate.py`, the established cross-repo gate home.

**Provenance:** repo pinned at `0fdd7d82de03e43c9e5a1000dfbcd380d1b687d2`;
clone read-only; external context via two web sweeps (math landscape + rails
landscape, 2026-10-15) — OpenAI #109 status, Recht/Fortnow/Tao reception, and
the sibling-rails landscape (Karpathy `autoresearch` lineage,
autoresearch-at-home, ZeroThesis, AgentRxiv) come from those sweeps, not from
the clone. Prior-art sweeps are summarized in the verdict round; no internal
`.research/` note previously referenced this repo (grep: zero hits).
