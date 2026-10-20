# Plan 270 — Batch 266: `isop-cover-from-decision-diagram-not-path-simplification` (rust_perf, complexity)

**Status:** OPEN — staged by the distill verdict (Research 302, 2026-10-21), NOT claimed. The next idle-cycle claimant appends the Claimed line and works the tasks in order.
**Claimed:** zed-m3-katopz-idle, 1792540800, expires 2026-10-23T00:00:00Z

Source: astral-sh/uv PR #22651 "Extract marker DNF covers from decision diagrams" (head `551c17c` / merge `2978e76`, MIT OR Apache-2.0) — memoized Minato–Morreale ISOP extraction from the hash-consed marker decision diagram, replacing path enumeration + the quadratic clause-subset simplifier. Verdict: YES B / no lane mapping / seeds the decision-diagram family (corpus first). Full record: `.research/302_distill_uv_pr22651_marker_isop.md`.

Expected yield: 1 own rule (`isop-cover-from-decision-diagram-not-path-simplification`, category `complexity`, suggestion-only) + 2 count-neutral rider faces in context_notes (the shared `Cover` intermediate; the `(lower, upper)` don't-care interval memo). rust_perf 241 → 242.

## Tasks

- [ ] T0 — claim: append `**Claimed:** <session>, <epoch>, expires <ISO-UTC>` under this Status line, push BEFORE any edit (48 h expiry; push is the arbiter; run `git fetch origin && py ../katgpt-rs/scripts/dual_allocation_gate.py` at claim).
- [ ] T0b — pin: shallow-clone `astral-sh/uv` into `.raw/uv` at the merge commit (resolve the FULL shas for head `551c17c` and merge `2978e76`); verify the LICENSE files (MIT OR Apache-2.0); re-verify EVERY quote carried from the verdict (Research 302's quoted lines) byte-for-byte at the clone before any corpus edit; the clone is ephemeral — removed at close (T12).
- [ ] T1 — dedup re-verification at authoring time, 3+ vocabularies per candidate: `isop|shannon.?expansion|quine|espresso|decision.?diagram|\bbdd\b|robdd|cover.{0,20}extract`; `dnf|cnf|sum.?of.?products|product.?of.?sums`; `memoiz|dont.?care|interval.{0,20}cache|hash.?cons|interner`. Zero owners expected (Research 302 evidence stands); any hit gets READ and adjudicated in the entry's distinct-from set before authoring.
- [ ] T2 — author the rule in the next free rust_perf entries file (verify `ls src/domains/rust_perf/corpus/` at write; nominal `entries_37.rs`): category `complexity`, suggestion-only, `bench_spec` ARMED. The rule body carries: the two-regime statement (sharing-heavy hash-consed diagram vs sparse/tiny), the anti-regime set (no interner available; unique-canonical-DNF consumers; byte-stable-output consumers; variable-ordering soundness contract), the regime trade numbers (1.5× two-clause / 10,500× conflict-32 / −5.5% tiny-marker regression as the honest cost line), the provenance chain (PR #22651 @ the pin ← Mishchenko IWLS03 §4 / Minato–Morreale), and the superseded-trio lesson (#22640–22642: simplify-after-enumeration is the wrong layer).
- [ ] T3 — fixtures ORIGINAL (never paste uv code): a minimal hash-consed decision-diagram interner (~40 lines, `NodeId` + memo table); buggy arm = path enumeration + quadratic clause-subset simplification (the DELETED simplify's SHAPE re-authored: subset checks + negation checks over `Vec<Vec<Lit>>`); fixed arm = memoized interval-ISOP recursion; the equivalence-oracle test walking all 2^n assignments (n ≤ 12 in-fixture) + roundtrip-through-the-representation + prime/irredundant spot asserts. Fixtures validated standalone (`rustc --test` scratch crate) before the corpus lands (the B265 discipline).
- [ ] T4 — rider faces in the rule's `context_notes` (count-neutral): (a) the shared `Cover` intermediate (the OUTPUT of the conversion is Rc-shared and guard-merged on equivalent suffixes, expanded only at the leaf renderer); (b) the `(lower, upper)` don't-care interval memo key (covered sub-cases become don't-cares; memo soundness requires the consistent variable ordering).
- [ ] T5 — GOAT bench on the rust_perf_bench Pareto validator: sizes {conflict-dense, sparse, tiny} × n {1K, 16K, 262K}, interleaved arms, alloc axis. ⛔ G1 is the EQUIVALENCE ORACLE, never byte-identity — covers are not unique; make_input sample-parity asserts both arms agree on the FUNCTION before any timing. Quote the bench_preflight `PROVENANCE:` line beside the numbers. A Dominated verdict at any size gets an honest rule downgrade or drop (the Batch-248 drop precedent).
- [ ] T6 — standing gates: `batch266` parse + needle tests in `src/domains/rust_perf/corpus/tests.rs` (needles verbatim from the re-verified quotes); count re-pins 241→242 at ALL sites (mod.rs module header, tests.rs asserts, AGENTS.md ×2, README, `.docs/11_domains/rust_perf.md` ledger) + the annotated/unannotated/benchable splits if they move.
- [ ] T7 — clippy `-D warnings` ×3 postures (`--all-targets` default / all-features / no-default).
- [ ] T8 — retrieval floors re-measure at an isolated `CARGO_TARGET_DIR=/tmp/b266-target`; re-pin with attribution only if moved (a new rule span should land top-1 for its own needle — assert it).
- [ ] T9 — score-bench run appended (rust_perf delta OWED, plan 137 T7.5); freeze regen NOT owed (zero clippy-corpus delta — the B255 precedent).
- [ ] T10 — maintained-binary rebuild + install at the release set (rust_perf IS in the release set — the B255 lesson; `--version` build hash == HEAD before push).
- [ ] T11 — docs: Research 302 landing addendum (status → the batch record), AGENTS.md rust_perf bullet, `.docs/11_domains/rust_perf.md` batch row, queue-snapshot LANDING row (this plan + the research cited).
- [ ] T12 — commit `feat:` on develop, push; plan ticked COMPLETE; `.raw/uv` and `/tmp/b266-target` removed.

## Process notes (standing discipline)

- rustfmt per-file only, never whole-file on shared data-table files (the Issue-167/B250 lessons).
- Watch the CRLF class on new `.md` files written by the edit lane (B255 instance #7) — LF-correct before commit.
- Rust string literals in entry bodies: single quotes inside, no raw unescaped doubles (the B247 class); `\n` spelled deliberately (the B262 splice lesson — read the diff after every multi-target edit).
- If CodSpeed-class upstream numbers move (the draft follow-ups #22665/#22666 merge), re-read the rider faces before landing — the plan is dated to the `551c17c` pin.
