# Plan 199: Spotlight Fusion — Lattice Evidence Memory + WASM Validator Lane

**Status:** ACTIVE — Phase 0 (nothing implemented; all gates are designs until measured; primitive implementation owned by katgpt-rs Plan 619). **UPSTREAM MEASURED 2026-10-07 (Bench 919, katgpt-rs `9eec34e80`): Plan 619 T1.1–T1.8 LANDED — floor NOT PROMOTED at 10⁶** (G5's fixed 0.05σ bar is beyond the 10⁶ point's support radius; the governing law `N_max ≈ 0.35·d_k·(3.76/R)²` = 63,336 items at d_k=32, R=0.05σ). **T1.A read:** G1/G2/G3/G6 green and G5 WINS vs Engram at the 0.05σ bar for N ≤ ~33k (measured at 10⁵: lattice 0.105 vs Engram 0.002 — graded recovery at the bar itself); NOT green at 10⁵+/10⁶ (support shrinks below the bar). **T1.C did NOT fire mechanically** (the G5-vs-Engram win EXISTS at consumer scale — the failure is the 10⁶ envelope, i.e. a default-promotion negative, not a primitive-absent negative). The Phase-2 proceed-or-drop is therefore a RESCOPE decision (evidence-memory slot within ≤ ~33k items at R=0.05σ) — left as the owner call, facts in `.docs/03_memory/lattice_memory.md` + `.benchmarks/919_lattice_family_goat.md` upstream.
**Date:** 2026-10-03
**Research:** [riir-refine/.research/233_Percepta_Spotlight_Growing_Memory.md](../.research/233_Percepta_Spotlight_Growing_Memory.md)
**Source:** Percepta blogs 2026-10-02 (Spotlight Memory; Can LLMs Grow Their Own Capabilities) + the PKM lineage (katgpt-rs Research 387/511, Plan 408)
**Constraint:** **Rust + WASM only — no Python anywhere** (owner directive). Substrates: `katgpt-core` (existing non-optional dep) for the primitive; `katgpt-percepta/src/wasm/` (optional path dep) or `wasmi` for execution — substrate decision is Phase 3 T3.1, boundary-checked.
**Target:** `../katgpt-rs/crates/katgpt-core/src/lattice_memory/` (Phase 1, new feature `lattice_memory`) + `src/self_evolve.rs` / `src/draft/` consumption (Phase 2, same flag) + `src/validators/` (Phase 3, new `wasm_validator` feature).

---

## Goal

Fuse the two Spotlight blogs into the healer, per-track: (1) extract the lattice-addressing MATH as a modelless open primitive — a 2D lattice of delta-rule cells with compact-support bump interpolation over SimHash-split addresses, allocate-on-first-write, Poisson-sized — and consume it for span-conditional evidence retrieval (traj/insight stores) where the corpus grows unboundedly; (2) port the capability-as-data pattern to a WASM validator lane: bounded fix-validators compiled Rust→wasm32, BLAKE3-committed corpus artifacts executed in-process at µs class as a pre-oracle gate. GOAT gate per phase; promote only on wins; demote the loser per slot.

## Phase 0 — Prereads

### Tasks

- [ ] **T0.1** Read `../katgpt-rs/BOUNDARY.md` + this repo's `BOUNDARY.md` before any dep/module addition (boundary-guard skill at Phase 2/3 wiring).
- [ ] **T0.2** `substrate-first` vocabulary pass: grep `lattice|bump|SimHash|lsh_cache|SimHashFingerprint|neighborhood` across `katgpt-core/src/` (**including** `analytic_lattice/` — Plan 330 transport operators, unrelated domain, disambiguate), `katgpt-pruners/src/lsh_cache.rs` (`SimHashFingerprint` + `LshApproximateCache` + `seed_from_config` — the shipped LSH lineage: consume/extend its projection + seeded-projection discipline, never build a third LSH), `riir-auth/src/session.rs`, `delta_mem/`, `product_key_memory/` (Research 233 §3/§4 is the current record; re-verify at implementation time).
- [ ] **T0.3** Read Plan 408's episodic store + `delta_mem/state.rs` APIs — the cell update and the gate baselines come from there (consume, never fork).

Primarily owned upstream by **katgpt-rs Plan 619** (`crates/katgpt-core/src/lattice_memory/` + its GOAT bench + per-stack ledger) — this plan's Phase 1 below is the CONSUMER-BRIEF mirror; implementation tasks live in 619, this plan consumes its artifact.

## Phase 1 — `lattice_memory` open primitive (katgpt-core; implemented under Plan 619)

Implementation is owned UPSTREAM by **katgpt-rs Plan 619** (`crates/katgpt-core/src/lattice_memory/` + its GOAT bench + the per-stack ledger) — the addressing (E2LSH per-axis `(a·x+b)/w`, bump LUT, delta-rule cells, Poisson+CDF sizer, G1–G6 gates) is fully specified there and is NOT repeated here. This phase is the consumer's acceptance contract only.

### Consumer acceptance criteria (Phase 2 starts only when all hold)

