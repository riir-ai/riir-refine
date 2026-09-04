# Research 129 — the MTPLX upstream residue distill (Batch 106, Plan 076)

**Status:** RECORD — Batch 106 landed same day; corpus 477 → 482 (D 106 → 111, 5 rules).

## Source

- **Repo:** youssofal/MTPLX, upstream main @ `dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d`
  (tag v2.11.1, committed 2026-09-04T03:25:00-07:00), **Apache-2.0**.
  Fresh FULL clone at `.raw/mtplx-upstream` (`core.longpaths` — mistake filenames
  exceed Windows MAX_PATH). Clone removed at landing per the `.raw` discipline.
- **Population (the three surfaces Batch 103 never read):** upstream `mistakes/`
  = 16 files (7 upstream-only vs the 9 the retired vendor clone carried);
  `docs/plans/` 2 files (1,217 lines), `docs/specs/` 1 file (508 lines),
  `docs/concurrency/` 1 + `concurrency.md` (259 + 113 lines); targeted upstream
  diff `8bc4d88..dd4031a8` kernel surfaces (native_extensions vendored kernels +
  mtplx/kernels/qsa_prefill_direct.py, the only .metal addition in the range).
  The queue row's "300 files" figure was the whole-diff file count; the
  kernel-shaped subset is the two clusters above.
- Every quote below re-verified against the pinned clone before landing.

## Why upstream, not the PR diff

Batch 103 mined PR #391 (the restack branch). Upstream main has NEVER been
mined and carries the operator's own post-mortem ledger — `mistakes/` is a
curated corpus where **the filename IS the rule** (highest confidence-per-token
axis this program has seen; the B98 ds4 batch's external-instances rider
pattern at 16-file scale). Head `dd4031a8` is also the fix commit for the
queue row's B+ lead, so the lead's fix shape is IN the pin, not reconstructed.

## The B+ lead — CONFIRMED as a rule (both instances found, third rider found)

**`prefix-reuse-key-must-mirror-the-restore-predicate`** (D). Two independent
production instances, mirror-image directions of one defect: a cache/identity
key that does not ask the question the RESTORE path answers.

1. **Over-keying (identity too narrow).** `mistakes/per-request-steering-…bank-fingerprint….md`:
   the bank's `policy_fingerprint` carried `tool_choice=` and the
   post-tool/force-answer/Pi-convergence flags while those bytes rode a
   transient trailing sentinel — "identical token prefixes were treated as
   different cache identities", so "every transition round re-prefilled the
   session cold" (41 s + 44 s at 41k). Fix quote (mtplx/server/openai.py
   `_policy_fingerprint`, L19831-19840):
   > "A bank entry is exact for its token ids; the fingerprint only has to
   > carry what changes the KV or the MTP history WITHOUT changing the tokens
   > (model, draft head, depth policy, history policy)."

2. **Under-keying (estimator asks less than restore serves).** Commit
   `dd4031a8` itself: the 2.10.2 pre-prefill memory guard asked
   `SessionBank.longest_prefix` (EXACT containment) while the restore path
   serves by BLOCK prefix — near the 97% Metal line the shed "read a full
   miss, cleared the session, and the release gate's tool_result_forced turn
   re-prefilled 41,901 tokens cold (54 s)" with a 41,391-token block restore
   available. Fix (openai.py L17541-17549):
   > "Ask the bank the question the restore asks, or the estimate reads 0
   > here and the session's own restorable entry is evicted below as
   > 'superseded'."

   `longest_shared_prefix_tokens`' docstring generalizes the mechanism: "The
   restore path serves prompts that no entry is an exact prefix of: a
   block-prefix restore rewinds to the last safe boundary under the common
   prefix … A memory estimate that asks only `longest_prefix` (exact
   containment) reads 0 for such prompts and calls the session compacted."

