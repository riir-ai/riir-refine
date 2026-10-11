# Issue 160 — CRLF line-ending contamination in tracked doc surfaces (cross-box flip-flop hazard)

**Status:** OPEN — repair IN PROGRESS: 6 legs landed (riir-refine `7fbca3a7b` 82 files; riir-reflex `9e517d2` 2 files; riir-reflexer `68cdaae` 1 file; riir-ai `ff56c5e113` 26 files; katgpt-rs `f2a4fd5a7` 23 files; riir-infer `e7b0e49` 6 files — all 2026-10-11, each repo census-at-HEAD reads 0); remaining leg (riir-train 28) gated on the 623 capture lane + routing restriction — live-lane re-check at execution time.

## The class

Agent edit tools on the Windows/4090 box write **CRLF**; the M3 writes **LF**. Two failure shapes, both measured:

1. **Wholesale flip (2 incidents, both caught + repaired same-session):** an LF-born
   file edited on the 4090 comes back with EVERY line's ending flipped — a 2-line
   content change lands as a 250+/250- whole-file diff that hides the real content
   from review and can mask sibling hunks (the `staged_set_audit` concern class).
   - `66a44c93e` → repaired `20bbbabc7` (the 16:5x snapshot row).
   - `f940fd763` → repaired `ba355fd53` (the 18:1x snapshot row).
2. **CRLF-at-birth (the standing census):** files created by 4090 sessions are CRLF
   from their first commit — internally consistent (edits diff cleanly) but they
   flip wholesale the day an M3 session edits them, and vice versa. The hazard
   grows with cross-box editing of the same files (the standard sibling-sync model).

## The census (2026-10-11, HEAD blobs, `git grep -l "$(printf '\r')" HEAD -- '*.md' '*.toml' '*.txt'` minus `.benchmarks/`)

| repo | doc-surface CRLF | bench-artifact CRLF (excluded) |
|---|---|---|
| riir-refine | 82 | 16 |
| riir-train | 28 | 41 |
| riir-ai | 25 | 136 |
| katgpt-rs | 23 | 32 |
| riir-infer | 5 | 17 |
| riir-reflex | 2 | 13 |
| riir-reflexer | 1 (rust-toolchain.toml) | 0 |
| riir-instinct | 0 | 2 |
| **total** | **166** | 257 |

`.benchmarks/` artifacts are generation-baked (Windows generators) and append-mostly —
excluded from repair scope: normalizing them is churn without an ongoing-edit hazard.

## The trap that let both incidents through

`git diff <path> | grep -c $'\r'` returned **0** on a CRLF file in this shell
(the `$'...'` ANSI-C quoting does not survive the command pipeline as written).
**Working check recipes** (use either):

```sh
file <path> | grep -q CRLF && echo DIRTY           # per-file
git grep -l "$(printf '\r')" HEAD -- '*.md'        # census at a ref
```

## Repair protocol (per repo, ONE commit, quiet window only)

1. NEVER normalize a file a live lane is editing. Deferred while their lanes are
   open: riir-ai `.issues/1051` (owner menu), `.issues/1054` (live sibling),
   riir-infer `.issues/039` (live sibling), reflex `.issues/085` (owner-gated) —
   re-check liveness at execution time, not from this list.
2. `sed -i 's/\r$//' <files>` (strips line-ending CRs only — no content byte moves).
3. **Verify zero content change** before committing:
   `diff <(git show HEAD:<path>) <path>` must show an endings-only diff (or nothing
   when compared with `--strip-trailing-cr`).
4. Commit `fix: restore LF line endings in <scope> (tool artifact, zero content
   change)` — the `20bbbabc7` / `ba355fd53` precedent shape.
5. Re-run the census line; the repo's doc-surface count must read 0 (or the
   remainder must be exactly the deferred live-lane files, listed in the commit).

## Open decision (owner)

Whether to add a standing gate (ratchet: doc-surface CRLF count must not increase)
once the normalization lands — a hard ceiling would red on the 166 standing files
today, so gate-first is wrong; normalize first, then ratchet. The reflexer
`rust-toolchain.toml` single hit may just be normalized en route (cargo is
ending-agnostic).

## Progress