- [ ] **T1.A** Plan 619 gates **G1 (recall@1), G2 (latency flat vs N), G5 (near-miss recovery vs Engram), G6 (RSS-vs-N within the byte budget)** are GREEN — G5 is the Claim A differentiator and is load-bearing for the healer's evidence semantics; G3 (overwrite/forgetting) green too, since evidence updates rely on in-place overwrite.
- [ ] **T1.B** The API surface Phase 2 consumes exists and is leaf-stable: `LatticeMemory::sized(items, collision_budget, byte_budget)` · `write(embedding, evidence_row)` (delta-rule accumulate) · `query(embedding) -> blended evidence` (per-cell read-then-blend) · freeze/thaw-compatible snapshot (BLAKE3-committed, atomic swap — the `FrozenProductKeyMemory` shape).
- [ ] **T1.C** Negative branch: if Plan 619's floor fails (no G5 win vs Engram, or locality absent → the claim restates as "Engram with delta-rule cells"), Phase 2 is DROPPED and this plan records the negative — no healer wiring without a measured primitive win.
- [ ] **T1.8** Kernel A/B once: cos² vs tent bump on the same G1–G3 cells (advocate: kernel shape is a tunable); pin the winner, demote the loser.

## Phase 2 — Healer consumption (evidence memory)

### Tasks

- [ ] **T2.1** goat-audit + boundary-guard at wiring time (the plan wires a katgpt-rs primitive into riir-* — run the audit before the first healer commit).
- [ ] **T2.2** First consumer: the issue-140 insight store (span/task-conditional evidence) — lattice index over `(span_embedding → evidence rows)` with delta-rule accumulation per cell; fallback consumer if 140 hasn't landed: `traj_store` evidence lookup keyed by `span_embed`. Feature forward `lattice_memory` (implies nothing new — katgpt-core already a dep; zero new deps this phase).
- [ ] **T2.3** Quality gate: score-bench / retrieval-eval A/B — top-1 and heal rate with lattice evidence vs current path, paired, same seeds. No regression tolerated on any existing floor.
- [ ] **T2.4** Growth-curve gate: synthetic 10× corpus — evidence-lookup cost flat vs corpus size (the PKM-at-scale argument, measured on our store).
- [ ] **T2.5** Promote/demote decision per slot from T2.3/T2.4 (per-stack ledger discipline); consolidation rider: closed-form split/merge triggers (Research 233 §2 #11) — bench post-consolidation recall no-regression + shrink ratio.

## Phase 3 — WASM validator lane (Rust → wasm32; no Python)

### Tasks

- [ ] **T3.1** Substrate audit + boundary decision: (a) which WASM MVP opcodes `katgpt-percepta/src/wasm/` interpreter actually covers (it was built for the compile lane — may be a subset); (b) `wasmi` as fallback. Adding either dep needs the BOUNDARY.md allowlist row + boundary-guard run; record the decision + reason. If BOTH are refused by boundary, this phase ends here with the audit as the record.
- [ ] **T3.2** Validator authoring lane: Rust → wasm32-unknown-unknown build step producing a bounded validator module (deterministic, no imports beyond a fixed ABI: bytes-in / verdict+score-out). One real first validator: post-edit span invariant for the P21 oracle-edits lane (e.g., the `useless_format` brace-escape predicate class).
- [ ] **T3.3** Committed artifact store: BLAKE3-committed, versioned validator artifacts in the corpus store (freeze/thaw shape); hot-swap on version bump; **withdrawal path** — `tier_guard` Lemma-4.2 LR test over measured validator advantage; inverted validator auto-revokes (Research 233 §6.3).
- [ ] **T3.4** **PoC gates** (all hard, THREE-arm comparator): (G-cost) verdict p99 on a 10⁴-span fixture set across **WASM interpreter vs the same predicate compiled NATIVELY into the healer (the honest baseline — likely faster, zero boundary cost) vs the `cargo clippy` re-run baseline**, measured on the same box with `../riir-reflex/scripts/bench_preflight.sh` provenance (port the script if the sibling path is absent — never skip provenance); (G-value) the committed-artifact hot-swap (capability-as-data without rebuild) is quantified — if native wins G-cost AND no hot-swap consumer materializes, the lane's verdict is the "native is fine" negative; (G-safety) **fixture-floored false-pass floor** — planted-violation fixture family (every violation class the validator claims to catch) caught 100%; one false pass fails the gate permanently. This is an empirical floor over enumerated classes, not a proof (a proof path would need Lean or exhaustive bounded-domain coverage — not claimed).
- [ ] **T3.5** Verdict: promote `wasm_validator` (default-off → default only if both gates pass AND a second independent validator lands) or record the negative + demote. Report-only integration into `--fix` run summary either way.

## Phase 4 — Docs + cross-repo record

### Tasks

- [ ] **T4.1** `.docs/` entries: lattice primitive (11_domains or the retrieval doc), validator lane (05_clippy_verify sibling); README feature-table rows.
- [ ] **T4.2** Cross-repo records (not filed): game-side consumers (NPC lesson memory over lattice cells — Research 228's riir-ai 1022 fusion; riir-auth session fingerprints) noted in the research note's revisit list.
- [ ] **T4.3** doc-sync lines; commit + push per phase; box-state provenance quoted beside every latency number.

## Guardrails

- Feature-gated opt-in until every gate passes; `--no-default-features` posture compiles clean at every phase.
- Gate against OUR baselines (C4) — the paper's numbers are external anchors, never adoption bars.
- Phase 3 is boundary-gated: no dep lands without the allowlist row; if refused, the audit is the deliverable.
- Every latency figure carries box state (bench_preflight / load class) per AGENTS.md G2.
