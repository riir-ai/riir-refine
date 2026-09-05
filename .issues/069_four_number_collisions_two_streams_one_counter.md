# Issue 069 — four duplicate numbers, and the cause is two work streams on one counter

**Status:** OPEN — measured, verdicts computed, **owner call to execute**. Plan 026 §Honest notes already deferred the `.research/083` half as an owner call on the grounds that "renumbering either 083 breaks live AGENTS.md references"; that concern is now priced and it is **2 sites**, not a barrier. The other three pairs were never gated.

## What was measured

Found by katgpt-rs `scripts/numbering_drift_sweep.py` (katgpt-rs Issue 725) — the
first sweep of the numbering gate over every contract repo instead of only
katgpt-rs. Four tracked duplicate numbers here; `Plan N` / `Research N` resolves
to two documents in each.

Arbitrated with katgpt-rs `scripts/citation_weight.py` per the Issue 724 T2 rule
(most inbound citations keeps the number). Every site below was READ, not just
counted.

| dup | KEEPS the number | MOVES | citations to rewrite |
|---|---|---|---|
| `.plans/015` | `015_kat011_phase0_payreq_substrate` (10) | `015_batch69_unaligned_word_load` (4) | `.research/075:47,51` · `AGENTS.md:198` (×2) |
| `.plans/026` | `026_kat_phase0_redaction` (8) | `026_dirty_encoding_write_elision_batch72` (3) | `.docs/11_domains/kernel_opt.md:119` · `.research/108:157` · `src/domains/kernel_opt/corpus/mod.rs:1370` |
| `.research/083` | `083_omlx_external_distill_batch71` (12) | `083_elo_selection_gate` (2) | `AGENTS.md:2344` (the link Plan 026 was worried about) · `.research/001:367` (`R083`, the starved-pool doctrine) |
| `.research/086` | `086_dockerfile_detection_inventory_hadolint_buildkit` (27) | `086_mnt_io_room_list_dod_batch73` (4) | `.plans/029:3,32,73` · `.research/108:153` |

Free numbers at measurement: `.plans/.highwater` **80**, `.research/.highwater` **132**.

## The cause, which is the part worth fixing

The winner is the **KAT product** doc in two pairs and the **mining batch** doc in
the other two, so this is not one stream losing — it is two streams **allocating
from the same counter without consulting it**. The batch docs run a contiguous
serial (`006_batch60`, `007_batch61`, … `039_batch81`) that is the *same*
allocator the `kat*` plans draw from, and a `kat*` plan landing between two batch
sessions takes a number the batch series is about to reach.

Renumbering the four is the cleanup. The standing fix is a decision this issue
cannot make for you:

- **(a) one counter, enforced** — every author reads `.highwater`, writes back
  `value + 1`, in the same commit. katgpt-rs `scripts/numbering_drift_sweep.py`
  reds on a duplicate above this repo's pinned count and on a `.highwater` below
  its directory max, so the discipline becomes checkable rather than remembered.
  (`numbering_gate.py` is the per-push half and is katgpt-rs-SCOPED — its floors
  file is that repo's, so pointing it here reports a population-floor breach, not
  a verdict. Use the sweep, or give this repo its own pins.)
- **(b) two directories** — `.plans/` and `.plans-batch/` (or a `b` prefix on the
  batch series), so the two streams cannot collide by construction.

(a) is cheaper and matches the rest of the workspace. (b) is the only one that
survives two agents landing in the same minute.

## Also fixed already, no action needed

`.plans/.highwater` read **78** while `080_*` existed — `value + 1` was already
taken, so the next allocation was **guaranteed** to collide. Repaired to 80 in
riir-clippy `9260441` (katgpt-rs Issue 725 T3). That is the mechanism that
produced these four, caught mid-act.

## Tasks

- [ ] **T1 — owner go/no-go** on the four renumbers above (13 citation rewrites total).
- [ ] **T2 — execute**, one commit per pair: `git mv`, rewrite the listed sites,
      add a renumber note to the moved document's header, bump `.highwater`.
      katgpt-rs `.plans/569`/`.research/363` are worked examples of the note format.
- [ ] **T3 — pick (a) or (b)** above and record it in AGENTS.md's numbering section.
- [ ] **T4 — lower the ratchet.** katgpt-rs `scripts/numbering_drift_floors.txt`
      pins `riir-clippy 183 4 0 0`; drop the `4` as pairs land, so a NEW collision
      reds instead of hiding under the backlog.

## Record

- katgpt-rs `.issues/725_cross_repo_numbering_drift_sweep.md` — the sweep, the
  instrument, and the six worked examples in riir-ai (all six resolved 2026-09-05).
- Run the numbers yourself:
  `python3 ../katgpt-rs/scripts/citation_weight.py . .plans 015 --margin 1 --show 40 --show-unresolved`
