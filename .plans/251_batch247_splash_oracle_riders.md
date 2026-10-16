# Plan 251 — Batch 247: the splash runtime-oracle rider pair

**Status:** COMPLETE (landed 2026-10-16; the claim row above discharged by the landing row in `.distill/001`)

Source: incoai/splash (Apache-2.0) @ the vendor clone `E:/git/vendor/splash` fetched
`be3b083c5` this session — TWO count-neutral riders on
`entries_gpu_95` (`mid-prefill-recurrent-state-capture-boundary-unsplit`, the B236
capture-boundary rule, whose own oracle face this extends):

1. **`4919202`** (PR #428, the NEW delta content past P60's pin) — the KV-ALIKE
   oracle gate: the captured-state check held restores to 0.999 "whenever their
   FFNs ran alike", which "assumed the GPU computes a row the same whatever the
   command's row count" — but GGUF float projections "pick the neural accelerator
   tile or the simdgroup one by row count; both are fp32 up to accumulation
   order", so differently-row-counted commands picked different tiles and the
   oracle flagged false defects (continued_conv_cos 0.963). The fix gates the
   comparison on bit-for-bit KV equality of the rows both computed
   ("kv_alike" replaces "ffns_alike"). LAW: an oracle comparing across
   differently-partitioned commands must gate on bit-for-bit agreement on the
   shared region — the like-arithmetic precondition is a checked predicate,
   never an assumption.
2. **`3903390`** (the Research-268 "content-vs-count oracle" deferral, now
   beside its twin) — the budget-vs-mask commit check "compared only the two
   lanes' retained counts and draft rings: the retained rows were compared
   nowhere". The fix compares, for every retained count the loop forces, the
   two lanes' GDN cells AND the retained rows, byte for byte ("so that the next
   step of either lane would fold the same state"), reading completed lanes'
   rows where they wait via `model::Runtime::pendingGdnRows`. Witness: one bf16
   ulp off in one retained row fails the check at its first count "where main's
   oracle compares no rows". LAW: count-equality is NOT state-equality — compare
   the CONTENT the next step consumes.

The pair = the oracle's two failure directions on one host: UNDER-compared
(counts without content) + OVER-compared (content across unlike partitions
without a bit-alike gate).

No overlap with the M3's live Batch 245 (regex/memchr/bstr rust_perf files).
Corpus STANDS 848. Freeze/score-bench NOT owed (P30, zero clippy-corpus delta).
Binary rebuild NOT owed (kernel_opt is not in the release set).

## Tasks

- [x] T0 Claim row pushed (`6c65a3bd8`) BEFORE authoring; pins read in full at
  the vendor clone; quotes byte-verified via `git show` greps this session
- [x] T1 Rider 1 (kv-alike gate) appended to `entries_gpu_95` gpu_concept
- [x] T2 Rider 2 (retained-rows content oracle) appended
- [x] T3 `batch247_oracle_riders_landed` needle+parse test in corpus tests.rs
      (needles from the authored corpus text, corpus count asserted 848)
- [x] T4 Gates: kernel_opt lib slice (3134/0/1i), clippy ×2 postures (default,
      kernel_opt+latent_retrieval) 0 errors, retrieval floors HELD 21/21 @312.88s
      (no re-pin — text-only append, byte-identical numbers)
- [x] T5 Research note 279 born-complete
- [x] T6 Docs: AGENTS kernel_opt bullet (×2 sites), `.docs/11_domains/kernel_opt.md` row
- [x] T7 Snapshot landing row + commit + push
