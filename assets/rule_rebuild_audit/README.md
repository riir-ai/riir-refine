# Rule rebuild description-fidelity audit — artifacts (issue 242)

This directory carries the PREP half of issue 242's T1 (the iWSP text-bottleneck
metric on the rule corpus): the frozen sample's composed descriptions and the
protocol of record for the seat-gated run. The controlling record is
[`.issues/242_rule_rebuild_description_fidelity_audit.md`](../../.issues/242_rule_rebuild_description_fidelity_audit.md);
the metric's provenance is `katgpt-rs/.research/678_iWSP_Interpretable_Weight_Space_Programming.md`
(arXiv:2611.18114).

## Status

**PREP COMPLETE (2026-11-22, zed-m3-i242-t1prep) — RUN BLOCKED-OWNER.** The hosted
writer seat has no reachable arm on any box (no `RIIR_HOSTED_EXPERT_TOKEN`, no R1
endpoint row, no egress consent — verified 2026-11-21 by amo-idle-i242-preflight).
Arming is a credential touch (owner gate per the 2026-11-11 gating decree). The
RUN itself executes only after the owner arms a seat on some box; this directory
is everything the run consumes that can be prepared without it.

## Files

- `descriptions_v1.json` — the 30 composed descriptions (10/10/10 clippy_lints /
  rust_perf / kernel_opt), one row per frozen rule. Canonical digest
  `de09306af862b4e400765b0c855c22c66ea397faeccc7c1c3d7ff8821495cc20` (sha256 over
  the sorted-key JSON of the row list), composed at HEAD `972a59596f9b`.
- This README — the procedure + scoring spec the run lane follows.

## The sample (v1 freeze stands)

The sample was FROZEN by amo-idle-i242-preflight (2026-11-21) BEFORE any hosted
call: 30 rules, 10/10/10 stratified, deterministic
`sorted(ids, key=sha256("242:<domain>:<rule_id>"))[:10]` at develop `bb47308e`,
sample-digest `a5ad3f9fba066fa15e5867779a1a0c09e154726ab922afca22d679212a5506f4`.
This lane re-derived the selection at HEAD `972a59596f9b` after the B398–B400
corpus delta (unique id sets: clippy 120 unchanged, rust_perf 380→389,
kernel_opt 1098→1112): **the same 30 ids win in all three domains — the v1
freeze is selection-stable, no re-freeze; the recorded digest remains the
anchor.** The freeze is the audit anchor, not a barrier: any future corpus
change re-freezes by the same procedure at the new commit, in the same commit
as the change.

## Composition law (what the descriptions are)

Each description is ≤200 words of PROSE covering (a) TRIGGER — the code shape
the rule fires on, self-sufficient for a fresh engineer; (b) FIX SHAPE — what
the remedy transforms the trigger into, with the invariant preserved; (c) the
entry's REGIME / ANTI-REGIME decision content. Composed from the corpus entries
at the composed-at HEAD. No code blocks; inline identifiers allowed; no rule id
inside the description text; no verbatim entry sentences longer than ~8 words.

Verification record (this lane): mechanical — word counts recounted (all
123–199), anchors exist and carry the rule id near the anchor, id-leak check,
distinctive-token presence in the entry files; semantic — every measured number
in every description grepped at its entry (eager-pointer
1903.70x/5414.62x/21353.90x, bufread 3.63x/3.00x/2.77x + 10892 allocs, byte-flag
2.62x/2.94x/2.10x, work-removed 15x→1.33x, smallvec ≤8 + `[T; 8]`, retain bench
1K..1M, regex bench 10/100/1K, all clippy pairs read at their anchors; kernel
anchors read at entries_gpu_07/10/15/52/63/104/113/145/146/157).

**Freeze discipline:** the descriptions are frozen PRE-RUN, before any rebuild
exists. A rebuild result must never feed back into a description; if a
description must change, the change lands as `descriptions_v2.json` with a NEW
digest and the run re-opens — no in-place edits after the first run consumes
v1.

## The run protocol (for the seat-gated lane)

For each of the 30 rules, TWO arms, paired:

1. **Description arm:** a FRESH session (no corpus access, no entry text) is
   handed ONLY the rule's description from `descriptions_v1.json` and asked to
   re-draft the fix: "Given this description of a code-transformation rule,
   write the fix for the trigger it describes" — the domain's fixture buggy-arm
   is NOT shown as code to imitate (the trigger is restated in the description;
   the fixture supplies the concrete instance at scoring time only). The
   re-draft is scored against the rule's authored fix on the entry's
   oracle-labeled fixture pair.
