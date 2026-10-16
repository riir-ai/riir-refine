# Research 279 — Batch 247: the splash runtime-oracle rider pair

**Status:** DISTILLED — LANDED (Batch 247 / Plan 251; count-neutral riders on
`entries_gpu_95`, corpus stands 848)

## Source

incoai/splash (Apache-2.0) — the vendor clone `E:/git/vendor/splash`, fetched
to `be3b083c5` this session (2026-10-16). The splash leg fired on the envelope
pass: `ffc2d4be` → `be3b083c5` (+7). Delta grades: #428 = the convertible
oracle gate (`4919202`); #416 = the weights-release endpoint honesty commit
(`48a0e7b`, WATCH C+ — 200-only-when-released-AND-not-restoring, 409
engine_busy, the shared release helper; ordinary API-state hygiene, no corpus
host); #419 = docs corrections (NULL); #418 = the P60 memory governor already
consumed by Batch 237. Beside the delta, the standing Research-268 deferral
"3903390's content-vs-count oracle" converted as the second rider — both
commit bodies read in full at the pins, every landed quote byte-verified via
`git show` greps this session.

## The pair — the oracle's two failure directions on one host

Host rule: `mid-prefill-recurrent-state-capture-boundary-unsplit`
(`entries_gpu_95`, B236) — the mechanism's riders extend its VERIFICATION
half; the mechanism half is untouched.

### (a) UNDER-COMPARED — counts without content (`3903390`)

The runtime oracle's budget-vs-mask commit check "compared only the two lanes'
retained counts and draft rings: the retained rows were compared nowhere, and
only the continued block checked one count end to end, the one the model's
acceptance gave". Since "a decode step leaves the rows it retains on its
lane's tape for the next step to fold in, the GDN cell a step writes holds the
state before its rows", equal counts with unequal rows = the next step folds
DIFFERENT state, invisibly.

The fix compares "for every retained count the loop forces, the two lanes' GDN
cells and the rows each retained, byte for byte, so that the next step of
either lane would fold the same state", reading a completed lane's rows where
they wait via `model::Runtime::pendingGdnRows` ("returns a resident request's
pending rows of every GDN layer's tape, field by field"; the diff:
`require(!retained.empty() && retained == executor.pendingGdnRows(masked.requestId))`).

Seeded-defect witness: "A scratch build that leaves one retained row of a
constrained lane one bf16 ulp off fails the check at its first count ('fixed
DFlash-8 retained=2 retained GDN rows differ between budget and mask
commits'), where main's oracle compares no rows."

**Law:** count-equality is NOT state-equality — an oracle over committed state
must compare the CONTENT the next step consumes, never only the counts.

### (b) OVER-COMPARED — content across unlike partitions without a bit-alike gate (`4919202`, PR #428)

The captured-state check "compares a restore from a state captured inside a
command with a restore from commands that ended at the same boundaries, and
held them to 0.999 whenever their FFNs ran alike" — which "assumed the GPU
computes a row the same whatever the command's row count". False:
"GGUF float projections (#403 loads BF16 and F16 tensors as such) pick the
neural accelerator tile or the simdgroup one by row count; both are fp32 up to
accumulation order" — so "on the 40-core M5 Max the 1632-row capturing command
and the 544- and 1056-row ended ones pick differently, so ISTA-DASLab and
Swift-1.5 Qwen3.8-27B GSQ-RCO IQ3_S failed test-real at --ane-ffn-share 0
(continued_conv_cos 0.963)" — an ORACLE artifact, never a kernel defect
("Any arithmetic that differs leaves the continued convolution state about
that far apart, as the mixed Neural Engine runs the check already skips show
(0.94-0.99)").

The fix: "The check now runs where the ended commands' KV of the rows both
computed equals the capturing command's bit for bit, which covers a split that
ran in one and not the other as well, and prints kv_alike instead of
ffns_alike" (diff: `ffns_alike=` → `kv_alike=`); passes both ways on the
Studio: ISTA IQ3_S at kv_alike=0, Qwen3.6-35B-A3B-4bit at kv_alike=1 with
every cosine 1.

**Law:** an oracle comparing across differently-partitioned commands must gate
on the partitions agreeing BIT-FOR-BIT on the shared region — row-count-keyed
kernel selection breaks the same-hardware-same-arithmetic assumption, so the
like-arithmetic precondition is itself a CHECKED PREDICATE, never an
assumption.

### Together

Compare content, gated on bit-alikeness. The two directions compose into one
oracle-design contract: under-comparison hides real defects at one bf16 ulp;
over-comparison fires false ones at 0.963 cosine.

## Dedup (greps run before authoring)

- `kv_alike` / `partition.{0,30}alike` / row-count-keyed tile selection → zero
  corpus hits.
- `3903390` was pre-adjudicated rider-grade on this host by Research 268
  ("content-vs-count oracle a future rider") — discharged here.
- Kinship in-rule: `reshape-relabel-axis-scramble-content-oracle` (B233,
  entries_gpu_92) owns content-vs-shape for LAYOUT correctness;
  `bit-equality-oracle-keyed-on-reference-dispatch` +
  `bit-equal-fusion-mirrors-reference-reduction-regime` (B242, entries_gpu_99)
  own oracle KEYING on the reference's dispatch/reduction regime; THIS pair
  owns the boundary-state oracle's own SOUNDNESS (WHAT to compare, WHERE to
  compare) — the third member of the oracle-law family, on the rule whose
  mechanism it verifies.

## Gates (all green, this session)

- `batch247_oracle_riders_landed`: needles (17 across both riders) + both
  pseudocode halves parse + corpus count asserted 848 — FIRST RUN GREEN.
- kernel_opt lib: 3134 passed / 0 failed / 1 ignored (B246's 3133 + this test).
- default lib: 2678/2678 standing.
- clippy `-D warnings`: 0 errors at default AND kernel_opt+latent_retrieval
  postures.
- Retrieval floors: 21/21 HELD at 312.88 s — blended 90.6/94.2/95.3 vs floors
  89.4/93.6/93.6, byte-identical to the Batch-246 read (text-only append,
  zero movement, NO re-pin).
- Freeze/score-bench NOT owed (P30 — zero clippy-corpus delta).
- Maintained binary NOT rebuilt (kernel_opt is not in the release set; the
  served corpus is unchanged).

## Envelope context (the same pass)

llama.cpp master `2f729cfa` → `59d5fad7c` = 2 NULL Granite convert commits
(#30621 MultiSwitch block_count + #30628 speech tensor-mapping regression —
conversion-script surface, zero kernel/perf law); NEW ENVELOPE PIN
`59d5fad7c`. prism `5ae2e169e` = pin; prism-b ladder top 10777 = pin (numeric
read); cubecl max `v0.11.0-pre.4` NO-FIRE; memchr top `2.8.3` NO-FIRE;
tensorfold `c7137bb31` UNCHANGED (the 72-commit learned-disk-admission drift
pool = the standing future lead, not grown); mlxfast floor ~10-17 NOT DUE.

## Deferred / watch

- `48a0e7b` (#416 weights-release endpoint honesty) — WATCH C+; re-read if a
  future delta lands more serving-endpoint state-machine content.
- The splash SSD-cache series stays watch-grade (Research 268's standing
  disposition; no new commits in this delta).
- TensorFold's learned-disk-admission drift pool (72 commits past the B219
  pin) — the standing future-lead pool for a third TensorFold batch.
