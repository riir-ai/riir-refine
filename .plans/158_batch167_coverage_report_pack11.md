# Plan 158 — Batch 167: the coverage-report re-arm pack XI (`unnecessary_cast` + `clone_on_copy`)

**Status:** IN PROGRESS — probe complete, corpus authoring next.

Frontier-lead mining pack XI for `clippy_lints`, following the Batch 164
coverage-report re-arm pipeline (the recorded-spare queue ran empty at
Batch 163; the re-arm derivation is the standing lead source). Leads from the
fresh `--coverage-report` derivation of 2026-09-21 (clean worktree at
`d8a2c0ec`, release feature set, clippy 0.1.98): 822 derived / 86 covered /
736 uncovered / **423 default-firing / 116 with evidence** (0 cross-domain
dedup collisions; 0 fix-set-only — the corpus-lockstep law holds from the
unmeasured side).

## Lead selection (probe-verified, `scripts/plan158_frontier_probe.py`)

| candidate | default-eco count | probe verdict |
|---|---|---|
| **unnecessary_cast** | 48 (top non-doc row) | **LEAD** — fires on proven-ident same-type casts (`p as u32` → `p`), annotated lets, floats, AND literals (`5 as u64` → `5_u64` — suffix-substitution sub-arm, never a bare drop). Cross-type casts fire OTHER lints (`cast_lossless` / `needless_type_cast` / `cast_possible_wrap`) — clean separation; the provenance must prove SAME type exactly. |
| **clone_on_copy** | 16 | **LEAD** — fires clean (`x.clone()` → `x` on proven-Copy), default-level confirmed; non-Copy clones fire `redundant_clone` (a different lint) — clean separation. |
| needless_borrowed_reference | 27 | **recorded REJECT** — all five natural pattern shapes SILENT under the full oracle (all+pedantic+nursery). Same posture as plan152's `mrp` reject. |
| double_ended_iterator_last | 17 | **recorded REJECT** — `iter().last()`, `into_iter().last()`, `rev().last()`, `values().last()` on BTreeMap all SILENT. Do not re-derive from memory; the probe is the record. |
| missing_safety_doc (186) / missing_errors_doc (90) / missing_panics_doc (78) | — | skip class: doc-AUTHORING, not a bounded mechanical fix (Research 191 adjudication). |
| len_zero (20) | — | NOT real debt: covered by our `comparison_to_empty` corpus rule under a different name — the name-keyed derivation counts it uncovered. Derivation alias gap, report-only. |
| result_unit_err / too_many_arguments | 28/28 | API-shape (architectural), not bounded. |

Fixture-design artifacts from the probe (applied to the corpus fixtures): the
annotated-let uc seed co-fired our own `let_and_return` (tail-expression
shape — the fixture uses the param shape instead); the call-expr seed co-fired
`missing_const_for_fn` (const-able helper — avoided).

## Tasks

- [x] T0 — probe the candidates with real clippy (`scripts/plan158_frontier_probe.py`, oracle-exact invocation; leads + rejects + group pins recorded above).
- [x] T1 — dedup greps: neither lead in any corpus (`cast_lossless` is a DIFFERENT lint — cast REWRITE vs cast REMOVAL; in-rule distinctness notes added). Derivation alias note recorded (len_zero ↔ comparison_to_empty).
- [x] T2 — this plan (plan-first protocol).
- [x] T3 — corpus entries ×2 (`corpus.rs`: `unnecessary_cast` complexity + `clone_on_copy` complexity, oracle-verified fixture pairs from the probe; corpus 86→88).
- [x] T4 — proposers ×2 (`proposers/unnecessary_cast.rs` + `proposers/clone_on_copy.rs`): the scalar-derivation family — uc = proven-ident same-type cast drop + int/float literal suffix sub-arm; coc = proven-Copy receiver `.clone()` drop. `scalar_param_heads_over` provenance + never-rebound guard; interior-comment refusals; module tests (seed-clean / fire / refuse / impl-method / comment).
- [x] T5 — registration: `proposers/mod.rs` (module docs table rows ×2 + `pub mod` ×2 + the FIXABLE list + the dispatch match arms) + the standing syn-parse gate extension (`batch167_pseudocode_parses`).
- [x] T6 — GOAT G1: new-rule module tests + corpus suite green.
- [x] T7 — GOAT G2: lib tests default + latent feature states, zero failures.
- [x] T8 — GOAT G3: clippy `-D warnings` in BOTH feature states + the CI guard.
- [x] T9 — freeze-table regeneration (corpus delta → the gen2/gen3 stride windows move; `src/score_bench/generations.rs` procedure, both `#[ignore]`d freeze e2es re-pinned and PASS 0 drift).
- [x] T10 — retrieval-precision re-measure (floors: kernel self-variants 100%, blended ≥ pins; a decay FAILS the batch).
- [x] T11 — score-bench run (+ the auto-appended frontier snapshot; double-run only if the verdict needs record capture).
- [ ] T12 — docs: AGENTS.md count rows (86→88) + the Batch 167 domain-list row; `.docs/` domain page; `.research/195` batch note (next free number at write time); `.distill/001` LANDED row + next-pool pointer.
- [ ] T13 — commit `feat: Batch 167: unnecessary_cast + clone_on_copy (clippy_lints 86→88)` + push.

## Standing gates (unchanged)

- G1 parse/syn on all new rules · G2 lib tests · G3 clippy `-D` both feature states + CI guard · freeze regen · retrieval floors · score-bench.
- Both leads are default-on groups (complexity) — pinned by the probe's no-flags arms.
- The fix-space is deliberately NARROWER than clippy (the documented divergence posture): uc v1 = param-provenance only (annotated-let widening is a later rider — the derivation substrate covers params; widening needs a let-annotation collector); literal sub-arm = int/float targets only (char/bool targets refuse — no suffix form exists); coc v1 = primitives + `&str`/`&String` receivers (arrays/tuples/`&mut` refuse — not in the proven map).