3. **Rider instance (third surface, same family).** The numerics-profile spec
   (docs/specs/2026-08-09, §Failure-mode check): "Reusable state crosses
   numerics profiles … include the profile and route fingerprint in reusable
   cache/session identities and reject mismatches before restore" — the
   fingerprint must also carry everything that changes KV semantics without
   changing tokens (a BF16-accumulation-order change is exactly such a thing).

In-stack analog: the Issue-742 GDN+KV prefix cache (441× TTFT) — the first
MTPLX row with one. Sibling of B103 `shape-class-keyed-capability` one layer
up (capability verdict ↔ shape tuple; cache identity ↔ the KV-determining
policy set). Distinct from B94 `cross-run-profiles-key-on-content-not-stat-triplets`
(that decides WHAT the key derives from; this decides that the key set must
EQUAL the restore predicate — no more, no less — and that every estimator
asking "can I reuse?" must derive through the same question).

## Axis B — the mistakes/ corpus (16 files, all read)

**Corpus rules (4, all D):**

| # | rule_id | source file (title IS the rule) |
|---|---|---|
| 1 | `prefix-reuse-key-must-mirror-the-restore-predicate` | per-request-steering…bank-fingerprint (lead, above) |
| 2 | `request-arrival-work-asks-reuse-state-first` | a-request-arrival-optimisation…never-await-unconsumed-work |
| 3 | `serving-raises-only-behind-strict-flag` | a-benchmark-arm-assertion…raises-only-behind-a-strict-flag |
| 4 | `derived-key-single-constructor-parity-test` | wizard-rebuilt-the-tune-state-key…parity-test |

