# Plan 047 — Batch 86: the Cloudflare 1.1.1.1 DNS-cache memory distill (rust_perf)

**Status:** QUEUED — not started (verdict: Research 100, YES B+, route `rust_perf` allocation + layout; 2–3 rules + 2 riders)

Source: https://blog.cloudflare.com/dns-cache-memory-optimization-1111/ (2026-08-27). Five storage changes, per-entry 953→420 B, +43% insert / −19% lookup (END-TO-END — cite only the near-isolated +13% insert / −5% lookup). Blog-only, no repo. Executes after the sibling's Batch 85 (cflow, Plan 046).

## Tasks

- [ ] T0 Pre-flight: re-scan `.plans/` + `.research/` + `.distill/001_mining_queue_snapshot.md` at batch start (the collision lesson — re-verify 047/100/86 are still ours; push-wins tie-breaker if not)
- [ ] T1 Dedup re-verification (MANDATORY at mine time — the corpus moves between verdict and mine): re-grep `src/domains/rust_perf/` for the five candidates under both vocabularies (`box_str_immutable`, `Box<[`, `box_enum`, `offset|arena|length-prefix`, `elide|intern|Option<Box`, `bitflag|bool-fields`, `wire|serialize-once`); re-read any hit before calling it a duplicate; record one line of evidence per candidate in the batch record
- [ ] T2 Rule 1 `serialize-once-byte-buffer-store` (A− GOAT lead): original minimal fixture (frozen typed records → one length-prefixed `Box<[u8]>` + `u16` section offsets; NEVER paste the article's structs — license hygiene + binary-only rule); `bench_spec` on the insert path (timing) + the alloc-count axis (per-entry allocs ~1-per-record → 1); regime in-rule: sequential-rebuild consumers win, random-index consumers lose; anti-shape note: full-wire-message storage (DNSSEC DO-flag two-variant trap, from the source's own rejection)
- [ ] T3 Rule 2 `context-equal-field-elide` (B): `Option<Box<T>>` None = same-as-enclosing-context, inferred at read; fixture with a %-equal parameter; falsifiable at authoring — if it collapses into the dedup/intern family shape, drop with reason and fold the insight into rule 1's doc
- [ ] T4 Rule 3 `bitflag-pack-when-footprint-dominates` (B−, NotBenchable-speed): bools → bitflag for stored-at-scale structs; deterministic size axis (the `box-large-enum-variant` precedent, `bench_spec: None` + documented reason); cross-cite the kernel_opt `bool-fields-over-bitset-hot-reads` inverse regime; mine-time check the default-repr field-reorder interaction ("measure `size_of` before/after", never promise the collapse)
- [ ] T5 Rider `box-str-immutable` ← generalize to `Box<[T]>` (any frozen T); cite the source's 64 B/entry + ~100 TB as external validation in the doc comment
- [ ] T6 Rider `box-large-enum-variant` ← the honest costs (allocator size-class rounding, locality scatter, rare variant pays more) + the skewed-frequency regime (80% small / 20% large) in the rule's comment
- [ ] T7 Exhaustive-equivalence gate for any ported numeric/bit construction (the ported-numeric MANDATE) — expected N/A here (no numeric port); record the N/A reason if none
- [ ] T8 Standing gates: `pseudocode_parses` gate for the new entries (mind the `#[cfg(not(feature = "kernel_verify"))]` attribute + isascii() scoped to literal content only); category counts updated (allocation + layout); `KernelRule`/`PerfRule` struct field completeness
- [ ] T9 Count pins walk (the run finds what grep misses): rust_perf header count, category counts, benchable count, annotated counts (drafter + serde round-trip + the drafter VERDICT-TOKEN contract), test-name pins (`*_benchable` renames)
- [ ] T10 GOAT: G1 (byte-identity / size determinism), G2 (insert-path Pareto + alloc-axis exact integers), G3 (retrieval floors HELD — kernel/perf/clippy/blended; no re-pins), G4 (alloc-det stable), verdict-stable ×2; bench under a quiet box (check sibling compute load first — the Plan 337 discipline)
- [ ] T11 Docs: `.docs/11_domains/rust_perf.md` rule table + counts; `.docs/02_corpus/` header count line
- [ ] T12 Queue + research: add the Batch 86 line to `.distill/001_mining_queue_snapshot.md` (newest-first); flip Research 100's Status → RECORD with the landing pointer; bump highwaters in the same commit as the files that consume the numbers
- [ ] T13 AGENTS.md rust_perf feature-blurb count update (the standing post-batch edit)
- [ ] T14 Commit + push on `develop` (`feat:` prefix for corpus+code, `docs:` for notes); commit-sweep guard before committing (`git diff --cached --name-only | grep -v <my-paths>` must be empty); never sweep sibling WIP

## Notes

- Generator lessons in force: the edit_file JSON layer strips one escape level in string-heavy entries (use the python heredoc + `rustlit()` path); grep-verify every new rule's `bench_spec`/`context_notes` lines landed; the bench COMPILE is the only type-check new snippets get (pseudocode_parses is necessary, not sufficient).
- Numbers discipline: cite ONLY the near-isolated +13% insert / −5% lookup; the 43%/19%/953→420 are END-TO-END cache-flow.
- The article's own rejected alternative (full wire message) is content: it goes in rule 1's doc as the anti-shape, not a drop-row.
