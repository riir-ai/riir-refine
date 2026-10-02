# Research 173 — Batch 146: the accumulated read-lane pack (nnethercote July-2026 + bevy 0.19)

**Status:** RECORD (Batch 146 landed 2026-09-16 — corpus 614 → 619: rust_perf 127 → 131, dist 5 → 6)

## Verdict

YES — executed on the accumulation rule, not a fresh distill. Two staged
charter inputs landed the same day (the nnethercote read-lane's own
condition — "stage until a second input arrives" — was MET by the bevy 0.19
read-lane), exactly the Batch-145 precedent. Both sources were RE-VERIFIED
against the live posts at admission (coordinator reads, not the charter
text): every load-bearing number below was checked verbatim against the
source before any entry was authored.

## Sources (re-verified 2026-09-16)

- **nnethercote, "How to speed up the Rust compiler in July 2026"**
  (2026-07-31, nnethercote.github.io/2026/07/31/how-to-speed-up-the-rust-
  compiler-in-july-2026.html — note the .github.io domain; nnethercote.com
  does not resolve and the r.jina.ai fallback inherits that failure).
  The post was PARTIALLY mined 2026-09-05 (#160005 → the shipped
  `hot-byval-move-over-memcpy-threshold`).
- **bevy 0.19 release notes** (2026-06-19, bevy.org/news/bevy-0-19/).
  TIMELY: the workspace migrated bevy 0.15→0.19 across five repos this week.

## Rules landed

### rust_perf (4, all suggestion-only, honestly NotBenchable)

