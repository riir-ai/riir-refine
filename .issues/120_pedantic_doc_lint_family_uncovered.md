# Issue 120 — Healer coverage gap: the pedantic doc-lint family is uncovered (observed live at the Plan 134 landing)
**Status:** OPEN — filed 2026-09-16 from the Plan 134 shader_perf session (post-mining intake class: the healer's own usage artifacts).

## The observation (usage artifact, not a survey)

Landing Plan 134 (`shader_perf`, 5 new files + 4 wiring edits) ran the healer
FIRST per the house rule:

```
cargo heal --fix --write --verify --verify-args "--features shader_perf" \
  src/domains/shader_perf/ src/domains/mod.rs src/lib.rs src/draft/domain_router.rs
```

It healed **5 edits** — `shrink-to-fit-after-build` ×3, `collapsible_if` ×1,
`unnecessary_map_or` ×1 — and was silent on **~35 findings** the same files
carried one `cargo clippy --all-targets` later, all in the doc/pedantic
family, all fixed BY HAND in the same session:

| lint | findings | manual shape |
|---|---|---|
| `clippy::doc_markdown` | ~25 | backtick-wrap identifiers in doc comments (9 files) |
| `clippy::doc_list_item_without_indentation` | 4 | list-continuation indentation |
| `clippy::doc_quote_without_marker` | 1 | `>` marker on quoted doc lines |
| `clippy::missing_errors_doc` | 1 | `# Errors` section |
| `clippy::missing_panics_doc` | 1 | (restructured away — the honest fix) |
| `clippy::format_push_string` | 1 | test-only `push_str(&format!(…))` |
| `clippy::must_use` suggestions | 2 | `#[must_use]` on two fns |

## Why it matters here specifically

This repo compiles with `#![warn(clippy::pedantic)]` at the crate root
(`src/lib.rs`), so **every new module landing generates this family** — the
healer being silent means a manual backtick pass on every domain/feature
landing, which is exactly the mechanical work the healer is supposed to own
(the 2026-08-24 GO verdict's whole premise).

## The coverage-report blind spot this evidences (Issue 119 tie-in)

`--coverage-report` (Issue 119) already lists `doc_markdown`-class lints in
its 768-UNCOVERED bucket, but its frequency axis joins only the Bench-061
histogram + kornel's top-10 — both measured on default-firing or
already-swept surfaces. An **allow-level lint that a repo CONFIG enables**
(pedantic-warn, here) is invisible to that axis by construction: first
measured run reported 0 of 449 default-firing gaps carrying any frequency
evidence, and this class is one axis over — evidence exists only in usage
artifacts like this one. Read: the intake queue is the only channel that
can see it.

## Candidate disposition (mine-time adjudication, not pre-decided)

`doc_markdown` is a bounded-fix candidate (backtick-wrap at token
boundaries) with real FP classes the miner must adjudicate against fixtures:
existing code spans, links/URLs, ALL-CAPS acronyms, attribute-shaped text
(`@compute`-style), and identifiers that clippy's config would rather
allow-list than wrap. Notable: clippy's own suggestion for the common shapes
is `MachineApplicable`, so once the lint is IN the corpus set the P21
oracle-edits lane can consume the suggestion verbatim — that lane refuses
out-of-corpus lints by design, so corpus membership is the gate either way.

Not filing a batch yet: single-session evidence, one repo's config shape.
The trigger to promote this to a mining batch is a SECOND landing hitting
the same manual pass (or the shader-toy editor lane, whichever comes first).

## Intake routing

Post-mining intake per AGENTS.md. The `.distill/001` §Post-mining intake
row is deliberately NOT added in the filing commit — that file carries
uncommitted sibling edits at filing time (shared-worktree discipline; the
row lands with the next batch commit that touches it).
