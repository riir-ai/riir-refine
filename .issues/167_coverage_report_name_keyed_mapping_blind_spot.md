# Issue 167 — coverage-report name-keyed mapping reads pattern-covered lints as UNCOVERED debt

**Status:** open — instrument honesty, report-only surface, fix small

`cargo refine --coverage-report` derives coverage by matching clippy lint NAMES
against corpus rule keys. But the corpus heals by PATTERN, not lint name — the
drafter matches spans, so a corpus rule can fully cover a lint it never names in
structured metadata.

## Measured precedent (this pass, clippy 0.1.98, binary bc13ca17)

- The report's UNCOVERED head lists `len_zero` (20 default-eco hits, style, warn).
- `src/domains/clippy_lints/proposers/comparison_to_empty.rs` heals exactly that
  pattern (`.len() == 0` -> `is_empty()`); `len_zero` appears only in its doc
  comments — no structured lint key — so the mapper credits nothing.
- Functional truth: the drafter DOES propose a fix for those spans. An agent
  staging from the report's dual-evidence head would author a DUPLICATE rule.

The harm is contained by Protocol A step 2 (dedup grep BEFORE proposing), and the
report is report-only / gates nothing — but the "coverage debt 715" number and the
staging head overstate authoring opportunity, and the seams+limits section does not
disclose the blind spot.

## Fix design (bounded)

1. **Disclosure (cheap, required):** a seams+limits line stating name-keyed
   coverage is a LOWER BOUND — pattern-covered lints read as UNCOVERED; cite the
   `len_zero` <-> `comparison_to_empty` precedent.
2. **Second-tier bucket (optional, better):** for each uncovered lint, grep corpus
   rule BODIES (doc comments + pattern literals) for the lint name / its
   documented alternates -> a `pattern-mentioned (uncited)` disclosure count, never
   folded into `covered`. Honest about being a text proxy, not semantics.

## Non-goals

- No semantic pattern-to-lint matching (undecidable in general; text proxy is the
  honest ceiling for a static pass).
- No change to the heal path — this is the suggest/ranking REPORT surface only.

## Verification

- The disclosure line lands in the report output.
- If (2) lands: `len_zero` appears in `pattern-mentioned`, never in `covered`;
  the existing buckets' counts stay unchanged (membership both directions).