1. **`dispatch-pass-fusion-skips-empty-handlers`** (complexity) — a visitor
   pipeline calling every handler's `check_foo` on every node pays N virtual
   dispatches, "the vast majority" entering empty bodies; fuse the passes so
   only implemented handlers are registered per node kind. Verified quotes:
   "reduced Clippy's runtime by 10-30%! Branch mispredictions were reduced
   by 20-80% on real-world examples and by 97% on one stress test" (xmakro
   #17124/#17132 merged; #157762 the unmerged twin with the measurements).
   Distinct-from: `dyn-to-enum` (one call's shape vs the call COUNT) + the
   two-data-passes fusion rule (data traffic vs dispatch).
2. **`hot-alloc-struct-fits-cache-line-class`** (layout) — a high-count
   allocation-site struct just above a line-class boundary streams zero
   whole elements per line; shrink/reorder under the boundary or move rare
   arms' cold fields to a side table. Verified quotes (#158720 + paving
   #158942): "72 bytes to 64 bytes. (Small enough to fit in a cache line.)"
   → ">10%" wall on AST-heavy benches, "cache miss rates ... fell by as
   much as 29%"; "stored temporarily to the side"; "104 bytes a few years
   ago ... chip, chip, chipping away". Distinct-from: the `#[repr(align)]`
   false-sharing rule (multi-thread coherence vs single-thread capacity),
   `hot-byval-move-over-memcpy-threshold` (move cost vs stream capacity),
   AoS→SoA (ownership restructure vs same-struct shrink).
3. **`dense-table-bulk-update-via-contiguous-iter`** (bevy) — bevy_ecs 0.19
   `contiguous_iter_mut` exposes the engine-owned table slice so LLVM
   auto-vectorizes a bulk-UNIFORM update. Verified table (10k entities):
   5.58/5.51 → 4.88/1.87 µs (+ bypass_change_detection 4.40/1.58); the three
   density gates are API contract (table storage; `Changed<T>`/`Added<T>`
   forbidden — "they require a per-entity check that makes it impossible to
   return raw table slices"; archetypal fine). DUAL-EVIDENCE law: our own
   Bench 930 / Issue 960 measured the view layer's real hot paths NEGATIVE
   (snapshot-driven random access; the one legal site 9-10% slower) — the
   entry carries BOTH sides, pinned mechanically by
   `batch146_dual_evidence_and_distinct_rows_present`.
4. **`gpu-upload-repack-skipped-when-layout-matches`** (layout) — a per-frame
   upload's serialize/repack pass over data whose `#[repr(C)]` layout
   already equals the WGSL target is the identity transform; a direct byte
   copy (`bytemuck::cast_slice`) is bit-identical. Verified quote: "Bevy
   uses the `encase` library ... In practice, this isn't always needed and
   incurs unnecessary overhead. We've swapped to cheaper direct memory
   copies when clustering lights". The verification precondition (repr(C) +
   static sizes + a pinned layout test) is the rule's other half — a layout
   drift is a correctness bug the copy cannot see.

### dist (1, suggestion-only, BuildTime family)

5. **`pgo-training-set-covers-real-workload-hint`** — verified quote
   (#159091): "added rustdoc benchmarks to the PGO training set ... mean
   2.85% reduction in wall-time across all the rustdoc benchmarks, with the
   largest reduction exceeding 6%. PGO is fiddly to set up but it can really
   make a difference, and the training set matters." ADMISSION-TIME DESIGN:
   the scanner can see NOTHING about PGO (it lives out-of-manifest), so the
   rule is proxy-triggered on the fully-tuned audience (ships_bin ∧ all
   three effective knobs strip+lto+panic set — own OR root) and STRICTLY
   ADDITIVE: silent for every manifest with a missing knob, fires alone on
   a fully-tuned one. The `-hint` suffix + conditional advice ("IF you use
   or add PGO") + measure-first caveat follow the
   `large-api-dependency-mostly-unused-hint` precedent.

## Admission-time corrections (the honest record)

1. **Charter name amended** `pgo-training-set-covers-real-workload` →
   `...-hint` (the proxy-trigger proxy-hint house vocabulary).
2. **Two dist gates pinned "complete profiles stay silent"** —
   `lib_only_and_complete_profiles_stay_silent` (corpus.rs) and
   `canonical_repo_manifest_fires_three_and_completes_to_silence`
   (dist_gates.rs). Both updated to the Batch-146 contract: a fully-tuned
   bin earns EXACTLY the PGO hint (ship-size rules silent) — the original
   intent (no ship-size noise on tuned manifests) is preserved and now
   pinned by value (`ids == ["pgo-training-set-covers-real-workload-hint"]`).
3. **Three annotated-count pins found by the test run, not by my grep**
   (`forty_three_annotated_rules_carry_context_notes` 70→74,
   `context_notes_backward_compatible_serde` 70→74,
   `drafter_propagates_context_notes_from_perf_corpus` 70→74) plus the
   `entries.len()` 127→131 / `without_spec` 36→40 pair — the count-pin walk
   discipline: run the suite, read every failure, move every pin WITH its
   comment.
4. **`str::as_str` is unstable (E0658)** — `DistFinding.rule_id` is already
   `&'static str`; the pre-existing pattern is `map(|f| f.rule_id)`. Both
   new call sites fixed to it.
5. **The score-bench bin lane's required-features trap fired again** (the
   Issue-91 class): `--features score_bench` alone errors; the bin demands
   `kat_service` too (`--features score_bench,kat_service`). Noted here
   because cargo now ERRORS instead of silently skipping — the inert-lane
   failure mode is closed on this toolchain.

## Dedup (coordinator re-verification, vocabulary translation)

All five NOVEL, re-grepped against the full rust_perf + kernel_opt corpora
before authoring (3+ name variants per concept): visitor/dispatch fusion
hits are kernel arithmetic pass-fusion (different axis); cache-line hits
are the align/false-sharing rule; zero PGO/training-set hits anywhere;
contiguous/dense-table hits are AoS→SoA + loop-order; repack hits all PAY a
repack (none SKIP one). Every entry carries its Distinct-from rows
mechanically pinned by the new batch146 gate test.

## GOAT (G1–G4, measured 2026-09-16)

- **G1** parse/presence/annotation: `batch146_rules_present_and_annotated` +
  `batch146_pseudocode_parses` + `batch146_dual_evidence_and_distinct_rows_present`.
- **G2** lib tests: default **1544/0**; `latent_retrieval` **1694/0**;
  `kernel_opt,latent_retrieval` **2041/0**; `rust_perf_bench` corpus slice
  **146/0**. Retrieval floors HELD byte-identical: blended **94.7/96.9**
  (op-bigram), clippy top-1 **89.6%** — the 4 new suggestion-only rules
  joined the fan-out without decay.
- **G3** clippy `-D warnings` clean on the write-set in 3 states (default
  `--tests`, `rust_perf_bench --tests`, `latent_retrieval --lib`); score-bench
  **FLAT heal_rate 96.2%** — row **#119** appended (rule_outcomes: 4 rules
  single-target, 24 multi-target skipped loud) + frontier snapshot appended
  (generation 7); cross-OOO axis unchanged (3 absorption candidates,
  identical to the standing record).
- **G4** zero new deps, zero new stores — corpus data + one detect arm in
  the dist line scanner.

## Killed at admission (dedup survivors the charter already killed — recorded)

nnethercote #157179 (short-sort-key — the extract-a-cheap-key family),
#158794/#159115 (dep-read dedup — caching family), #155678 (query merge —
caching family), #159623-family (rustdoc-internal, no user fix space);
bevy change-lists/last-frame-clustering (recompute-unchanged doctrine),
entity-removal-scan-from-end (too thin standalone), NoCpuCulling /
partial-bindless / GPU-clustering (engine internals, no user-code fix
space). INTEL, not rules (bevy): Metal partial-bindless free win
(+15% frame M2 Max, −57 MB Bistro) inherited by the 0.19 migration —
verify at the next bench; wasm Task drop cancellation — correctness
inherited, no action.
