# Proposal 012 — `riir-python-heal`: the Python perf-healing domain as a NEW REPO (sixth surface, first language split)

Status: **draft** (no plan opened, no code — owner decision pending)
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 001 (HealDomain meta-architecture) × Proposal 006 (the docker playbook: non-Rust language, external-oracle ladder, two rule classes, close condition) × Proposal 005 (KAT service network — the monetization rail a Python market multiplies) × Proposal 003 (binary-only distribution — the separate product surface) × [katgpt-rs Proposal 012](../../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md) (the repo-split precedent: a specialized healer as its own repo)
Related: [Proposal 001](001_multi_domain_modelless_code_healer.md) · [Proposal 002](002_real_file_span_healing.md) §Deep fusion A (ruff named as the confidence-less incumbent) · [Proposal 006](006_docker_heal_domain_fifth_healdomain.md) · [Research 003](../../katgpt-rs/.research/003_Commercial_Open_Source_Strategy_Verdict.md) (secrecy posture the new repo inherits)

## TL;DR

Open Python perf healing as a **new repo** (`riir-python-heal`, workspace repo 17) implementing a `PythonPerfDomain: HealDomain` over a **bounded-fix corpus** (string building, membership, iteration shapes, hot-loop hoisting, pandas/MLX pattern families) — perf/sec only, never style (style is ruff's territory and ruff is free). The money argument is already market-proven: astral built ruff/uv in Rust for Python developers (~150× flake8; the fastest-adopted Python tooling in years) — "Rust tool, Python surface" is the winning wedge, and the *healing half* (apply + verify with confidence, Proposal 002's exact critique: incumbents "emit ONE fix with no confidence") is unoccupied. The cost of the repo split is real and named: the `HealDomain` trait must be extracted into `crates/heal-core` (Plan 001's own "extraction is a move, not a rewrite" clause) so the new repo does not depend on the whole riir-clippy crate. Verdict: **GO**, mine-first, reject-if-covered close condition, no default-on until the GOAT gate passes.

## The problem this solves

1. **The market is 10× and we ship nothing to it.** riir-clippy heals Rust (+ Dockerfile). ~99% of the AI world writes Python; their perf pain is measured in GPU-hours ($/hr), not aesthetics — a fix that saves 20% of an MLX/torch loop has a wallet-pain number no style lint has. Proposal 005's KAT fixer-node economics scale with addressable market; Python widens it ~10×.
2. **The tool gap is pinned, same shape as docker's was**: ruff detects ~800 rules with mechanical autofixes (mostly style/correctness; fast, free); **perflint** does perf anti-patterns but is a pylint plugin that self-describes as false-positive-prone and ships **no fixes**. Nobody ships *perf* healing: detect → bounded fix → verify loop, modelless (no LLM bill, deterministic). That is the unoccupied cell.
3. **The corpus knowledge exists but is discipline-per-developer**: join-vs-concat, set membership, `iterrows`→`itertuples`, hot-loop attribute hoisting, MLX sync points (`.item()`/`mx.eval` placement, lazy-graph materialization). Same "knowledge exists, substrate doesn't" gap Proposal 006 closed for Dockerfiles.
4. **Context bloat is structural, not aesthetic** (the owner's ask): AGENTS.md, doc-sync, boundary-guard population, the mining-queue snapshot, bench books, and every sibling agent's context tax all operate **per repo**. A Python domain inside riir-clippy would flood riir-clippy's books with a foreign language's corpus + mining queue + benches. A separate repo keeps each surface load-bearing.

## The proposed design

### Repo shape (the split, and the one architectural cost)

```
riir-clippy/                       ← stays Rust-first; default build unchanged
  crates/heal-core/                ← NEW member: HealDomain trait + pipeline L0–L4
  │                                   skeleton + self-evolve memory + score-bench harness.
  │                                   riir-clippy re-exports it at the EXISTING paths
  │                                   (zero downstream import churn; count-identical tests).
  └─ feature python_heal = ["dep:riir-python-heal"]   ← opt-in registration arm

riir-python-heal/                  ← NEW repo (17th; own BOUNDARY.md, AGENTS.md,
  src/domain.rs        PythonPerfDomain: HealDomain   books, mining queue, benches)
  src/parser.rs        L2 = ruff_python_parser (pure Rust — ZERO Python at build time)
  src/corpus/          rule entries { rule_id, buggy_code, fixed_code, family,
                                      min_version, perf_metric, context_notes }
  src/oracle.rs        Tier 1 ruff binary shell-out; Tier 2 pytest smoke (external
                       binaries only — never cargo deps, never a build-time interpreter)
  bins: python-heal (--suggest / --fix --write --verify)  ← own product surface later
```

**Dep direction (cycle-free):** `riir-python-heal → heal-core ← riir-clippy`, plus `riir-clippy →(opt-in feature) riir-python-heal`. Package-level, no cycle: heal-core is its own package inside riir-clippy's workspace. Allowlist for the new repo: `heal-core` (path), `katgpt-core` (ternary drafter + `ConstraintPruner` — the public dep, same posture as riir-clippy's default), `ruff_python_parser` (crates.io, pure Rust). Fallback if extraction proves disruptive: the documented ~200-LOC local reimplementation of the drafter pattern (precedent: riir-clippy itself reimplements `TernaryDraftModel` locally), temporary trait divergence, converge later — extraction first, fallback second.

### Corpus: bounded-fix classes ONLY, min-version tagged

| Family | Example rules (bounded) | Class |
|---|---|---|
| string building | `+=` concat in loop → `join`; `%`/`.format` → f-string | A (perflint/ruff detect, fix-poor) |
| membership | `in [list]` per-iteration → `set` once | A |
| iteration | `range(len(x))` → `enumerate`/direct; `df.iterrows()` → `itertuples()` | A |
| hot-loop hoisting | attribute/global lookup → local rebind; `dict.get` vs try/except KeyError | A/B |
| pandas shapes | `pd.concat` in loop → list + single concat | B (novel fix) |
| **MLX family** | `.item()`/`.numpy()` sync points; `mx.eval` placement; lazy-graph surprise materialization; dtype flow (f16/bf16 on Metal) | B (novel — the money niche) |

**Decline classes (L0 bounded-fix-space discipline, verbatim from Proposal 006's anti-cargo-cult):** loop vectorization ("rewrite with numpy"), algorithm changes, async restructuring — suggest-only at most, default decline. These are rewrite classes; a heuristic that "vectorizes" is actively harmful.

### Oracle ladder (`python_verify`, the docker Tier pattern)

| Tier | Check | Cost | When |
|---|---|---|---|
| 0 | `ruff_python_parser` syntax + AST-shape equivalence | free, pure Rust | always |
| 1 | `ruff check` re-run (external binary on PATH) | ~100 ms class | `--verify`, Class-A agreement |
| 2 | pytest smoke of the healed module | slow | opt-in, `python_runtime_verify` |
| 3 | py-spy/perf wall-clock of the rule's `perf_metric` claim | slowest | GOAT gates only |

Tier 1 absent → degrade to Tier 0 with one log line (never silently claim verification).

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Ruff incumbency is the kill condition.** Ruff ships ~800 rules with mechanical autofixes, free. This proposal is GO only on the perf/verify-loop wedge; if the Phase-1 dry-run shows ruff (+ perflint) already covers ≥~90% of our corpus's yield on real repos, **reject or pivot to MLX/service-only** (the docker close-condition precedent, stated up front this time).
2. **Bench 009's scope limit applies verbatim, in public.** The L1 drafter is a cache over corpus-resident fixes; held-out exact-match was 0/10 and misses are silent. Python fixes are *more* template-shaped than Rust (no borrow checker interacting with the fix) so exact-match should be higher — but the deploy posture is unchanged: cache with a fallback path, never a capability claim.
3. **Version sensitivity is a correctness axis.** 3.11's specializing interpreter and 3.12+ comprehension inlining obsoleted classic advice; wrong-version advice is worse than none. `min_version` per entry is mandatory, and the corpus needs a rot-check (the percentile/count floor pattern) — a stale Python corpus fails loud, not quiet.
4. **The heal-core extraction is the riskiest step, not the corpus.** It touches every domain's imports in riir-clippy. Mitigation: count-identical test validation (the cargo-heal sweep precedent) and the fallback in §Repo shape. If extraction balloons past one focused PR, take the fallback — do not block the market bet on a refactor.
5. **perflint's false-positive reputation is a warning, not just a gap.** Every rule carries `perf_metric` evidence (measured speedup, M3, honest methodology) or it does not ship — the sec/docker falsification discipline, where 5 of 11 and ~40 of ~70 candidates died at mine time.
6. **No standing workspace consumer exists for Python.** docker earned default-on via riir-chain's CI adopting it; no riir-* repo is Python. The standing-consumer evidence must come from the KAT fixer-node service plane (riir-dapps) or the dry-run corpus repos — name the adopter BEFORE Phase 2, or Python stays opt-in forever (that is acceptable; opt-in still sells via the service).
7. **The repo-split justification is falsifiable.** If the corpus stalls under ~50 rules and the books stay thin, a module behind a feature would have sufficed and the split was ceremony. The split is justified by product surface (own binary + own public distro repo per Proposal 003, without touching cargo-heal's release hygiene) + queue volume, not by language alone.
8. **The L4 boundary holds.** No trained Python drafter in this proposal; the `l4_lora` seam stays inert (`decline` is the default). MLX-the-framework is never a dependency — GPU inference execution stays in riir-ai/riir-train; this repo heals Python *text*.

## Fusion lineage

- **Proposal 001** — the HealDomain container; sec + docker proved the trait holds for non-clippy and non-Rust targets, twice.
- **Proposal 006** — the non-Rust-language playbook adopted wholesale: two rule classes (Class A = incumbent-detects-we-fix; Class B = novel detection), external-oracle ladder with graceful degradation, close condition, consumer-pull promotion rule.
- **Proposal 005** — KAT service network: the Python domain is new fixer-node demand; Proposal 002's calibrated-UQ fusion ("which fix is correct") is the differentiator no incumbent has.
- **Proposal 003 + Research 003** — the new repo inherits source secrecy + binary-only distribution; a second public distro repo (the `gist-rs/cargo-heal` shape) is a Phase-3 decision, not Phase 0.
- **katgpt-rs Proposals 009/010** — the recorded negative: cross-language canonical directions are permanently demoted on hidden-state construction; this proposal does NOT touch model reuse across languages — the corpus is native Python text, the drafter is the same modelless ternary matvec for every domain.

## GOAT gate

| Gate | Floor | Decision |
|---|---|---|
| G1 correctness | Tier-0 parse + AST-equivalence on all corpus fixtures; Tier-1 ruff agreement on Class-A fixtures ≥ the docker bar (hadolint-agreement precedent) | required to ship at all |
| G2 perf | suggest latency µs-class on M3; ≥100× wall-clock vs an LLM fixer on the mechanical class; per-rule `perf_metric` reproducible at Tier 3 on ≥1 real repo | required |
| G3 no-regression | riir-clippy default build unchanged post-extraction (count-identical `--lib` tests; `cargo heal --score-bench` generation record unperturbed) | required |
| G4 alloc-free | drafter hot path zero-alloc after warm (the standing G4 shape) | required |
| Close condition | dry-run yield overlap vs ruff+perflint ≥ ~90% on the sampled repos | REJECT or pivot to MLX/service-only |
| Promotion | a named standing consumer (service-plane fixer node or dry-run corpus repo) running the domain in its loop — never count-only | default-on gated on this |

## What ships now (riir-clippy + new repo) vs deferred

### Ships now — Phase 0 (nothing until the owner says go)
- `crates/heal-core` extraction in riir-clippy (workspace member, re-export shim, count-identical validation).
- `riir-python-heal` scaffold: root `BOUNDARY.md` (owns: Python perf healing; allowlist: heal-core, katgpt-core, ruff_python_parser), AGENTS.md, empty books.
- `distill` verdicts on ruff + perflint as mining sources (corpus dedup grep FIRST — the standing distill discipline), filed as `.issues/`.

### Deferred — gated
- Corpus batch 1 (~20-30 rules), `--suggest` CLI, dry-run precision bench (Phase 1) — gated on the distill verdict being GO.
- `--fix --write --verify` + oracle ladder Tiers 1-2 + MLX family + `min_version` rot-check (Phase 2) — gated on Phase-1 precision.
- KAT service-plane rows (riir-dapps) + separate public distro repo + own release lane (Phase 3) — gated on the GOAT gate + a named consumer.

### Explicitly NOT shipped by this proposal
- Python→Rust (RIIR) translation — stays research-grade in katgpt-rs `.research/`; healing Python in place is the product, RIIR is the future upsell, not this.
- MLX/torch/any-framework execution or GPU work — riir-ai/riir-train own that; zero framework deps here.
- Style/formatting domains (black/isort parity) — ruff is free and wins; we do not re-ship it.
- Trained drafter (L4), lint-for-Go/JS/TS, and any default-on promotion before the gate.

## Phased rollout (sketch — a plan would expand this)

### Phase 0 — distill + extract + scaffold (riir-clippy + new repo)
- [ ] T0.1 `distill` verdict: ruff rule corpus + perflint as mining sources; dedup grep; domain routing (all python_perf)
- [ ] T0.2 heal-core extraction (re-export shim; count-identical `--lib`; score-bench record)
- [ ] T0.3 new-repo scaffold + BOUNDARY.md + allowlist rows both sides + boundary-guard/`repo_set.txt`/count-prose integration rows (derive the repo set; never hand-type it)
- [ ] T0.4 dry-run target selection (MLX examples + numpy/pandas-adjacent repos)

### Phase 1 — corpus batch 1 + suggest + precision bench
- [ ] T1.1 ~20-30 bounded rules, every entry min_version + perf_metric
- [ ] T1.2 `python-heal --suggest`; Tier-0 parser; negative control on clean repos (the sec discipline)
- [ ] T1.3 dry-run precision bench + ruff/perflint overlap measurement → close-condition verdict

### Phase 2 — fix + verify + MLX
- [ ] T2.1 `--fix --write --verify` + Tier-1 ruff oracle
- [ ] T2.2 MLX family + rot-check floor
- [ ] T2.3 GOAT bench (G1-G4) + promotion decision with the named consumer

### Phase 3 — service + distribution (gated on Phase 2 GO)
- [ ] T3.1 KAT fixer-node python_perf rows (Proposal 005 Phase-3 extension)
- [ ] T3.2 public distro repo decision (Proposal 003 shape)

## Risks

1. **Market risk (correctness of the bet):** ruff absorbs perf rules faster than we mine them → mitigation is the close condition + the MLX/service niche where ruff has no roadmap.
2. **Perf risk (the tool):** Python source chunking + parser on huge ML files could eat the µs-class claim → bench before Phase 2, chunker budget pinned.
3. **Architectural risk:** heal-core extraction couples the new repo to riir-clippy's pipeline evolution → the path-dep is versioned by the sibling layout convention; the fallback (local pattern reimpl) exists and is documented.
4. **Ecosystem drift risk:** CPython release cadence outdates rules → min_version + rot-check floor; a stale corpus fails loud.
5. **Attention risk:** the mining queue is already deep (Batch 32+); this adds a queue → distill-first, zero infrastructure before the Phase-1 yield proof.

## Out of scope

Jupyter/notebook healing (separate surface, later proposal); Python packaging (uv is free and wins); concurrency/GIL rewrites (unbounded class); any trained component; any change to cargo-heal's release artifacts; wasm32 target for the Python domain (parser dep weight — decide at Phase 2 with the wasm guard).

## References

1. **ruff** (cited-only) — docs.astral.sh/ruff; the market proof: Rust tool, Python surface, ~10-100× incumbents (crmarsh.com "Python tooling could be much much faster" measures ~150× vs flake8 single-core). Not a mining source for code — rule *ideas* only, MIT/Apache, mined under the same distill discipline as clippy lints.
2. **perflint** (cited-only) — github.com/tonybaloney/perflint; perf anti-pattern detection, self-described false-positive-prone, no fixes. The gap this proposal occupies; also the cautionary tale (precision bar).
3. [katgpt-rs Proposal 012](../../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md) — the specialized-healer-as-own-repo precedent (distilled).
4. [riir-clippy Proposal 002](002_real_file_span_healing.md) — the confidence critique of clippy/ruff/eslint --fix (distilled, quoted).
5. [Research 003](../../katgpt-rs/.research/003_Commercial_Open_Source_Strategy_Verdict.md) — "anything riir-* is internal"; the new repo inherits source secrecy + binary-only distribution (distilled, standing).

## TL;DR

**GO — new repo, not a module.** Next action when the owner says go: run the `distill` verdict on ruff/perflint (T0.1) and open Plan NNN for Phase 0 (heal-core extraction + scaffold). Until then: no code, no scaffold, no queue entries — this file is the whole deliverable.
