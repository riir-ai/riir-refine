# Issue 160 — CRLF line-ending contamination in tracked doc surfaces (cross-box flip-flop hazard)

**Status:** OPEN — repair IN PROGRESS: leg 1/8 DONE (riir-refine `7fbca3a7b`, 2026-10-11 — 82 files, 0 at HEAD); legs 2-8 deferred to the verified-idle window (live-lane files per protocol).

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
- [ ] legs 2-8 (katgpt-rs 23, riir-train 28, riir-ai 25, riir-infer 5, riir-reflex 2, riir-reflexer 1) — quiet window, live-lane deferral re-checked at execution time.
