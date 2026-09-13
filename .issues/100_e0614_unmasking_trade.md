# Issue 100 — the unmasking trade: E0614 deref-at-use playbook + the lexicographic keep measure

**Status:** OPEN — in flight (T1 measured; T2–T6 implementing). The named follow-up lead from Issue 099 (Bench 089 §Honest limits: "an E0614 deref-removal-at-use playbook is a healer-corpus lead, deliberately out of that issue's scope").

## The measured premise (why a playbook ALONE is inert)

Bench 089's mechanisms table: F1/F3 + clone-out → `E0614@*head` ONLY — the E0499 is
masked (typeck blocks borrowck) → stall. The named lead assumed an E0614 playbook
"would unstrand F1/F3's clone-out paths" — **measured FALSE as stated**: the driver's
keep rule is `new_diags.len() < before_errors` (`driver.rs`, the strict count
decrease), and the star-removal fix trades 1 error for 1 unmasked error (1→1) — a
revert, not a keep. Probed on rustc 1.98.1 (2026-09-13, transcript below):

| state | diagnostics | consequence under the CURRENT rule |
|---|---|---|
| `let head = v[0]; … *head` (organic stale star, alias pair broken) | exactly 1 × E0614, primary span covers `*head`, label "can't be dereferenced", no suggestions | star-removal → E0499 unmasked (1→1) → REVERT → stall |
| F1+clone-out (`let head = v[0].clone(); … *head`) | exactly 1 × E0614 (E0499 masked) | same 1→1 revert |
| b_only (star fixed, pair broken) | exactly 1 × E0499 | swap fixes it — the unmasked target state is fixable |
| multi-use `*head + *head` | one E0614 diag PER site | re-proposal handles the rest; single-edit candidates suffice |

So the playbook is necessary but NOT sufficient. The load-bearing half is the keep
rule: a keep that trades a masking-stage error for a fixable borrowck error is real
progress the count cannot see — and, symmetrically, a count-decreasing keep that
CREATES a masking error (clone-out on Copy strata, the T4 counter poison) is a
regression the count cannot see either.

## The design

**The lexicographic keep measure**: keep iff `(masking_count, total_count)` strictly
decreases lexicographically, where `masking_count` = diagnostics whose code is in
`MASKING_STAGE_CODES` (codes emitted at a compiler stage that BLOCKS all later
stages; fixing one can reveal the blocked stages' diagnostics). Seeded
`MASKING_STAGE_CODES = ["E0614"]` — every member must carry a measured masking
justification (the probe table above; Bench 089's mechanisms table).

- Well-founded: ℕ×ℕ under lex order — the Bench 087 Lyapunov argument survives
  (kept ⇒ strict lex-decrease; no infinite keep sequence; the tried-set bound is
  unchanged).
- **F1 clone-out (0m,2)→(1m,1)**: masking UP → honest REVERT (was: the strand-keep
  that poisoned the T4 counters). The T4 regression (learned arms 10/34) repairs —
  the orderings fall back to deref-removal on the Copy strata.
- **F3 clone-out (0m,2)→(1m,1)**: same honest revert — and the run now ends with the
  file UNMODIFIED instead of carrying a semantically-changed clone-out + E0614
  (strictly better for the user; it also blocks the measured 3-edit dead end:
  clone-out → star-removal → swap → unfixable E0502@clone-line stall).
- **F4 star-removal (1m,1)→(0m,1)**: masking DOWN → KEEP — the unmasking trade the
  rule exists for.
- Existing 246-case corpus: no case declares E0614 → masking is always 0 → the rule
  degenerates to EXACTLY the current one (zero behavior change; floor re-pin only
  moves by the new F4 rows).

**The F4 corpus family** (12 cases, shared-segment composition like F1–F3): member
A = the stale deref star at the use (organic E0614: the binding is by-value, the
use still dereferences); member B = the alias pair (E0499). broken = E0614 only
(E0499 masked); a_only = E0614; b_only = E0499; the 2×2 admissibility (set tier:
pair multiset ≠ sum of singles) holds by the same masking mechanism as F1.
MutatorKind::AddDerefStar (`head` → `*head` at the use — the honest reporting label).

**The playbook** (`e0614-deref-at-use`, `playbooks/e0614.rs`): anchor = the primary
labeled span in the primary file; fire only when its source text is EXACTLY `*ident`
(ascii ident chars after the star — `**x`, `*x.f`, `*f(x)` stay misses); edit =
delete the star byte. No semantic note (reading the binding by name where the star
was a type error; no ownership change).

**Instrumentation**: `AttemptRecord.masking: (usize, usize)` (`#[serde(default)]`,
old rings deserialize as (0,0) — consistent: the old rule's keeps all had count
decrease, so a (0,0) masking read cannot misclassify a legal old record). The
`fixseq_prevalence` invariant checker goes lex-aware with both-side canaries; the
per-run ring print carries the masking delta when nonzero.

## Non-goals

- No new ordering policy, no corpus-family work beyond F4, no E0502 head-reorder
  template (F3's residual unfixable state — the next lead if it ever matters).
- `MASKING_STAGE_CODES` grows only by measured members, never by taxonomy guesses.

## Tasks

- [x] T1 — the E0614 diag-shape probes on rustc 1.98.1 (table above; scripts under
      `.heal/`, gitignored scratch).
- [ ] T2 — F4 family in `corpus_interacting.rs` (12 cases) + `MutatorKind::AddDerefStar`
      + the `ix_f4_` spot row; cheap structural tests stay green.
- [ ] T3 — `E0614DerefAtUse` playbook + unit tests + `Playbooks::production()`
      registration.
- [ ] T4 — the lexicographic keep measure: `MASKING_STAGE_CODES`/`is_masking_code`
      in `diag.rs`; driver lex keep rule; `AttemptRecord.masking`; lex-aware
      invariant checker + canaries in `fixseq_prevalence.rs`.
- [ ] T5 — measurements: admissibility gate (expect 46/46), T3 flip-prevalence lane
      (expect reverts 12→24: F3's clone-out joins its deref-removal as an honest
      revert; F4 adds 2-keep clean runs), T4 ordering A/B (expect every arm 34/46 —
      the regression repaired, parity honest: still no learned WIN), floor re-pin
      (N 246→258), full-corpus integrity, clippy on touched files.
- [ ] T6 — Bench 090 + HISTORY record; issue closed + removed per the
      noise-reduction rule.

## Decision criteria (pre-written)

- Gate must ADMIT all F4 cases (else the family is dropped, loudly — rustc drift).
- The T4 A/B verdict line must print WORSE/IMPROVEMENT/PARITY against BOTH axes
  (clean, reverts) — parity is the expected honest outcome for the learned arms.
- The loop-invariant report must still print HOLDS over every attempt of every lane
  — under the LEX reading, with the old rings' (0,0) default verified consistent.
