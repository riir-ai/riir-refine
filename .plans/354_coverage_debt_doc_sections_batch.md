# Plan 354 — the coverage-debt dual-evidence doc-section batch: missing_errors_doc + missing_panics_doc

**Status:** LANDED 2026-11-09 (zed-m3-issue120-gate — the zed-native-agent lease CLOSED: expiry 2026-11-09T12:00:00Z passed with zero corpus commits from that lane since 11-07, superseded per the 48 h expiry law; corpus 115→117, T0–T7 all green; the record: Research 390 + the snapshot landing row) — Bench 141 T3 improvement queue item E, first stage: the two dual-evidence coverage-debt heads (`missing_errors_doc` 90 + `missing_panics_doc` 78, Bench 141's only dual-evidence rows). SERIALIZED behind plan 353 (landed first — the serialization held). **Lease closed:** zed-m3-issue120-gate, 1794232922, was expires 2026-11-11T14:02:02+00:00 (the 18-file working-tree rustfmt churn was NOT this batch's surface — staged by name throughout)

## The ask (Bench 141 T3, 2026-11-07)

The coverage report's evidence-joined reading law stages authoring by evidence strength. The ONLY two lints with dual evidence (local Bench 061 fired histogram AND the study/default-eco joins) are the pedantic doc-section family — the same family as the landed Issue-120 `doc_markdown` precedent (fired every landing by this crate's own `#![warn(clippy::pedantic)]` root). Lint-level reality: both are pedantic (allow-by-default), so the `lint_level` machinery gates the fixes to crates that actually enable pedantic — the self-selected audience that asked for doc standards. Zero default-posture cost.

## Probe matrix (captured 2026-11-07, pinned toolchain 1.99.0, real clippy, scratch crates — record per the P9/P11 intake discipline)

**missing_errors_doc** (fires on exported fn returning `Result<_, _>`):

| shape | verdict |
|---|---|
| doc'd fn, no `# Errors` section | FIRES |
| NO doc comment at all | FIRES (absence of docs = absence of section) |
| fixed form: section added | SILENT |
| section spelled `# errors` (lowercase) | SILENT (accepted) |
| section spelled `## Errors` (deeper level) | SILENT (accepted) |
| section spelled `# ERRORS` (uppercase) | SILENT (accepted) |
| section spelled `#Errors` (no space after hashes) | **FIRES** — not a section |

**missing_panics_doc** (fires on exported fn whose body contains panic sources — assert!/unwrap/expect/panic! family):

| shape | verdict |
|---|---|
| doc'd fn with panic sources, no `# Panics` section | FIRES (span names the first panic site) |
| fixed form: section added | SILENT |
| `# panics` / `## Panics` (case/depth variants) | SILENT (accepted) |
| `#Panics` (no space) | **FIRES** |
| no panic sources in body | SILENT (the proposer must refuse there — nothing to document) |

**Section-present predicate (probe-pinned, both lints):** any doc line whose trimmed form matches `^#+\s+(errors|panics)` CASE-INSENSITIVELY — leading hashes of any depth, whitespace after the hashes REQUIRED. The proposer refuses when this matches (nothing to fix; rewriting a compliant doc would be a no-op re-write of code the crate's own clippy accepts — the lint_level skip-direction doctrine).

## Tasks

- [x] T0 Serialization gate: confirm plan 353's P118 batch has LANDED (corpus 115 state committed, sibling's dirty files resolved) — rebase this batch's landing on it; re-run the probe matrix at the landed HEAD toolchain if the pin moved. NOT startable until then (the claim holds the lane).
- [x] T1 Protocol-A dedup grep: corpus + all domains for errors/panics doc-section rules (expected 0 — the doc family covers markdown/lazy-continuation/overindent/tab shapes only); record the grep in the research note.
- [x] T2 Corpus entries ×2 in `corpus.rs` (pedantic group): doc-flavored fixture vocabulary (centroid-defensive per the doc_markdown note), per-lint distinctive tokens so the `every_lint_retrievable_via_suggest` top-5 gate stays green against its own buggy_code.
- [x] T3 One proposer module `proposers/doc_sections.rs` housing BOTH rules (the Batch-151 two-lints-one-module precedent), edit-collecting via the shared `edit_emitter`:
  - shared: section-present predicate (probe-pinned above), exported-fn detection, doc-attr walk, insertion offset = after the last doc line, before the signature (byte-splice, everything outside the insertion identical).
  - `missing_errors_doc`: fire iff exported + returns `Result<T, E>` + predicate absent; content derived from E when E is a concrete type path (e.g. `Returns an error when the operation fails (see `std::num::ParseIntError`).`); REFUSE on type params / `impl Trait` / `Box<dyn …>` / opaque generics (content would be a lie) → generic-body stub only when E is unresolvable? NO — refuse loudly instead (the bounded-fix-space law: a wrong E in prose is worse than no section; those sites stay for the drafter/self-evolve lane).
  - `missing_panics_doc`: fire iff exported + body carries panic sources + predicate absent; content mechanically extracted: `.expect("msg")` → `Panics when <msg>`; `panic!("msg")` / `assert!(cond, "msg")` → `Panics with "<msg>"`; bare `.unwrap()` → `Panics if an internal unwrap fails`; multiple sources → one line each; unknown-shape sources (todo!/unreachable!) → refused (v1 divergence, loud).
- [x] T4 Tier + registration: FIXABLE vs VERIFY verdict recorded with reasons (current lean: VERIFY — the edit is compile-trivial (comment insertion) but CONTENT-generating; the verify lane's real-clippy re-run proves the lint actually resolved, which the fast-path parse gate cannot see for a section-presence lint); `FIXABLE_LINTS`/`VERIFY_LINTS` + the propose_fix dispatch arm + `LINT_GROUPS` rows ×2 (pedantic — the driver-agreement test verifies against the installed driver).
- [x] T5 Standing gates (the batch discipline, in-batch): corpus unit tests green (`all_buggy_codes_fire_their_lint` needs both proposers firing on their own fixtures); clippy `-D warnings` at default + release-set postures; `scripts/pins_manifest.py --write` (clippy_lints rules 115→117) landed WITH the corpus change; freeze-table regeneration (gen2/gen3 e2e — a `build_seed_corpus()` delta moves the stride windows); 5b2 retrieval gate + floor re-pins with the delta recorded (corpus growth re-ranks — the Batch-230 precedent); score-bench run after the corpus change.
- [x] T6 Docs: research note (next `.research` number ≥ 387), AGENTS.md clippy_lints count line (115→117 + the tier-count split), `.docs/11_domains/` clippy_lints row, Bench 141 queue item E status update.
- [x] T7 Commit+push (Session line in the body), `.distill/001` post-mining intake note if the landing reveals misses, binary rebuild check (the corpus is compiled into the shipped binary — the installed `cargo-refine` serves the old corpus until rebuilt; `cargo refine --version` build hash == HEAD per §0 of `scripts/self_evolve_audit.sh`), scratch target dirs cleaned.
