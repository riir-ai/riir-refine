# Bench 090 — Issue 100: the unmasking trade — E0614 playbook + lexicographic keep rule — F4 12/12, the ordering regression REPAIRED (10→34), the first legal equal-count keep

**Status:** RECORD — the follow-up lead from Bench 089 ("an E0614 deref-at-use playbook
would unstrand F1/F3's clone-out paths") landed as the measured PAIR it had to be:
the playbook alone was inert (the count-only keep rule reverts the star-removal at
1→1), so the load-bearing half is the **lexicographic keep measure** `(masking,
count)`. The F4 family resolves 12/12 through the first legal equal-count keep in
the driver's history; the T4 learned-arms regression (10/34 → now 34/46) is
repaired; F3's clone-out strand-keep is now an honest revert with the file
restored byte-exact; and the corpus gained its first runs with flip POWER
(3-attempt depth).

**Date:** 2026-09-13 · **Box:** 4090 RTX box "shikuwa" (Windows 11, i7-13700K), cargo quiet (no sibling compute) · **Toolchain:** rustc 1.98.1 (48a229cea 2026-09-01) · **Profile:** `--release`.

> **Issue:** `100` · **Predecessor:** [Bench 089](089_interacting_error_corpus.md) (the E0614 stall + the T4 regression this bench repairs) · **Gate instrument:** `examples/interacting_gate.rs`.

## The premise correction (measured, not derived)

Bench 089's lead assumed the playbook alone would unstrand. The driver's keep rule
was `new_diags.len() < before_errors` — the star-removal trades 1 error (E0614) for
1 unmasked error (E0499), and `1 < 1` is false → revert → still stalled. Probed on
rustc 1.98.1 before any code:

| state | diagnostics | old-rule fate |
|---|---|---|
| organic stale star (`let head = v[0]; … *head`, pair broken) | exactly 1 × E0614, primary span covers `*head`, label "can't be dereferenced", no suggestions | star-removal 1→1 → REVERT → stall |
| F1+clone-out | exactly 1 × E0614 (E0499 masked) | same |
| b_only (star fixed, pair broken) | exactly 1 × E0499 | swap fixes — the unmasked target is fixable |
| multi-use `*head + *head` | one E0614 PER site | re-proposal suffices; single-edit candidates |

## What landed

- **`MASKING_STAGE_CODES = ["E0614"]`** (`diag.rs`) — codes emitted at a compiler
  stage that BLOCKS every later stage; membership is probe-measured, each member
  cites its transcript. The keep rule's primary term.
- **The lexicographic keep rule** (`driver.rs`): keep iff `(masking, count)`
  strictly decreases lex. The unmasking keep (equal count, masking down) is legal;
  the strand-keep (count down, masking UP) reverts. Well-founded on ℕ×ℕ lex — the
  Bench 087 termination argument survives; `AttemptRecord.masking` extends the
  fixseq ring (`#[serde(default)]`: pre-100 rings read (0,0), consistent because
  every old keep had a strict count decrease at masking 0).
- **`e0614-deref-at-use`** (`playbooks/e0614.rs`): deletes the stale star when the
  primary span's source text is exactly `*ident` — `**x`, `*x.f`, `*f(x)` stay
  misses (multi-token operand = a real edit decision this template refuses to
  guess). Registered in `production()`; the evolve-ledger ratchet red until the
  `FRONTIER_LANDINGS` row landed in the same commit (the gate working as designed).
- **F4 `ix_f4_e0614_e0499_star_*` ×12** (`corpus_interacting.rs`): member A = the
  stale star on a by-value use; member B = the alias pair. Shared-segment
  composition like F1–F3; `MutatorKind::AddDerefStar`; `ix_f4_` spot row.
- **`fixseq_prevalence`** lex-aware invariant checker + 3 new canaries: the
  unmasking keep LEGAL, the strand revert LEGAL (the old checker would have
  flagged it), a revert-despite-lex-decrease REPORTED.

## Lanes run

| lane | command | result |
|---|---|---|
| cheap structural tests | `cargo test --features rustc_errors --lib` | **1265/1265** |
| admissibility gate | `cargo run --release --example interacting_gate --features rustc_errors` | **46/46 ADMITTED** (19.0s) — F4 interacts by the measured masking mechanism |
| T3 flip prevalence | `fixseq_prevalence -- --interacting --runs` | below |
| T4 ordering A/B | `rustc_errors_floor --features rustc_errors,latent_retrieval,nonergodic_driver -- --only ab --interacting` | below |
| floor + driver lanes | `rustc_errors_floor -- --only floor,driver` | G-floor 41.5% (258); driver **62.0%** (148→160); **G1 160/160**; G2 p50 129ms/case |
| full-corpus integrity | `RIIR_FULL_CORPUS_INTEGRITY=1 cargo test --test generated_corpus_spot full_corpus` | **PASS over 258** (164.6s) |
| nonergodic A/B tests | `cargo test --features rustc_errors,latent_retrieval,nonergodic_driver --test nonergodic_driver_ab` | **6/6** |
| clippy (touched files) | `cargo clippy --features rustc_errors --all-targets` | 0 findings in touched files (healer run; one manual unused-`mut`; push_attempt carries the repo-precedent `#[allow(too_many_arguments)]`) |

## T3 — flip prevalence over the extended corpus (n=46)

```
runs 46 · attempts 104 · depth hist {2: 34, 3: 12} · runs ≥3 attempts: 12  <- flip POWER now exists
flips: code 0 · span 0 · pair 0 (power nonzero for the first time — the zero is now a sample)
loop invariant: HOLDS over all 104 attempts (the LEX reading)
repair quality: 73.9% total — E0502 34: 12/10/12 (unchanged); E0614 12: 12/0/0
```

**F4 rings** (the designed shape, every one identical):
`E0614:e0614-deref-at-use=KEEP(1->1),m1->0  E0499:e0499-swap-second-decl=KEEP(1->0)`
— the first legal equal-count keep: the star-removal unmasks the E0499 the typeck
error was blocking, and the swap finishes.

**F3 rings** (the honest-repair shape):
`deref-removal=REV(2->2)  clone-out=REV(2->1),m0->1  swap=REV(2->2)` — the
clone-out that USED to strand-keep (leaving a semantically-changed binding + an
unfixable E0614) now reverts; the run continues to the swap (which also reverts —
B's swap alone cannot fix A's E0502), and the file is left **byte-identical to the
broken original**. Three attempts deep — the depth a flip needs, so the flip
instrument's power statement changed from "0 runs can flip" to "12 runs can".

## T4 — the ordering A/B over the extended corpus (n=46)

| arm | clean | iterations | reverts | attempts | multi-attempt runs |
|---|---:|---:|---:|---:|---:|
| static (min-delta) | **34/46** | 104 | 36 | 104 | 46 |
| flat counters | **34/46** | 104 | 36 | 104 | 46 |
| shape-conditional | **34/46** | 104 | 36 | 104 | 46 |
| nonergodic shared | **34/46** | 104 | 36 | 104 | 46 |
| nonergodic per-run | **34/46** | 104 | 36 | 104 | 46 |

`discord` 0/0 everywhere · `HEADROOM PREDICATE = 184 ≠ 0` · verdict: **PARITY —
the Bench-089 regression (learned arms 10/34 while static held 22/34) is
REPAIRED**; no learned arm WINS on this corpus either (the honest negative stands:
static stays the shipped ordering).

Mechanism of the repair: clone-out's per-attempt counters now carry the F1/F3
reverts (`10k/24r` vs deref-removal's `12k/12r`+F4's `12k/0r`), so no
counter-driven ordering prefers it on the Copy strata — the poisoned
keep-that-strands no longer scores as success.

## Floor movement (N 246 → 258)

- driver lane: **62.0%** (was 60.2% — numerator 148→160: the 12 F4 heals); E0614
  row 12/12/0; E0502 row unchanged 33/26/12 (static-path outcomes untouched by
  the lex rule — predicted and confirmed).
- G-floor lane: 41.5% (was 43.5%) — the NUMERATOR is unchanged (107); the harness
  fixer cannot fix E0614 (it is the driver-only shape), so the percentage diluted.
- **G1 semantics 160/160** (was 148/148 + the 12 F4 heals); G2 p50 129ms/case
  (was 128 — stable).

## Honest limits

- All numbers are SYNTHETIC-corpus numbers (the harness's standing label); the
  organic `.heal/fixseq/` surface still has zero rings across 33 repos.
- `MASKING_STAGE_CODES` is exactly one measured member. Every future member needs
  its own masking probe — a taxonomy guess here would silently re-rank every keep.
- F3 remains unfixed (12/12): the residual state after the honest reverts is the
  E0502-on-the-indexed-clone-line shape, whose repair is a head-read REORDER no
  playbook ships (named as the next lead only if it ever matters organically).
- The equal-count keep is legal ONLY with a masking drop — a same-stage trade
  (borrowck→borrowck at equal count) still reverts, by design.
