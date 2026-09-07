# Plan 104 — Batch 124: the arXiv:2609.04040 matched-control evaluation-design distill

**Status:** RECORD 2026-09-07 — single rust_perf io rule landed (110 → 111), all gates green (lib 1166/0, clippy 0 ×2, retrieval floors byte-identical, score-bench FLAT 96.2/1.9), Research 147.

- [x] T0 — walk #8 verdict: YES batch (queued post-B123); abstract-only distill (evaluation-design rule, paper-cited, no code repo)
- [x] T1 — entry authored + `batch124_rule_present_and_parses` gate (the gate caught the case-sensitive needle + the ENOSPC-corrupted write)
- [x] T2 — count pins: total 110→111, annotated 53→54, io 14→15, header ledger ×3 surfaces
- [x] T3 — GOAT: lib 1166/0 + clippy 0 default/rust_perf + retrieval floors byte-identical + score-bench FLAT
- [x] T4 — Research 147 + AGENTS.md + `.docs/11_domains/` + snapshot row (same commit) + push

## Incident (recorded in Research 147)

Data volume hit 100% mid-batch (`/tmp/plan047_*` sibling dirs ~9 GB — untouched; my 46 GiB target cleaned). The ENOSPC write corrupted the appended rule (real newline inside the string + dropped paren); the parse gate caught it on the next build. Standing lesson: after ENOSPC, re-read the whole touched region.
