# Research 263 — Batch 232: the TensorFold 1.0.9 leads conversion (the deferred-background-pass yield law + the HEADROOM-ADMISSION rider)

**Status:** DISTILLED — landed (Batch 232 / Plan 234); see the snapshot landing row for the commit.

## Source

ashhart/TensorFold (Apache-2.0):

- **(1a)** `c724dd9d` — "Nemotron CUDA: a reply's background pass waits for the next
  request, then runs short chunks". Quote basis: the commit message + the full .patch
  (`/tmp/tf_1a.patch`, ephemeral), read in full this session. The intake row
  (cycle 84, ~05:2x +07) graded it D-class B+; confirmed at the slot.
- **(1b)** `b264f534` — "The 27B's Neural Engine share loads only inside the 70% load
  limit". Quote basis: the commit message + the full .patch (`/tmp/tf_1b.patch`) —
  `zig/src/native/cache_fit.zig` `fitsLoadLimit` + its test rows.
- **(1c)** the v1.0.9 release `72b45368` changelog — read in full (`/tmp/tf_rel.json`);
  league/lane intel only, NO rule (see the verdict below).

## The conversion

- **(1a) → OWN kernel_opt D-class rule** `warm-background-pass-quiet-hold-yield-chunking-cut-invariance`
  (new `entries_gpu_91.rs`; corpus 804→805; D 188→189). The law's three faces:
  1. **HOLD-BEFORE-GPU + DROP-DON'T-QUEUE** — the next-turn background prefill waits
     `warm_quiet` (the Nemotron CUDA host: 150 ms; other hosts 0) of engine quiet; a
     request, a decoding stream, a cancel or a close during the wait DROPS the pass
     before it touches the GPU — never queues it behind the work it would delay.
  2. **YIELD-CHUNKING + STEP-ASIDE STATE-KEEPING** — once running: `yield_rows = 256`-row
     chunks, one in flight, so an arriving request waits one chunk at most; on step-aside
     the pass KEEPS the prefill state it reached (rows + the history mark) for the next turn.
  3. **CUT-INVARIANCE (the legality contract)** — "A row's bits do not depend on where a
     pass is cut, so replies and kept states are unchanged"; the exception classes:
     lessons/dumps/decision-row passes keep full 2,048-row chunks; the warm mode splits
     `.on` (drops a history mark near the resume point — fine when the pass always runs)
     vs `.waits` (keeps the mark — required when the pass may never run).
  Dedup at the slot: B182 `verify-window-prework-loads-before-stores` = INTRA-kernel
  ordering (unowned cross-request surface confirmed); B215
  `zero-anchored-prompt-grid-chunk-identity` (same repo) = the cut-placement law — this
  rule composes it (bit-level contract vs the scheduling that uses it);
  `multiwave-dual-decode-admission-gate` = decode-wave admission inside one launch
  (different conjunct class); `metal-keepalive-residency-unwire` = idle work that must
  run vs idle work that must yield.
- **(1b) → count-neutral rust_perf rider** on `degrade-not-fail-load-qualification-probe`
  (entries_21) — the HEADROOM-ADMISSION face: a load-time admission with the same
  two-class posture (admit / degrade loud and keep serving), predicate axis widened
  from measured exactness to resource headroom. `fitsLoadLimit`:
  `bytes +| MARGIN <= total / 100 * SHARE_PERCENT` (the saturating add; fail-closed on
  an unknown RAM reading). The defect economics (#643): 5,760 MiB of ANE share swapping
  for ~5% faster prompts = a net loss; the served host answers with the 70% load limit
  + 2 GiB margin ("a 32 GB Mac runs prompts on the GPU alone and says so"; 48 GB+ keeps
  the share); direct-loader tools keep it wherever it can load (the carve-out: the limit
  is a served-host contract). Thermal cross-ref rides in-rule (the runtime pacing axis
  sibling); B139 prefault-admission distinguished (page-fault eagerness, not copy admission).
- **(1c) → NO rule.** The release arms decode as: Cortheon mode (out of lane); Nemotron
  CUDA concurrent-round batching (#768-771, the shared-round MTP draft + one-launch
  Mamba/attention fusion — league intel; the bit-equality contracts restate landed
  concurrent-round parity laws); Nemotron CUDA long-prompt 4,096-row passes (#720 —
  the same cut-invariance the (1a) rule carries, applied to the pass size); **the
  "faster 27B on Macs" arm** = the per-token DeltaNet state copy eliminated (288 MiB
  r/w per token, #711) + the chain round attending its own keys in the prefix kernel
  (4 dispatches/layer instead of 7, #712) + DFlash2 drafter start sooner (#713) — the
  state-copy elimination is the ANTI-pattern face of the corpus's state-reuse laws
  (our deltanet lanes are state-reuse-by-construction); the dispatch fold is the
  owned dispatch-fold family; Apple GPU gen-18 (M6) bring-up = league hardware intel.
  No mechanism survives dedup as a new rule.

## Gates (all green)

- G1: `batch232_rules_parse_and_needles_landed` (20 verbatim needles + both pseudocode
  halves parse) + `batch232_rider_landed_on_degrade_host` (9 needles) — 3/3 passed.
- G2: kernel_opt lib 3097/0/1i (batch-231 base 3095 + 2); default lib 2651/0 (+1 rider
  test); rust_perf posture 2651/0.
- G3: clippy -D warnings at default + all-features, 0 findings (2 doc_markdown heals in
  the new file via `cargo refine --fix --write --verify` — compile-gated; 2 more fixed
  manually in the wiring docs, same mechanical class).
- Retrieval floors HELD: 21/21 (kernel self-variants + blended) — no re-pin.
- Freeze/score-bench NOT owed (zero clippy-corpus delta, P30).

## Count pins moved (the ×7 sites)

mod.rs exhaustive-doc + D-comment (188→189) + D-list; router.rs expert-total; tests.rs
batch116 pin, seed_corpus pin, plan-003 per-category table + sum, grouped round-trip,
batch-231 historical re-pin (the batch-227 re-pin chain pattern).

## Honesty notes

- All release-body figures (2.0-2.2× resume, 1.17-1.26× at 9k-16k histories, M5/M3 Ultra
  decode cells) are SOURCE-REPORTED provenance, never adoption bars; the (1a) rule is
  NotBenchable here (no CUDA MoE serving validator in this corpus).
- The intake's needle 9 phrasing ("other work came in first") was corrected at the patch
  to the character-exact "// other work came first: it never ran" — the intake-drift class
  the batch-231 landing also caught (P53's third-mechanism capitalization).
- The rider header needed the full 40-char SHA (the G1 needle caught the short form the
  authoring subagent wrote — the gate doing its job).
- (1c)'s league figures are opponent movement for the next perf-rematch bout window, not ours.
