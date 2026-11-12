# Plan 363 — Batch 353: FORGE std memory-safety transcription (sec)

**Status:** OPEN — staged (verdict: YES B, Research 397, nominal Batch 353; no corpus edit before this plan)

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

- [ ] T0. Re-read Research 397 + the source post; re-run the dedup greps at the landing HEAD (Research 397's greps are dated — `ManuallyDrop|panic.safe|exception.safety|unwind` and `as_ref|Borrow::|environ|read.lock|invariant` over `src/domains/**`).
- [ ] T1. Family placement decision: append-only `SecFamily::UnwindSafety` variant vs widening `PanicSafety`'s doc — no renumber (the corpus family law). Record the decision in the batch landing note.
- [ ] T2. Author the lead: `unsafe-invariant-panic-exposed` (suggest-only, NotBenchable) — the uninit-under-armed-Drop + user-call-in-window shape; three named fixes (`ManuallyDrop` until written / drop guard / finish-panicking-work-first) in the fix text; distinct-from in-rule: `drop-impl-no-panic` (inverse direction), `ffi-extern-fn-catch-unwind` (boundary containment), `panic-safe-autotune-candidate` (kernel_opt, crash isolation).
- [ ] T3. Author `user-impl-call-once-keep-result` (B+) — the Borrow/AsRef-bound method called twice, results assumed to agree; distinct-from `read-output-trust-inherits-argument-taint` (taint axis) and `unsafe-impl-send-sync-unsound-field` (marker axis). MERGE DECISION: if the T4 fixture work shows #3 cannot be separated from this one by a bounded gate, merge into one rule with both fixes and record why.
- [ ] T4. Author `user-impl-derived-property-validate` (B) — unchecked constructor beside a trait-derived value (`from_utf8_unchecked` shape); fixes: validate / type-encode / unsafe trait.
- [ ] T5. Gate-check `process-global-write-under-shared-lock` (B−): author ONLY if a bounded detector shape survives T0's greps and a fixture reproduces the race window deterministically; otherwise record the drop in the batch note (Research 397 already carries it as the swing candidate).
- [ ] T6. Fixtures: ORIGINAL consumer-shaped reproductions (never std code, never the post's snippets verbatim); each fixture standalone-compiled AND executed pre-entry — the lead's fixture must demonstrate the UB path with a panicking user call through a broken invariant and the `ManuallyDrop` repair (deterministic; `catch_unwind` harness).
- [ ] T7. Provenance headers per rule: source = the FORGE post URL + date + CC BY 4.0; upstream std fixes named as the origin bugs (decision content only — no PR intent).
- [ ] T8. Standing gates at the isolated target dir (`CARGO_TARGET_DIR=/tmp/b353`): pins_manifest agrees (61 → landed count), retrieval floors HELD both lanes, clippy `-D warnings` at default + all-features for the touched crate slices; sec is NOT in the release set — binary rebuild NOT owed (B325/B322 precedent, verify at landing).
- [ ] T9. score-bench: sec suggestion-only rules shift suggest output; run `cargo refine --score-bench` and land the history row.
- [ ] T10. Docs: AGENTS.md §Role sec bullet (rule count moves in §Role per-batch chain — the counter lives THERE, never hand-typed here), README domain table if it carries a sec count, `.docs/` family notes if the UnwindSafety family lands.
- [ ] T11. Research 397 status line → `DISTILLED — batch-353-landed` (same commit as the landing).
- [ ] T12. Queue snapshot landing row + dual_allocation_gate at the landing commit. Commit `feat(353): batch 353 — the forge std memory-safety transcription (Research 397, Plan 363)`; push; claim expiry honored.
- [ ] T13. Outbound-PR screen: N/A by construction (std upstream, decision content only) — record the screen result in the landing note.
