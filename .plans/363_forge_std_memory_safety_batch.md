# Plan 363 — Batch 353: FORGE std memory-safety transcription (sec)

**Status:** LANDED 2026-11-13 — Batch 353 complete (the landing commit is the record; all tasks T0–T13 done)
**Lease closed:** zed-amo-b353-forge, 1794514228, expiry was 2026-11-14T20:10:28Z (landed well inside the lease)

## Summary

Transcribe the FORGE Lab "Memory Safety Bugs in Rust Standard Library" post
(2026-11-09, CC BY 4.0 — Research 397) into sec-domain suggestion-only rules.
The class is soundness hygiene of CONSUMER unsafe code; every rule is
NotBenchable-by-domain (the dial9/136 lane). Lead:
`unsafe-invariant-panic-exposed`. Target: 2–3 rules + the queue/docs updates.

Source pin: the blog post itself (no repo sha). Quotes in Research 397 are
from the fetched page; issue-level claims (#156501, #156491, #161018,
#156951, #164243, #158265, #165076, #164904) re-verify at mine time if quoted
byte-level.

- [x] T0. Re-read Research 397 + the source post Re-read Research 397 + the source post; re-run the dedup greps at the landing HEAD (Research 397's greps are dated — `ManuallyDrop|panic.safe|exception.safety|unwind` and `as_ref|Borrow::|environ|read.lock|invariant` over `src/domains/**`).
- [x] T1. Family placement decision Family placement decision: append-only `SecFamily::UnwindSafety` variant vs widening `PanicSafety`'s doc — no renumber (the corpus family law). Record the decision in the batch landing note.
- [x] T2. Author the lead Author the lead: `unsafe-invariant-panic-exposed` (suggest-only, NotBenchable) — the uninit-under-armed-Drop + user-call-in-window shape; three named fixes (`ManuallyDrop` until written / drop guard / finish-panicking-work-first) in the fix text; distinct-from in-rule: `drop-impl-no-panic` (inverse direction), `ffi-extern-fn-catch-unwind` (boundary containment), `panic-safe-autotune-candidate` (kernel_opt, crash isolation).
- [x] T3. Author `user-impl-call-once-keep-result` Author `user-impl-call-once-keep-result` (B+) — the Borrow/AsRef-bound method called twice, results assumed to agree; distinct-from `read-output-trust-inherits-argument-taint` (taint axis) and `unsafe-impl-send-sync-unsound-field` (marker axis). MERGE DECISION: if the T4 fixture work shows #3 cannot be separated from this one by a bounded gate, merge into one rule with both fixes and record why.
- [x] T4. Author `user-impl-derived-property-validate` Author `user-impl-derived-property-validate` (B) — unchecked constructor beside a trait-derived value (`from_utf8_unchecked` shape); fixes: validate / type-encode / unsafe trait.
- [x] T5. Gate-check `process-global-write-under-shared-lock` Gate-check `process-global-write-under-shared-lock` (B−): author ONLY if a bounded detector shape survives T0's greps and a fixture reproduces the race window deterministically; otherwise record the drop in the batch note (Research 397 already carries it as the swing candidate).
- [x] T6. Fixtures Fixtures: ORIGINAL consumer-shaped reproductions (never std code, never the post's snippets verbatim); each fixture standalone-compiled AND executed pre-entry — the lead's fixture must demonstrate the UB path with a panicking user call through a broken invariant and the `ManuallyDrop` repair (deterministic; `catch_unwind` harness).
- [x] T7. Provenance headers per rule Provenance headers per rule: source = the FORGE post URL + date + CC BY 4.0; upstream std fixes named as the origin bugs (decision content only — no PR intent).
- [x] T8. Standing gates Standing gates at the isolated target dir (`CARGO_TARGET_DIR=/tmp/b353`): pins_manifest agrees (61 → landed count), retrieval floors HELD both lanes, clippy `-D warnings` at default + all-features for the touched crate slices; sec is NOT in the release set — binary rebuild NOT owed (B325/B322 precedent, verify at landing).
- [x] T9. score-bench score-bench: sec suggestion-only rules shift suggest output; run `cargo refine --score-bench` and land the history row.
- [x] T10. Docs Docs: AGENTS.md §Role sec bullet (rule count moves in §Role per-batch chain — the counter lives THERE, never hand-typed here), README domain table if it carries a sec count, `.docs/` family notes if the UnwindSafety family lands.
- [x] T11. Research 397 status line Research 397 status line → `DISTILLED — batch-353-landed` (same commit as the landing).
- [x] T12. Queue snapshot landing row Queue snapshot landing row + dual_allocation_gate at the landing commit. Commit `feat(353): batch 353 — the forge std memory-safety transcription (Research 397, Plan 363)`; push; claim expiry honored.
- [x] T13. Outbound-PR screen Outbound-PR screen: N/A by construction (std upstream, decision content only) — record the screen result in the landing note.

## Landing record (2026-11-13, zed-amo-b353-forge)

- **T1 family decision:** the lead (`unsafe-invariant-panic-exposed`) rides
  `PanicSafety` with the DOC WIDENED to the exception-safety axis — the
  family already carries unwind-shaped members (`nan-unsafe-sort-comparator`
  is a foreign comparator exposing std's invariants;
  `panic-handler-nested-catch-unwind` is unwind hygiene); a separate
  `UnwindSafety` variant would be a synonym shard of panic-safety. Rules 2+3
  seed the NEW APPEND-ONLY `SecFamily::TrustedImpl` (the caller-provided
  VALUE-impl axis — no existing family owns it); rule 4 seeds the NEW
  APPEND-ONLY `SecFamily::ProcessState` (the post's Pattern 3, process
  globals at the OS/libc boundary). Labels: `trusted-impl`, `process-state`;
  `PanicSafety`'s label unchanged.
- **T5 gate-check verdict: AUTHORED** (above the research's B− swing
  estimate). The bounded shape survived: `env::set_var`/`remove_var` under a
  live `.read()` guard binding is textually precise and low-FP (set_var is
  rare + deliberate); the fixture reproduces the hazard deterministically on
  a toy RwLock (both writers hold read guards simultaneously — non-exclusion
  PROVEN — and the reader between the write and restore steps sees the
  child-only value, the post's own sentence). The WIDER axis (a `static`
  swapped under a read guard) is honestly OUT of the bounded shape — alias
  analysis cannot separate the locked data from an unrelated counter — and
  is carried as the rule's generalization, not the detector.
- **T13 outbound-PR screen: N/A by construction** — the source is std
  upstream (all origin fixes already landed upstream: #156501's commit,
  #156556's type-encoding, #157144 in review); the rules are decision
  content only, no PR intent.
- **Coexistence:** authored in the detached `/tmp/wt-b353` worktree at
  origin/develop (the shared checkout carried zed-amo-b352-lone's
  UNCOMMITTED B352 sec WIP — corpus.rs/proposers/mod.rs are in both
  write-sets; hunk-level rebase at push, the B358 precedent). Their WIP
  LANDED mid-session (`6f6e0363`, sec 61→65): the rebase carried BOTH
  landings — their Numerics block + my memory-safety block, the tests
  regions rebuilt sequential (B352 then B353), counts re-derived 65→69
  (pins_manifest --write re-run at the rebased head), health pins
  consolidated sec 69 / total 1644. En-route
  repairs: the E0392 unused-lifetime on `CallVisitor`, 2 doc_markdown
  mechanicals, the corpus fixed_code's unstable `transpose()` replaced with
  the stable `assume_init_read` form (fixtures + corpus), the origin-issue
  assertion widened to accept CVE-2020-36323/#155708 (the join rule's
  origins differ from the lead's).
- Session: zed-amo-b353-forge, epoch 1794514228.
