# 390 — the doc-section pair (missing_errors_doc + missing_panics_doc): coverage-debt authoring, plan 354

**Status:** LANDED (2026-11-09, plan 354 / Batch 346 — corpus 115→117, both `--fix`-eligible VERIFY-tier; the landing commit carries the batch record; the /tmp/i354_probe probe scratch deleted after recording)

## Source of record

Not a distill — an INTAKE landing: Bench 141's coverage-debt evidence join (`missing_errors_doc` 90 + `missing_panics_doc` 78 — the only two dual-evidence rows: the local Bench-061 fired histogram AND kornel's 100K-crate top-10) × plan 354's claim-time probe matrix (captured at pinned 1.99.0, commit `00c4cb68`, the lapsed zed-native-agent claim superseded per the 48 h expiry law).

## The probe matrix (re-verified live at the landing, clippy 1.99.0, `/tmp/i354_probe`)

| doc shape | clippy verdict |
|---|---|
| exported `Result` fn, no section (or NO doc at all) | FIRES |
| `# Errors` / `# errors` / `## Errors` / `#  Errors` (extra space) | SILENT — accepted |
| `#Errors` (no space) | **FIRES** — not a section |
| `# ERRORS` (uppercase) | SILENT — accepted |

Predicate of record: `^#+\s+(errors|panics)` case-INSENSITIVE, whitespace REQUIRED after the hashes.

## The fix shape (one module, two lints — the Batch-151 precedent)

`proposers/doc_sections.rs`: exported-at-site fns only; `missing_errors_doc` appends a `# Errors` section naming the CONCRETE error type's last path segment (generic/opaque E refuses — prose naming a type parameter would be a lie, the bounded-fix-space law); `missing_panics_doc` walks the body for RECOGNIZED literal-message panic sources (`.expect("lit")` → "Panics when lit.", `.unwrap()` → the internal-unwrap line, `panic!/assert!` literal → "Panics with \"lit\".") and refuses loud on format-arg/todo-family shapes (inventing prose for an unreadable message is worse than none). VERIFY-tier deliberately: the append is comment-local (cannot change compilation) but GENERATES content — the verify lane's real-clippy re-run is the only proof the section satisfied the lint (the fast-path parse gate cannot see section-presence).

## The gates' findings (en-route, recorded)

1. **The cross-fire invariant weakened by STRUCTURAL ABSENCE** (the batch's own discovery): `clippy_fixture_labels_verify_via_proposers` + both `switch_cost` cross-domain tests assert every fixture fires NO other proposer — but a structural-absence lint fires on any exported fn lacking the section, so fixtures carrying `.unwrap()`/`Result` are GENUINELY non-compliant with it (true positives, not matcher looseness). Exempted as `STRUCTURAL_ABSENCE` in all three walks with the reason recorded; every other pair stays strict.
2. **The T3.5.4 circularity prediction re-registered**: All-mode top-1 == pool-recall held only under zero-cross-fire; with true-positive co-fires the prediction inverts to `All ≤ pool` (exceeding it stays the contamination signal). Measured: pool 95.8% (46/48), All top-1 93.8% — BELOW pool exactly as the new reading predicts; no re-pin (the retrieval floors HELD, `retrieval_precision_regression_gate` green at the latent posture).
3. **Freeze v2+v3 PASS** (fresh-clippy oracle over the corpus delta — the seeded fixture spans the generations cover didn't drift); score-bench gen-8 appended, heal rate 96.4% no decay; pins_manifest 115→117 (clippy_lints.rules), health_views corpus line + tree total 1601→1603 same-commit.

## Anti-record

`missing_const_for_fn` (the histogram's #1) stays KILLED (`.plans/050`); `must_use_candidate` (1011, the histogram's #2) was NOT staged — single-evidence (local only, absent from kornel's top-10), the evidence-join law stages dual-evidence heads first.
