# Research 233: Percepta Spotlight — Growing Lattice Memory + Capability-as-Data

> **Source:** Percepta Research blogs, both 2026-10-02 (not peer-reviewed, no arXiv yet):
> (1) ["Spotlight Memory"](https://percepta.ai/blog/spotlight-memory) — Tzamos, Zheng, Jacob et al.; growing 2D-lattice sparse memory, linear cost, constant access.
> (2) ["Can LLMs Grow Their Own Capabilities?"](https://percepta.ai/blog/can-llms-grow-their-own-capabilities) — hand-compiled Python interpreter + capability-as-data inside a Spotlight-powered transformer.
> **Date:** 2026-10-03
> **Status:** ACTIVE — Gain, per-track verdicts; plans filed same session (riir-refine Plan 199 primary, riir-train Plan 440 probe).
> **Related Research:** katgpt-rs 387 (FwPKM), 511 (Memory Layers at Scale — the ancestor synthesis + moat record), 006 (Raven RSM), 278 (Engram), 455 (Hebbian); riir-refine 228 (RLTL;DR insight memory — the evidence-store consumer), 168 (probe-gated verification), 182 (frontier miner).
> **Related Plans:** katgpt-rs 408 (ProductKeyMemory, default-on); riir-train 358 (memory-layer KB adapters); **riir-refine 198 (this note's primary plan)**; **riir-train 440 (Spotlight 140M probe)**.
> **Classification:** Private (riir-refine; healer-domain distill files here per the MOAT table)

---

## TL;DR

Spotlight Memory is the newest member of the memory-layer lineage our stack already ships half of (Lample 2019 PKM → Memory Layers at Scale arXiv:2412.09764 → FwPKM 2601.00671 → Sparse Delta Memory 2607.07386 → MARCH 2608.12435). Its three lineage-new deltas: **(i)** 2D lattice spatial addressing with a compact-support cos² bump kernel — reads/writes touch a fixed 3×3 cell neighborhood regardless of memory size (locality + smooth interpolation, not independent top-k); **(ii)** each cell is a per-head DeltaNet associative state (cells are memories, not single KV rows) updated in place — the forgetting-MQAR variant reads 0.998 with zero stale values; **(iii)** allocate-on-first-write growth — capacity scales with distinct content while access stays O(9 cells). The companion demo hand-compiles (MILP, no SGD) a Python interpreter into a <100K-param intelligence module with >100M params of memory holding interpreter + packages + program state, at 134–144K tok/s flat over 47.6M-token traces.

**Distilled for the healer (modelless):** the *architecture* is trained and we do not chase it; the *addressing math* is fully deterministic-able — SimHash-split projection onto the lattice + LUT bump weights + closed-form delta-rule cell updates is a coherent modelless primitive (No-GD advocate: FOR, with 4 binding conditions, all adopted as gate design below). It is the candidate **fifth retrieval complexity class** (growing state + constant-neighborhood access + graded near-miss recall) beside Raven O(1) / Engram O(1) / δ-Mem O(r) / PKM O(√N). The capability demo VALIDATES the healer's founding thesis (corpus-is-the-model) and ports to a **WASM + Rust validator lane** — Percepta's Python-in-model becomes bounded fix-validators compiled Rust→wasm32, BLAKE3-committed into the corpus, executed in-process at µs class. Python appears nowhere, per owner directive.

**Not Super-GOAT.** Q1 fails at class level: the lattice-of-delta-cells class is dense published territory (SDM, MARCH, KV-Means, memory layers); what is new is the *combination in our stack* and the deterministic (no-GD) addressing form. Verdict: Gain (GOAT-tier on the modelless track), per-track separation below.

---

## 1. Paper core findings (condensed, numbers as published)

### Spotlight Memory (architecture)

| Claim | Number | Baseline |
|---|---|---|
| MQAR extrapolation (trained ≤64 pairs → 256) | 0.998 | attention 1.000, GDN 0.142 |
| MQAR capacity (131,072 pairs, 524K ctx) | 0.999 | attention 1.000 (quadratic), GDN 0.005 |
| MQAR forgetting (half keys overwritten) | **0.998, stale-value rate 0** | attention 0.748, GDN 0.002 |
| MAD suite mean | 0.787 | GDN-2 0.749, GDN 0.698, attention 0.664 |
| RULER S-NIAH-1 at 128K (16× train ctx, 8K-pretrained) | 93–100% recall | GDN ≤5.6%, attention 0% |
| Held-out loss 8K (670M) | 2.154 | GDN-2 2.157, GDN 2.142, attention 2.203 |
| 128K midtraining loss (670M) | 1.440 nats/tok | ~0.1 below both recurrent baselines |
| Cost model | memory grows with allocated cells; **O(T)** total, constant access/token | attention O(T²); fixed-state O(T) but capped capacity |

Mechanism: keys/queries → 2D lattice addresses; cos² bump `b(t)=cos²(πt/3), |t|<3/2` (normalized `φ_z(x)=b(x−z)/√Σ_j b(x−j)²`) distributes each read/write over exactly 3×3 cells; 2D routing separated from high-dim content (`exp(q₁k₁)·exp(q₂k₂) ≈ exp(q₁k₁)(1+q₂k₂)`); each cell = one d_k×d_v DeltaNet state per head; same learned projection/update rule reused as memory grows.

### Capability demo (hand-compiled, not trained)

| Claim | Number |
|---|---|
| Intelligence module | <100K matrix params (hand-compiled execution semantics + memory control) |
| Memory | >100M params: interpreter, core packages, program code, runtime state |
| Throughput | 134–144K tok/s flat from 6.5M to 47.6M-token traces |
| Runtime capability growth | "pip install" = model writes package source into memory; updates overwrite cells in place |
| Capability from memory | MNIST classifier with more parameters than the model's own weights, loaded in chunks and evaluated — predictions match reference on every example |
| Lineage | builds on their earlier WebAssembly-computer-in-transformer (Percepta transformer-vm — the same lineage our `katgpt-percepta` crate distilled: hull cache O(log n), Futamura projection, MILP compilation) |

---

## 2. Path 0 decomposition (inventory; No-GD advocate consumed in full)

| # | Component | Modelless-extractable? | Form it takes here |
|---|---|---|---|
| 1 | cos² bump + separable normalization | YES — closed form | 3-tap per-axis LUT (fraction quantized 2⁸), outer-product combine; zero per-call trig. Kernel shape is a **tunable** (cos² vs tent GOAT'd once), not a law — cos² exists for SGD differentiability |
| 2 | Constant 9-cell access | YES — property of addressing | latency-vs-N flatness gate (slope≈0) across 10⁴→10⁶ items |
| 3 | Routing/content factorization | YES | sigmoid substitution `σ(λ·content-penalty)` for the first-order term (house law: sigmoid, never softmax); rank-preservation probe |
| 4 | Allocate-on-first-write | YES — control flow | preallocated slab / papaya arena backing; counting-allocator proves 0 hot-path allocs |
| 5 | Gated delta-rule in-place overwrite | YES — rank-1 algebra, not GD | `S ← S(I−βkkᵀ)+βkvᵀ`; the forgetting semantics are algebraic (their 0.998 vs GDN 0.002) |
| 6 | Position-independent recall | YES — structurally | SimHash addresses are position-free; recall-vs-interposed-writes W flat gate |
| 7 | Two-stage addressing | YES | coarse deterministic pick + exact verification over the fixed 9 candidates |
| 8 | **Poisson collision/capacity law** | YES — the derived predictor | fixed total mass over 9 cells → closed-form multi-writer occupancy → offline grid-sizing rule (advocate C2) |
| 9 | Small-logic / large-memory split | YES — the blog is itself the modelless existence proof | host-side fixed intelligence (drafter/pruner/oracle lanes) + lattice as the only growing mutable state |
| 10 | Chunked artifact storage | YES — plumbing | foreign artifacts (validator bytecode, embeddings) in addressable cells; byte-identity round-trip |
| 11 | Consolidation targets from kernel geometry | YES | closed-form merge (low-occupancy neighbors) / split (high-collision cells) triggers — no learned compaction policy |

**Advocate binding conditions (adopted as gate design):**
- **C1** — **continuous E2LSH per-axis addressing, never integer-split SimHash.** The rejected form (recorded because it is the obvious first idea): random-projection sign bits → two integers on a combinatorial grid — a 1-bit high-order change moves |Δx| ≫ 1, so lattice neighbors are NOT embedding neighbors, and sign bits yield no fractional part (the bump LUT would always read fraction 0, degrading to a fixed weighting over 8 unrelated cells — killing the near-miss property Claim A depends on). Adopted form: per-axis `(a·x + b)/w` with seeded `a` (p-stable/Gaussian), `b`, width `w` — continuous before the floor, integer part = cell coordinate, real fractional part = bump input, per-axis 1-D locality preserved (the classic E2LSH construction). Gate the claim itself: off-cell queries must recover through neighbor weight, measured vs Engram; if the fixed design shows no locality, Claim A restates as "Engram with delta-rule cells".
- **C2** — grid sized by the Poisson multi-writer law + an explicit byte budget, gate replays measured histogram vs prediction. ⚠ Occupancy caveat: E2LSH projections of real embeddings are ~Gaussian, not uniform — central cells overload to capacity first, so the uniform law under-predicts collisions exactly where recall collapses; the fix is a monotone per-axis Gaussian CDF before quantizing (locality-preserving) or an uneven-occupancy law (Plan 619 T1.5).
- **C3** — per-cell read then blend (9 × d_k MACs each), never sum-the-9-states-then-read (smears interference).
- **C4** — gate against OUR baselines (Engram, PKM) at OUR key distributions; the paper's 0.998 rides a learned router and is an external anchor only.

**Honest failure modes (from the advocate, carried into the plan):** near-collinear delta-rule interference accumulates across overwrites (cell capacity ≈ d_k associations — collision-rate × write-count is the governing product); hard collisions are NOT soft (rank-d_k cells fight — abrupt, not graceful; the bump's grace applies to near-neighbor addressing only); zero-alloc backing must be enforced, not assumed; recall transfer to our key distributions is exactly what the gates decide.

**Coverage side (what already ships — signal-diff in §4):** delta-rule writes (δ-Mem/Plan 053), δ-write episodic store (PKM Phase 5), consolidation (sleep), hashed span embeddings (rule_embed two-tower, `span_embed`), SimHash (riir-auth session fingerprints), frozen committed tables (FrozenProductKeyMemory). No row above ships the *combination*; each cousin's delta is stated in §4.

**Training-target decomposition:** the trained half (learned addressing end-to-end, 140M–670M pretraining, 128K midtraining) is NOT modelless-extractable — it is genuinely a training contribution. → per-track verdict (c), riir-train Plan 440.

---

## 3. Vocabulary translation

| Paper term | Codebase equivalent | Where it ships |
|---|---|---|
| Spotlight memory cell | delta-rule cell (`DeltaMemoryState` rank-1 update) | `katgpt-core/src/delta_mem/` (Plan 053); PKM episodic δ-writes |
| 2D lattice address | E2LSH continuous per-axis coordinates `(a·x+b)/w` — integer part = cell, fractional part = bump input | new; nearest shipped: `katgpt-pruners/src/lsh_cache.rs` `SimHashFingerprint` (Plan 220) — 64-bit binary SimHash + Hamming-radius bucket lookup over logit vectors. Same LSH lineage, different address form (binary Hamming vs continuous bump) — consume/extend its projection + seed discipline, do not build a third LSH. ⚠ name disambiguation: NOT `katgpt-core/src/analytic_lattice/` (Plan 330 transport operators — unrelated module; a later vocab grep on "lattice" must not conflate the two) |
| bump kernel weights | 3-tap LUT interpolation | new; nearest: IDW scoring (PKM Plan 408 optional mode) |
| growing addressable memory | allocate-on-first-write sparse grid | new; nearest: Engram table, papaya-indexed stores |
| intelligence module / memory split | fixed drafter+pruner+oracle / corpus+trajectory store | `riir-refine/src/draft/`, `self_evolve.rs`, `traj_store.rs` — the healer's founding shape |
| pip install / capability-as-data | corpus rules + committed validator artifacts | mining batches, `frontier_miner`, corpus entries |
| hand-compiled interpreter | WASM bytecode + in-process interpreter | `katgpt-percepta/src/wasm/` (decoder/lowering/interpreter, Futamura), wasmi lanes (vessel/artb) |

---

## 4. Signal-diff vs the four shipped classes + published cousins (§3.6)

| Cousin | Signal it consumes | This note's delta (one read of the formula) |
|---|---|---|
| PKM (`ProductKeyMemory`, O(√N)) | span embedding → two √N codebooks → independent top-k | access cost grows with N (√N); top-k has **no locality structure** (two neighbors-in-embedding can land in disjoint slots); cells are single rows. Lattice: 9 cells constant in N + graded interpolation + per-cell associative state |
| Engram (O(1) hash) | embedding → exact bucket | 1-bit key change ⇒ different bucket ⇒ **zero recall** (hash cliff). Lattice bump: near-miss projections earn partial weight — graceful near-neighbor degradation. Engram cells carry no delta-rule accumulation |
| Raven RSM (O(1) slots) | routing over a FIXED slot set | parked because fixed capacity; lattice grows by allocation, access still constant |
| δ-Mem (O(r) fixed matrix) | single fixed-size associative state | no growth — capacity = state size; lattice = many cells, each δ-rule, allocated on demand |
| Published: SDM arXiv:2607.07386 | sparse top-k state expansion for linear RNNs | trained routing (no lattice locality, no bump interpolation, no allocate-on-first-write); differentiable-kNN flavor. Closest published cousin — cited, not beaten here |
| Shipped: `lsh_cache::SimHashFingerprint` (Plan 220) | logit vectors → 64-bit SimHash → Hamming-radius bucket near-miss | the in-stack LSH lineage. Binary Hamming near-miss IS a graded miss response — but: discrete bits (no continuous coordinate ⇒ no bump interpolation), flat buckets (no 2D locality, no per-cell delta-rule accumulation, no growth geometry). Lattice consumes/extends its projection + seeded-projection discipline rather than forking a third LSH |
| Published: MARCH 2608.12435, KV-Means 2605.09877 | content-routed / clustering-based expansion | same class; KV-Means is the closest *allocation* cousin (blocks allocated by clustering — trained). Ours is deterministic SimHash + closed-form sizing |
| Published: Memory Layers 2412.09764 (Research 511) | product-key lookup, trained values | Research 511's moat record (frozen-table + δ/Hebbian-constructed values unclaimed) EXTENDS: lattice addressing over constructed cells is likewise unclaimed *in our deterministic form*; the trained form is theirs |
| Percepta transformer-vm (their prior work) | WASM-in-transformer, hull cache | our `katgpt-percepta` already distilled it (hull O(log n), Futamura projection). Spotlight's demo adds *growing memory* to that lineage — the hull cache is O(log n) in history; lattice access is O(1) in everything |

## 5. The capability lane — WASM + Rust, NOT Python (owner directive)

Percepta's demo: capability-as-data with a hand-compiled intelligence module and a Python interpreter in memory. Healer port, keeping every kernel we already own:

| Percepta element | Healer WASM-lane element |
|---|---|
| Python interpreter (hand-compiled semantics) | in-process pure-Rust WASM interpreter (`katgpt-percepta/src/wasm/` — already shipped; wasmi fallback) |
| packages (`pip install`, in-place update) | bounded fix-validators / repair playbooks compiled **Rust → wasm32**, stored as BLAKE3-committed, versioned corpus artifacts — the freeze/thaw shape we already trust |
| fixed <100K-param intelligence | the fixed modelless lanes (drafter, pruner, oracle feed) — unchanged |
| runtime capability growth | mining batches / frontier_miner emit validator artifacts alongside fixtures |
| in-place overwrite | artifact hot-swap (atomic, versioned); **withdraw on inverted pass-rate** — `tier_guard` Lemma-4.2 LR test reused from rule-rows to validator-rows (Research 228's tier fusion, third axis) |
| MNIST-from-memory | validator executed pre-oracle: µs-class bounded checks (span invariants, format predicates) gate BEFORE a seconds-class clippy re-run |

Candidate first consumer: the P21 oracle-edits lane and `--fix-compile` (post-edit invariants), where today verification = full `cargo clippy` re-run. A µs-class committed validator that is fixture-floored (catches every violation class it claims, planted fixtures, zero misses required) is a real cost win; anything fuzzier stays with clippy. The honest comparator set has THREE arms — WASM interpreter, the same predicate compiled natively into the healer (likely faster, zero boundary cost), and the clippy re-run — and the lane only earns its place if the committed-artifact hot-swap (capability-as-data, no rebuild) is worth the interpreter overhead over native. Phase-gated; a "native is fine" outcome is the likelier negative and is recorded as such.

## 6. Fusion section

1. **Lattice × δ-rule cells × bump interpolation** — near-miss-graceful growing evidence memory: Engram's hash cliff → graded neighborhood; PKM's O(√N) → 9 cells. The candidate 5th retrieval class.
2. **Lattice × Poisson sizing × sleep consolidation** — closed-form split/merge triggers from kernel geometry (advocate #11); consolidation becomes measurement-driven, no learned policy.
3. **WASM validators × freeze/thaw × tier_guard** — capability-as-committed-data with a withdrawal path: an artifact whose measured advantage inverts (LR test) is revoked the same way Certified tiers are — Spotlight's "update knowledge in place" with our integrity machinery.
4. **Spotlight × corpus-is-the-model** — validation record, not code: the healer's founding thesis (fixed intelligence + growing corpus) is now demonstrated at frontier-lab scale with constant-cost recall. Cite in moat docs; changes nothing structurally.

## 7. Verdict

**Novelty claims (pinned per §1.5 precondition):**

- **Claim A (lattice):** *A deterministic 2D lattice of delta-rule cells with compact-support bump interpolation over continuous E2LSH per-axis coordinates `(a·x+b)/w` (allocate-on-first-write, in-place overwrite, Poisson-sized grid under an explicit byte budget), for the healer's span-conditional evidence retrieval, consuming span embeddings, distinguished from PKM by constant-neighborhood-vs-O(√N) access + locality interpolation vs independent top-k, from Engram (and from the shipped binary `SimHashFingerprint` LSH) by continuous-coordinate near-miss recall with per-cell delta-rule accumulation vs hash/bucket cliff, from Raven by unbounded growth vs fixed slots, from δ-Mem by a growing cell lattice vs one fixed matrix — at 10⁴–10⁶ stored items. If the E2LSH design measures no locality, the claim restates as "Engram with delta-rule cells" and is re-gated on that weaker form.*
- **Claim B (WASM lane):** *Bounded fix-validators compiled from Rust to wasm32, BLAKE3-committed into the healer corpus and executed in-process at µs class as a pre-oracle gate with a fixture-floored false-pass floor (an empirical floor over the enumerated violation classes — a proof path would need Lean or exhaustive bounded-domain coverage, neither claimed) — the capability-as-data pattern ported to a code healer with WASM instead of Python. Honest competitors, both gated: native Rust (the same predicate compiled into the healer — faster than any interpreter, no boundary cost) and `clippy_verify` (seconds-class whole-crate re-run). WASM earns its slot only through hot-swap-as-data without rebuild — the gate measures interpreter-vs-native overhead AND the swap value, not just µs-vs-clippy.*

**Novelty gate Q1–Q4:**

| Q | Answer | Evidence |
|---|---|---|
| Q1 no prior art | **PARTIAL** — the trained class is dense published territory (SDM, MARCH, KV-Means, memory layers — §4); the *deterministic addressing + constructed cells* form is unclaimed per the Research 511 moat method and §4 sweeps; in-stack the combination is new against all four shipped classes | §4 table; searches below |
| Q2 new behavior class | **NO** — the healer already grows capability via corpus (corpus-is-the-model); Spotlight *validates*, does not add a class | §6.4 |
| Q3 product selling point | **PARTIAL** — "our healer's evidence memory and validators grow without cost growth or retraining" strengthens the existing moat row; not headline-new | — |
| Q4 force multiplier | **YES** — composes PKM/δ-Mem/Engram baselines, sleep consolidation, traj/insight stores, tier_guard, frontier_miner, and (game-side) NPC memory + auth fingerprints as future consumers | §6 |

**Per-track verdicts (never pooled):**

- **Track (a) modelless — Gain, GOAT-tier.** Open primitive `lattice_memory` → `katgpt-core` (leaf-clean, feature-gated opt-in), healer consumption behind the same flag → **riir-refine Plan 199** (primary; serving-envelope: the decision layer runs inside the heal hot path). Promote only on the C1–C4 gates.
- **Track (b) self-adaptive — folded into (a).** δ-rule lattice writes ARE the legal latent-update class (no base weights touched); lands as Plan 199 Phase 2 semantics.
- **Track (c) model-based — conditional GO on the cheap probe only.** 140M @ ~5B tokens ≈ 15 GPU-h on the 4090 + eval harness (~1–2 d); 670M/50B ≈ 3–4 wk; retrofit/27B = NO (paper is from-scratch-only; 24 GB cannot hold 27B@128K); **deciding factor is the serving fork** (GGUF arch + lattice kernels in llama.cpp + Metal ≈ 4–8+ wk — a deliberate owner-gated project). → **riir-train Plan 440** with pre-registered kill criteria. Not deferred — filed (owner rule: do not defer GPU tasks; this is the filed instrument).
- **Capability demo — validation record.** Moat-doc citation only.

**MOAT gate (per domain):** riir-refine **primary** (consumer-first moat: retrieval floors + fixer verification cost — in scope); katgpt-rs hosts the open primitive (memory slot; per-stack ledger: feature + bench + promote/demote vs PKM/Engram at the same slot); riir-train hosts the probe; riir-ai/riir-auth recorded as future consumers (NPC lesson memory, session fingerprints) — not filed this session.

**§4 prior-art searches run:** internal grep `[Pp]ercepta` + `[Ss]potlight` across all `.research/` (4 prior Percepta notes found: katgpt-rs 000/001/004/006 — the transformer-vm lineage, none covers Spotlight; zero Spotlight hits); web: "Memory Layers at Scale sparse key-value" / "growing memory transformer constant access" / "Percepta Spotlight arxiv" / "Sparse Delta Memory arxiv". Landscape found and cited: SDM 2607.07386, MARCH 2608.12435, KV-Means 2605.09877, GDN/GDN-2 (2412.06464, 2605.22791), plus the internal lineage (387/511/408) and the shipped LSH substrate (`katgpt-pruners/src/lsh_cache.rs`, Plan 220 — reviewer-surfaced; consumed/extended, not forked). No blog arXiv exists yet — the blogs are cited as primary source; revisit on arXiv posting.

**Advocate ledger:** both advocate findings consumed (No-GD: 11 structures + C1–C4 conditions + failure modes → §2 and Plan 199 gates; model-based: recipe/GPU-h/kill-criteria → Plan 440). **Zero discards — nothing needed an auditable discard reason.**

**Benchmarks run this session: none** (research-only; all GOAT claims are gate designs, not results — box-state rules apply when Plan 199/440 execute).

---

## 8. Cross-references

- **riir-refine Plan 199** — lattice primitive consumption + WASM validator lane (this session).
- **katgpt-rs Plan 619** — the upstream home of the primitive itself: `crates/katgpt-core/src/lattice_memory/` + its GOAT bench + the per-stack ledger (filed this session after reviewer finding: code in the public upstream repo needs its own katgpt-rs document for the promote/demote ledger to live in).
- **riir-train Plan 440** — Spotlight 140M probe with kill criteria (this session).
- katgpt-rs `.research/387`, `.research/511`, `.plans/408` — the PKM lineage and moat record; Plan 199's gates gate against `ProductKeyMemory`/Engram directly.
- riir-refine `.research/228` (RLTL;DR) — the insight store consumer + tier_guard withdrawal fusion; `.research/182` (frontier miner) — the validator-artifact emitter.
- Prior Percepta distills: katgpt-rs `.research/000/001/004/006` + `crates/katgpt-percepta/` (hull cache, Futamura projection, WASM interpreter) — the transformer-vm lineage Spotlight builds on.
- Revisit trigger: Percepta posts an arXiv version (code/kernel details would sharpen Plan 440's reference-impl estimate).