2. **Control arm (the paper's description-less control):** the same re-draft
   task with NO description — the task prompt alone (domain + "write a fix that
   matches the authored fix for this trigger", with the trigger stated as the
   fixture's buggy arm). The paired Δ (description − control) is the metric.

- **Fix-match** (primary): the re-draft, applied to the fixture's buggy arm,
  compiles at the fixture's own posture AND is semantically equivalent to the
  authored fixed arm (same transformation; byte-match is the tie-break, not the
  bar — the authored spelling is one witness, not the only correct answer).
  Per-rule verdict, then per-domain rate.
- **Hosted budget:** ≤ $10 list-price cap for the whole run (Research 671's
  dspy-wsp writer-spend datum); refuse to start without the cap recorded.
- **Kill bar (pre-registered in the issue):** Δ < 10 points of fix-match over
  the control ⇒ the audit CLOSES NEGATIVE — descriptions carry no re-derivable
  signal beyond task prompts; the instrument dies with the negative recorded.
- **Pass bar:** Δ ≥ 10 points with per-rule variance that identifies specific
  low-fidelity rule families — the only promotion-worthy outcome is a
  rule-quality defect the existing gates missed.
- **Report-only:** the run lands as a `.benchmarks/NNN_*` record; NO promotion
  path, NO corpus mutation, NO gate wiring until the defect-showing bar is met
  in a NEW pre-registration.
- **Code-egress consent:** rule descriptions leave the box (the seat consumes
  them); rule code need not and must not (the `RIIR_REFINE_CODE_EGRESS=snippet`
  scope applies to any snippet-bearing call).

## What remains (all seat-gated)

- The seat arm: token + R1 endpoint row + egress consent on some box (owner act).
- The `.benchmarks/` record of the paired Δ.

## The scorer lane (BUILT 2026-11-25, amo-i242-scorer) — the instrument exists; the run stays seat-gated

The 2026-11-17 routing note ("a lane without its seat measures nothing") was
UN-PARKED by a recorded Claude verdict (3 rounds, 2026-11-25: AGREE-with-
conditions — protocol-conformant fix-match with a MANDATORY compile leg, the
alpha-normalized PRIMARY semantic column replacing the verdict lane's first
inversion, the authored tests as the behavioral witness, the std-macro-family
renaming, the verdict ranking pinned, the fence refusal, and the two
disclosure conditions below). The lane is `rule_rebuild_audit` (opt-in; NOT
default, NOT in the release set):

- `cargo refine --rule-rebuild-audit [--rule-rebuild-cap <usd>]
  [--rule-rebuild-writer replay|hosted]` — report-only, exit 0 on a read,
  exit 2 on an arming/posture refusal (cap / seat / consent, each naming its
  piece). `replay` is the no-seat self-test (fake oracle, no spend).
- Scoring: `strict` / `alpha` / `compiled` columns never pooled; the verdict
  = MATCH (compiled AND authored-tests pass (or vacuous) AND (alpha OR
  strict)) / UNDECIDED-shape / UNDECIDED-signature / NO MATCH; the writer's
  own tests are a disclosed-only column.

**The pre-run census (verdict round-3 condition 1 — recorded BEFORE any
hosted call):** the replay self-test at the landing commit classifies ALL 30
frozen rules **scoreable 30/30, UNSCOREABLE none** — every authored fixed arm
parses as Rust (the clippy entries are fn-shaped with the doc comments
attached to items). Both bars (kill Δ < 10 pts, pass Δ ≥ 10 pts) therefore
compute over the FULL n=30; the low-power warning (n < 20) does not apply.
The census re-derives at every load — a corpus change that makes an authored
arm unparseable re-opens this note, never silently shrinks the sample.

Vacuous disclosure (verdict round-3 condition 2): 6 of the 30 authored arms
carry a `#[test]` surface (rust_perf: watchdog-deadline, batch-reserve,
byte-flag; kernel_opt: walk-terminating-fill, dtype-branch, moe-group) — the
other 24 score MATCH on the alpha/strict columns alone, printed per arm as
`tests=vac` and folded into the report's vacuous line, never into a real
pass.
