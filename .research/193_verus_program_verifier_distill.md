# 193 — Verus program verifier: distill verdict + FV-tooling intel

> **Source:** [Developing provably correct Rust code with Verus](https://www.amazon.science/blog/developing-provably-correct-rust-code-with-verus) — Bryan Parno (Amazon Scholar, CMU), Amazon Science blog, 2026-08-31 · tool: [verus-lang/verus](https://github.com/verus-lang/verus) · paper: Lattuada et al., *Verus: Verifying Rust Programs using Linear Ghost Types*, OOPSLA 2023 (PACMPL 7(OOPSLA1):286–315) · follow-up artifact: *Verus: A Practical Foundation for Systems Verification* (verus-lang, GitHub).
> **Date:** 2026-09-20
> **Status:** DISTILLED — pending owner decision (two-track verdict; executed follow-up = issue 127)

## TL;DR

Two-track verdict, one note, because both skills (research + distill) share the source.

- **Track A — distill/corpus: MARGINAL C+ as a mining source.** Verus is a *verifier*, not a lint-pattern source; the mineable surface is a **property taxonomy** (overflow-freedom / panic-freedom / bounds / invariant preservation). Yield after falsification: **1 rule** — the staged lead `unchecked-wire-arithmetic` (InputValidation 3→4). Zero perf rules; no plan, no batch opened.
- **Track B — research/FV-moat: Gain (intel + decision recommendation).** The three-way tooling trade recorded (Lean `.proofs/` vs Verus vs Kani). **Toolchain-adoption verdict: NO for Verus** — verification runs through Verus's own pinned rustc build, against this workspace's pinned stable `rust-toolchain.toml`s + wasm32 gates + per-repo dep allowlists. The actionable follow-up is **issue 127 — the Kani oracle POC**: Research 078 graded Kani B+ on the sec ORACLE ladder and it has never been RUN; the POC machine-checks one shipped sec rule's property claim and pre-validates the wire-arithmetic lead.

## What Verus is (facts, each attributable)

- Open-source **automated program verifier for Rust**, SMT-backed (Z3, per the CMU talk). Developers annotate Rust source with `requires`/`ensures` specs **inline, in Rust-like syntax**; stock rustc ignores the annotations, so annotated crates stay cargo-consumable by unverified projects (blog).
- **Linear ghost types** (the OOPSLA'23 contribution): proof code is tracked by the same borrow checker as exec code — a proof cannot be forged, duplicated, or silently dropped.
- **Sub-second per-function feedback** at production scale ("verify projects with thousands of lines of code and proof in the time it took some prior automated program verifiers to verify individual functions" — blog, attributed). The blog explicitly notes this makes **AI-agent proof iteration** practical; cf. KVerus (arXiv 2026), a retrieval-augmented system for Verus-based verification — AI-assists-proof is already a published subfield.
- Verifies **`unsafe` blocks** and **custom lock invariants** (concurrent correctness, not just data-race freedom) — the Nitro Isolation Engine use case; Amazon's related content pairs Verus with Isabelle/HOL for the hypervisor's separation kernel.
- Ecosystem (blog): Vest (binary-format parser codegen + proofs) · Verdict (x.509 validation) · CapybaraKV (crash-safe persistent-memory logs) · Atmosphere (verified microkernel) · Anvil (K8s controller liveness) · CortenMM (verified concurrent allocator).
- Toolchain cost: annotated code compiles under stock cargo, but the VERIFICATION step runs through Verus's own pinned rustc build (per the repo's release layout) — the load-bearing fact for Track B's refusal. The refusal is robust to the exact spelling either way (even a pinned-nightly-only requirement is a nonstarter against workspace toolchain pins).

## Track A — distill/corpus verdict

**Verdict line: MARGINAL C+ (SEC_SEED) / ORACLE B− / no lane mapping / product-coverage weak-fire.**

**Domain routing:** `sec` — explicitly NOT `rust_perf`/`kernel_opt` (nothing here moves cycles), NOT `clippy_lints` (spec macro syntax carries no lint shapes), NOT `rustc_errors` (killed below — proof failures are not rustc error codes).

### Ranked candidate table

| # | Candidate | Grade | Decision |
|---|---|---|---|
| 1 | `unchecked-wire-arithmetic` — `+`/`*` directly on `u32/u64::from_{le,be}_bytes(..)` results, no checked ops | B− | **KEEP — staged lead** (InputValidation, suggest-only; the fix must INVENT the error path — the `length-prefix-validate-before-alloc` shape) |
| 2 | Lock-invariant / concurrency-property annotations | KILL | no lint shape; ambient correctness property, not a bounded fix-space |
| 3 | Ghost/proof-syntax patterns from the Verus source tree | KILL | ambient verifier structure; source not cloned (see caveats) |
| 4 | Verus diagnostics → `rustc_errors` playbook content | KILL | proof failures are not rustc error codes; that domain repairs compile errors keyed by code |
| 5 | Property taxonomy (overflow/panic/bounds/invariants) as the sec family roadmap | recorded | mostly ALREADY-SHIPPED coverage: PanicSafety (`nan-unsafe-sort-comparator`) + InputValidation (3 rules); the delta is candidate 1 |

### Dedup gate (executed 2026-09-20, both vocabularies)

- Source vocabulary (`verus|spec|invariant|smt|program verifier`): the only mentions in this repo are Research 076 ("IronFleet/Verus are different provers/domains") + Research 078's oracle row. No corpus coverage.
- Operator vocabulary (`checked_add|from_le_bytes|wrapping_add|saturating_add|overflow` over `riir-clippy/src/**`): **zero corpus hits** — only an LCG test helper (`behavioural_equivalence.rs`) and the unrelated `REASONING_OVERFLOW` LLM-budget const. PASS.
- Neighbours, distinct fix-spaces: `length-prefix-validate-before-alloc` (alloc-size gate) · `unbounded-read-into-vec` (read cap) · `nan-unsafe-sort-comparator` (total order). Candidate 1 is arithmetic-overflow — a distinct class. `arithmetic_side_effects` (restriction) fires on ALL arithmetic, so the rule follows the corpus's narrower-than-restriction convention (the `drop-impl-no-panic` vs `unwrap_used` precedent) by scoping to wire-byte origins, parse-provable.

### Decision content + GOAT test

- Decision content: bounded fix-space (checked op + error path), regime conditions (untrusted wire ints vs internally-bounded counters), parse-provable detector (a byte-origin flowing into arithmetic in the same expression chain — bounded walk). PASS as **suggest-only decision content** — the Batch-136 NotBenchable-first-class precedent; sec's oracle is compile+semantics equivalence, not the perf Pareto.
- GOAT candidate test: **no GOAT — by domain.** Criterion 1 fails for a DOMAIN reason (sec rules do not Pareto-bench) → per-rule ANNOTATION, not a demotion; criteria 2/4 fine; criterion 3 (magnitude story) is N/A for a correctness rule. The value axis is decision content × dedup clearance, which the lead passes.
- Lane-intel axis: **does not fire** — Verus carries no perf mechanism; no mapping to the league lanes. Checked honestly, not assumed.
- Product-coverage axis: **weak fire.** Wire-int arithmetic lives in every wire parser (rustls/quinn/protobuf decoders/image decoders). InputValidation today = 3 rules; the lead grows it to 4. **Follow-up source pool** (accumulate into the same batch window, the 174/146 staged-charter precedent): RustSec integer-overflow advisories (class anchors, CC0) · the Vest-style verified-parser corpus · `checked_*`/`overflow-checks` stdlib + cargo-book docs.

**Yield estimate: 1 rule** after ~50% coordinator falsification. Taxonomy row 5 is already-shipped coverage, not new rules.

## Track B — research/FV-moat verdict

The stack's FV moat is spec-level: Lean 4 theorems across 4 `.proofs/` instances (KatgptProof / RiirAiProof / RiirChainProof / NeuronDbProof), Mathlib-free, `proof_gate.sh` asserting build + no-sorry + axiom budget + pinned theorem counts — **paired with Rust `spec_match.rs` tests on finite samples**, which is the moat's one honest gap. Verus is the published existence proof that the gap can be closed inline; the trade table decides whether it closes it *here*:

| Axis | Lean 4 `.proofs/` (shipped) | Verus | Kani (078: ORACLE B+) |
|---|---|---|---|
| What is verified | the SPEC (theorems: boundedness, conservation, determinism, monotonicity) | the Rust IMPLEMENTATION (functional correctness, inline) | impl PROPERTIES on real MIR (overflow/panic/UB/assertions), bounded unwind |
| Spec↔impl gap | bridged by `spec_match` tests on finite samples | none — the impl is what is checked | none for the checked properties (within bounds) |
| Toolchain | lean4 side-channel; cargo untouched | **Verus's own pinned rustc build** | its own pinned nightly via cargo-kani; repo toolchain untouched |
| Trust story | asserted axiom budget (proof_gate) | SMT + Verus soundness + stdlib models (blog names these) | solver trust bounded by unwind bounds + kani models |
| Concurrency/unsafe | out of scope | **in scope — the showcase** (lock invariants, unsafe safety) | limited (single-threaded) |
| Inputs | theorems (unbounded by construction) | unbounded (deductive) | bounded unwinding |
| Feedback loop | human-guided, slow per proof; CI = proof_gate | sub-second per fn; AI-agent-friendly | slow on real crates; harness-scoped |
| Fit | shipped moat | **refused for now** (fork vs pinned toolchains + wasm32 gates + allowlists) | middle path; POC-gated (issue 127) |

**Decision: do NOT adopt the Verus toolchain.** The reasoning: our verified surfaces (LatCal fixed-point round-trip, quorum-commit determinism, value conservation, freeze/thaw torn-read) are spec-proved and sample-tested; the marginal value of impl-level inline verification is real but does not outweigh a rustc fork across the workspace contract. If impl-level checking is wanted, Kani is the standard-toolchain route and is already on the ladder — the honest next step is to RUN it once (issue 127) before promoting or demoting its B+.

### Fusion — what Verus × Lean-.proofs × Kani produces that none alone does

The **property-evidence ladder**: Lean proves the property P (top-level spec, small axiom budget) → Kani machine-checks impl ⊨ P on the real MIR (bounded) → the sec corpus carries the *detectable subset* of P-violations as healable patterns. If issue 127 lands, the concrete fusion output is a **"property evidence" column** in the sec corpus doc table (harness id + verdict) beside the existing "killed neighbours (evidence)" column — the evidence-tier idea the traj_store Certified/Heuristic vocabulary already uses, applied to rule authoring. Verus's role in the ladder is the unbounded rung we refuse to pay for today; this note keeps it on the shelf with its price stated.

Intel side-nugget: **CapybaraKV** (Verus-verified crash-safe persistent-memory logs) is direct prior art for riir-neuron-db's LocalKv WAL + the dapps DoWal crash/chaos harnesses — a citation, not a mining lead.

## Cousins (prior art, workspace)

- `katgpt-rs/.research/292_Bridge_Neuro_Symbolic_Formal_Verification_Gap.md` — the "which prover" analysis (Lean vs Coq vs F* vs TLA+); this note extends it with the INLINE-VERIFICATION axis (Verus) and the model-checking axis (Kani) that 292's table lacks.
- `katgpt-rs/.research/198_Lean4Agent_Formal_Workflow_Verification.md` — decided Rust-native runtime predicates (a different layer: runtime belief predicates vs code verification).
- `riir-clippy/.research/078_sec_domain_seed_verdict_rust_secure_code_index.md` — the oracle ladder; Kani B+ graded there, never executed. Issue 127 executes it.
- `riir-clippy/.research/104_sec_phase2_unsafe_crypto.md` — the sec family phase-2 record (crypto hygiene shipped there).

## Caveats (honest)

- **Verus source NOT cloned** (no `.raw/`): grades rest on the blog + paper metadata + repo docs. No byte-level quotes are cited, and none are needed for a taxonomy verdict; the wire-arithmetic lead's fixtures will be ORIGINAL syntheses (license hygiene + the binary-only distribution rule) and require no Verus source read.
- The blog is Amazon-house framing (Parno); the "under a second" and "thousands of lines" numbers are the blog's own claims, quoted attributed, cross-checked only against the paper's existence/venue and the ecosystem projects' public descriptions.
- Kani's B+ is 078's grade, not re-measured; its cost profile (slow on large harnesses, own nightly pin) is exactly what issue 127 exists to measure. A NEGATIVE POC outcome is a valid result and demotes the ladder rung accordingly.
- The rustc-fork point is stated from the repo's release layout; if it were wrong in the direction of "standard nightly suffices", the Track-B refusal survives anyway (a pinned-nightly requirement is equally a nonstarter against the workspace's pinned stable toolchains).

## Recording

This commit: research note (this file; `.research/.highwater` 192→193) · a queue-snapshot line in `.distill/001_mining_queue_snapshot.md` · issue 127 (Kani oracle POC; `.issues/.highwater` 126→127). No plan, no batch: track A is MARGINAL (1 staged lead; a batch opens plan-first only if the owner pulls it), and track B's follow-up is the POC issue. Queue slot: post-mining intake, dated 2026-09-20.