**Rule 2 anchors** (`mtplx/generation.py` `_bank_may_preempt_first_span`
L805-830 + `shares_ram_prefix` session_bank.py L1094-1111 + commit `6fd0c67`):
the default-on `MTPLX_QWEN4_PLE_FIRST_GATHER_EARLY` predicted the first chunk
"from the prompt alone", gathered 650k n-gram rows at request arrival, and
blocked on the gather at scope exit — "On a warm bank turn the prefill starts
40k tokens later; the gather was pure waste and its wait was pure dead time".
Fix: "anything that runs at request arrival must ask the session bank whether
the prefill will start at token 0 … and an owner thread must never wait for
work it will not consume (cancel, orphan, move on)." The probe is cheap by
construction ("One tuple-slice compare per entry, no cold-tier scan, no lookup
side effects") and duck-typed with a conservative default ("banks without the
probe are treated as unable to preempt, which keeps the pre-2026-09-03
behaviour").

**Rule 3 anchors** (`mtplx/ple_prefill_lookahead.py` L102-109 + L243-254):
> "Measurement law vs serving contract. The engagement verdict below exists so
> an A/B can never report the control under the candidate's label … an armed
> lane that quietly served zero chunks RAISES. That is the right answer for a
> benchmark arm and the wrong one for a user: on 2026-09-03 the same raise
> reached a live serve as an SSE finish_reason: 'error' after a 30 s cold
> prefill of a 31k-token prompt … Serving therefore records the verdict
> (counter + last_scope_status()[…] + one warning) and keeps generating;
> measurement arms export this flag and get the raise."

Plus the one-warning-per-class-per-process cap ("a serve that hits this on
every request must not turn its log into the receipt it failed to be") and the
pre-arming check ("Before default-arming any lane from a port, grep it for
`raise` on paths a request can reach"). COMPOSES with B103
`request-shape-decline-mirror-completeness`'s "armed-lane-must-engage raise":
that is the MEASUREMENT side (an armed arm silently serving stock is worse
than an outage → raise); this is the SERVING side (the same verdict reaching a
user IS the outage → count + warn). Cross-referenced in-rule.

**Rule 4 anchors** (`mtplx/commands/public.py` `_tune_state_context_for_args`
L4142-4156 + `tests/test_tune_record_wizard_parity.py` header + the mistake
file): the 2.8.0/2.8.1 wizard "rebuilt the settings dict by hand (stale
performance-cold profile, unjoined depths, missing keys), so its lookup hash
could never match the record tune had just saved — users were offered tuning
again on every start" (issue #280: "machine heats up with zero requests").
Fix: "Every reader of the tune-record store MUST derive its key through this
function" + a drift-failing parity test. Second half (provenance markers): the
wizard's model picks "carried no explicitness markers (`_cli_flags`/
`_model_explicit`), so tune resolved the hardware default (picked 4B → tuned
27B)"; and public.py L1435: "a config value that happens to EQUAL the parser
default (0.6/0.95/20) is still the user's standing pin — the bare
`in (None, 0.6)` checks read it as 'unset' and silently replaced it". In-stack
cache-key cousins: B29 `weight-handle-cache-keyed-by-allocation-identity`,
B94 `cross-run-profiles-key-on-content-not-stat-triplets`, B39
`autotune-cache-extract-outlives-owner` — all single-constructor keys; the NEW
content is the parity-test requirement + the save/lookup duplicate rot
mechanism + the provenance-marker half.

**Rule 5 (axis A find): `vendor-fused-op-instantiation-check`** (D). Anchor
(docs/perf/deepseek-v4-dispatch-levers.md §sdpa): "MLX takes `sinks=`
natively, but its fused Metal kernels are only instantiated for head dims
64/96/128/256 (0.31.2) and 64/96/128/192/256 (0.32.0 …) — **verified by
reading the symbol table of each shipped mlx.metallib**. DeepSeek-V4's MLA
latent is **512** wide, so `sdpa` takes MLX's own unfused fallback on every
version available here; `fused` is that same fallback minus two copies over
the block, and wins." Decision content: before adopting, tuning around, or
benchmarking a vendor fused op, verify the ACTUAL instantiated specializations
in the shipped binary (symbol table / metallib dump), not the API surface —
a fused op whose specialization set misses your shape silently routes to the
vendor's own fallback, and an A/B "fast path" measures that fallback. The
consumer-side complement of B23 `hardcoded-fast-path-jit-specialization`
(producer-side which-kernel) and the vendor-binary cousin of B103
`shape-class-keyed-capability` (our own gate; this is the vendor's shipped
set). In-stack analog: Issue 731's coverage hole — the attention fast path
routed only (8,4,256,50) and everything else silently took the parametric
kernel with ZERO parity tests.

**Riders / doctrine citations (no corpus rules):**

- **p99 inter-emit gap census** (`mean-tps-and-sliding-averages-hid-thirty-…`):
  "felt smoothness is the p99 inter-emit gap, so census gaps in every request
  record"; "Gate ceilings must sit just above the measured-good distribution,
  not an order of magnitude up"; "When a probe's mean is green but the user
  feels stutter, histogram the gaps before exonerating anything." External
  confirmation of our percentile doctrine (Issue 853's p99-is-the-max class;
  the league doc's wall-clock medians). → cite in the league doc at next
  touch; also the 2-3 sentence ceiling-placement rule is the percentile-floor
  gate's philosophy (scripts/percentile_floor_gate.py) independently arrived at.
- **Stale-binary provenance** (`two-ab-rounds-measured-a-stale-…`): verify the
  resolved binary path + probe "for a marker only the new code has" before
  trusting any A/B; "when two different code changes produce IDENTICAL wrong
  pixels, stop debugging the code and verify WHICH code is running." External
  confirmation of Bench-749 (the 0.02× cold-start artifact) + Issue-866's
  marker-probe; the Python twin (site-packages shadow + neutral-cwd import
  check) is the same lesson for interpreters.
- **git-stash-is-shared-by-every-worktree** (`git-stash-is-shared-…`):
  "refs/stash lives in the common git directory and is one stack for all
  worktrees"; fix = `git show HEAD:<path>` swaps + `git fsck --unreachable`
  recovery, pin the other lane's commit under a recovered branch. External
  confirmation of the global AGENTS.md stash rule + the hash-object/
  update-index fix the workspace already documents (katgpt-rs AGENTS.md
  staged-set section). Research record only.
- **Delivered cadence ≠ server TPS** (`every-desktop-chat-froze-…`): the
  uncapped-client-path rule (every benchmark request passed max_tokens, so the
  holdback never engaged in QA but engaged on every real chat); `read(n)`
  coalesces chunked frames into self-made 8 s stalls — `read1()`/line
  iteration is the honest cadence read; "warm/benchmark-cosmetic GPU work
  does not belong in product profiles" (echoes our Bench-641/760 cold-start
  and product-vs-harness-profile separation).
- **Census vs wall-clock separation** (DSV4 dispatch-levers doc): "the census
  above establishes dispatch structure only and is not presented as E2E
  timing" + the 91% attribution (census predicted ~15 ms host encode, window
  realised 13.7) + "once the host encode is gone, decode is GPU-forward bound,
  not qmm-ALU bound" (the mlx-0.32 clean null). Fourth instance of the
  micro-vs-e2e doctrine (Bench-641/B77 third instance noted in B103).