- [x] 2026-10-11 riir-refine `7fbca3a7b` — 82 files normalized, per-file byte-verified endings-only (HEAD-minus-CR identical), census-at-HEAD reads 0; all files closed lanes (0 unchecked boxes); the .benchmarks/ 16 artifacts stay excluded.
- [x] 2026-10-11 riir-reflex `9e517d2` — 2 files (wire_contract_v1.md, .issues/085_drex_v15_revisit.md) normalized; `git diff --ignore-cr-at-eol` empty (zero content change); census-at-HEAD reads 0. Liveness re-check at execution: tree clean, 085 lane owner-gated on the Nace email (not being edited) — normalizing now is the protective direction against the future wholesale flip.
- [x] 2026-10-11 riir-reflexer `68cdaae` — rust-toolchain.toml normalized (cargo is ending-agnostic); census-at-HEAD reads 0.
- [x] 2026-10-11 riir-ai `ff56c5e113` — 26 files normalized (the founding census's 25 + `.research/397` born-CRLF the same evening); batch-verified endings-only: `git diff --ignore-cr-at-eol` EMPTY across the set (2253/2253 symmetric line flips); census-at-HEAD reads 0. Liveness re-check at execution: tree clean at commit; 1054's arc closed (0 unchecked boxes, 17:41 closeout 2h44m prior); 1051 owner-menu 8h-quiet (the reflex-085 protective precedent applied); 397 born-complete with its terminal 19:18 dedup verdict; all 13 `.plans/` + 6 older `.research/` files closed Sept-era records; `crates/riir-games-shared/clippy.toml` stable config.
- [x] 2026-10-11 katgpt-rs `f2a4fd5a7` — 23 files normalized (.docs/1 .plans/3 .research/19); batch-verified endings-only: `git diff --ignore-cr-at-eol` EMPTY across the set (2356/2356 symmetric line flips); census-at-HEAD reads 0. Liveness re-check at execution: the trainer Plan-621b WIP files (katgpt-core Cargo.toml + 3 new bench/test files) verified DISJOINT from the census set; newest census-file mtime 2026-10-07 (4+ days stale, no sibling edits) — the gate dissolved, leg executed while the trainer lane continues unaffected (their WIP untouched, still dirty post-commit).
- [x] 2026-10-11 riir-infer `e7b0e49` — 6 files normalized (.docs/002, .issues/039, .issues/041, .plans/618, .research/010, THIRD_PARTY_LICENSES.md — the founding 5 + 041 born-CRLF at 21:00 the same evening, the class caught in the act); batch-verified endings-only: `git diff --ignore-cr-at-eol` EMPTY (984/984 symmetric); census-at-HEAD reads 0. Liveness re-check: 038-lane WIP (22 dirty .rs files, all source) DISJOINT from the census set; newest non-mine census mtime 12:35 (039, sibling-committed 8.5h prior); edit+commit atomic against the b004f44e shared-index sweep class — verified post-hoc: the sibling's interleaved `2569a3f` (21:12, the .issues/038 update) swept nothing of mine.
- [ ] remaining leg (riir-train 28) — 623 capture lane + routing restriction (behind-3 + dirty .issues/623 by the watcher), live-lane re-check at execution time.
- [x] 2026-10-12 cycle-13 CRLF hazard re-scan (read-only, refine idle loop): census-at-HEAD **holds 0/6 repaired repos** through every post-repair commit (refine `fb839d6f`+`293de63d`, ai `88212f0f9e`+`43347b74e1`+`378f94cdff` incl. the BOUNDARY.md edit, katgpt-rs `a124bfac8`). Sibling-WIP endings: reflex `distill.rs` LF ✓ · train `.issues/623` LF ✓ · infer 4-file sample LF ✓ · katgpt-rs `Cargo.toml` (modified) LF ✓. **NEW class-2 instances detected (NOT normalized — live lane, protocol step 1): the trainer's 3 UNTRACKED katgpt-rs Plan-621 files (`bench_621b_healer_lane_eval.rs`, `tests/common/plan621_healer_sim.rs`, `tests/healer_lane_eval_t4.rs`) are CRLF-at-birth** — committing as-is plants 3 wholesale-flip hazards (out of the doc-surface census scope [.rs], rustc ending-agnostic, purely the cross-box flip + hunk-masking class); advisory: normalize before commit, or fold into this issue's execution-time re-check. En route: this box's edit tool flipped the refine snapshot to CRLF twice during cycle 12 — caught + repaired in-edit both times (the `file`+diff-shape check is load-bearing on every 4090 edit of an LF file).