**Out-of-scope dispositions (read, no Rust/kernel analog):**
`a-lazyvstack…` (SwiftUI/AppKit lazy-container scroll ownership),
`hands-off-automation…` (CFRunLoop .beforeWaiting starvation under input
storms — the phase-aligned stall census is good methodology, iOS-only),
`an-isolated-onboarding-qa-run…` + `when-the-user-forbids-app-launches…` +
`pytest-tests-rewrote…` (QA-hygiene; the autouse-scratch-override rule is
already our test-artifact-hazard discipline), `piped-build-output-masked…`
(pipeline exit-status; our `set -o pipefail` + the T16 exit-code discipline),
`silently-stripping-dead-tool-call-markup…` (suppression-never-silent —
echoes Issue 709's loud-stub pass, serving-UX shaped), `felt-speed-died…`
(app-side O(transcript)-per-frame; the "severity scales with conversation
length, fresh-chat validation sees zero" is B104 rule-2's fixture-realism
echo).

## Axis A — docs + targeted diff

- `docs/concurrency.md` + `concurrency/qwen35b-mtp-batch.md`: the generic-mode
  vs backend-lane split ("The mode name is generic. It does not define a batch
  width … Those are properties of the installed model/backend implementation")
  is a clean statement of the capability-vs-configuration boundary B103's
  decline-mirror rule already encodes; the width-neutral session-bank identity
  paragraph ("entries bank and restore identically across solo, B3, and B8 …
  the policy fingerprint does not embed the physical width") is rule 1's
  over-keying half restated.
- `docs/specs/…numerics-profiles-design.md`: attribution-before-kernel-work +
  one-candidate-at-a-time promotion with paired rounds + pre-registered floors
  (300 greedy / 95%-of-control default-sampler) + the balanced-route candidate
  ladder recorded with receipt SHA-256s — our GOAT doctrine, externally
  confirmed end-to-end; no new rule (the spec's operator-selection content is
  theirs, the METHOD is ours already).
- `docs/plans/2026-08-08-qwen35b-eight-way-mtp.md`: fail-closed construction
  contract, cohort sealing, `/tmp/mtplx-gpu-exclusive.lock` exclusivity — all
  covered (B100 fail-closed family; our GPU-exclusivity rule).
- Upstream diff `8bc4d88..dd4031a8` kernel subset: the vendored Steel
  sparse-GQA kernel (oMLX PR #3244) — B100's `dispatcher-eligibility-predicate-
  lags-kernel-capability` is ITS origin (the #4416 predicate lag), and the
  896-thread static_assert rider (issue #400, M2/M3 threadgroup ceiling) is
  B100 `register-budget-head-split-gqa-reread-trade`'s process-ceiling
  companion — recorded as riders, no new rule. `qsa_prefill_direct.py`'s
  per-dtype first-dispatch proof (prove bfloat16 says nothing about float16;
  FAILED is terminal; no dense fallback, errors re-raised) is rule 3's
  fail-closed lane state machine in production form — folded into rule 3's
  concept as corroboration.
- `docs/perf/` beyond DSV4: `agent-loop-cache-vs-vllm-20260903.md`,
  `laguna/`, `qwen27b-gdn/`, `receipts/` — NOT in the queue row's population;
  left for a possible future residue pass (recorded here so the next miner
  knows they are unread, not unfruitful).

## Dedup ledger (grep of buggy/fixed BODIES, not just concepts — the B66 lesson)

- `prefix-reuse-key-must-mirror-the-restore-predicate` vs
  `shape-class-keyed-capability` (B103): capability VERDICT keyed by shape
  class vs cache IDENTITY keyed by the restore predicate — different surfaces;
  cross-referenced one-layer-up in-rule. vs `cross-run-profiles-key-on-content-
  not-stat-triplets` (B94): what-derives-the-key vs the-key-set-equals-the-
  restore-predicate. No body overlap (B94's fixture is a stat-triplet cache
  key; this is a token-prefix/fingerprint pair).
- `request-arrival-work-asks-reuse-state-first` vs `gate-prep-fold-into-
  consumer-via-packed-side-tensor` (B77): that folds prep into the consumer's
  dispatch; this GATES arrival-time prep on the reuse state + forbids awaiting
  unconsumed work. vs `sibling-dispatches-before-any-readback` (B36): enqueue
  ordering, not arrival gating. vs `fill-gate-speculation-until-working-set-
  resident` (B94): admission latch for speculation, not reuse-state probing.
- `serving-raises-only-behind-strict-flag` vs `request-shape-decline-mirror-
  completeness` (B103): the two halves of one verdict-delivery policy
  (measurement raises, serving records) — COMPOSED in-rule with explicit
  cross-reference both directions, not merged (different fix-spaces: B103
  mirrors decline branches; this re-routes the verdict through an env gate).
- `derived-key-single-constructor-parity-test` vs B29/B39/B94 (above): key
  CONTENT rules; this is the single-constructor + parity-test + provenance-
  marker contract. No corpus rule requires a drift-failing parity test.
- `vendor-fused-op-instantiation-check` vs `hardcoded-fast-path-jit-
  specialization` (B23): producer-side specialization vs consumer-side
  verification of a VENDOR binary's shipped specialization set. vs
  `cooperative-tensor-mma-capability-route` (B105): OUR capability probe
  defaulting false; this reads the VENDOR's shipped set. Distinct axes.

## Drop ledger

- 11 mistake files dispositioned out-of-scope (listed above with reasons).
- `hyper` admission mode, cohort width tables, the 35B numerics ladder:
  model-specific serving policy, not kernel-shaped decision content (the
  B103 non-goals held).
- The B103-era queue-row sub-claim "docs/plans 1217 lines unread" is now read;
  "docs/concurrency 259" likewise. The kernel-shaped doc residue is exhausted;
  `docs/perf/` subdirs + `docs/research/` are the named-unread remainder.

## Batch record (Batch 106 / Plan 076)

- 5 rules, corpus **477 → 482** (D 106 → 111), new slice `entries_gpu_19.rs`.
- Fixtures: original minimal Rust reproductions (parse-only; suggestion-only
  rules — no `--fix` path, the B77/B92/B103 architectural precedent).
- GOAT: batch106_rules_present + batch106_concepts_covered +
  batch106_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal
  lib + default lib + kernel_heal+latent retrieval floors + clippy 0 both
  states. Count pins walked (tests.rs 75/5368/5382 + expected-array D +
  router.rs 461 + mod.rs D header/list + the "464-rule" doc line — found
  2-batches stale at 464 post-B104/B105, repaired to 482).
- `.raw/mtplx-upstream` removed at landing; pin recorded above.
