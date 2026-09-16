# Bench 094 — the default-level fired histogram (Issue 119's T4 successor)

**Status:** RECORD — landed 2026-09-17 (data + corpus_coverage join + 3 new tests; lib 1694/0)

## The question

Issue 119's first `--coverage-report` run found its real gap in the evidence
axis, not the candidate list: **0 of the 449 default-firing uncovered lints
carried ANY frequency evidence**. The local histogram (Bench 061) was
captured with pedantic+nursery ENABLED over already-clippy-swept sibling
repos, so it ranks the opt-in tail and is structurally blind to the debt a
plain `cargo clippy` shows an ordinary user. The report printed the DISJOINT
cross-tab and said so; the filed successor lead was "a default-level fired
histogram".

## The design — and the two populations refuted before the one measured

**Sibling-repo git history (the "pre-heal" idea) — REFUTED.** Pre-heal ≠
pre-clippy-clean: every workspace repo has enforced `cargo clippy -D
warnings` in its CI/guard since long before the healer existed (2026-08-24
adoption; the guards predate it by months), so default-level warnings were
being fixed at write time all along. No commit of ours carries unhealed
default-level debt in quantity; the git history is blind in exactly the same
direction as current HEAD. The healer's own intake rows (a handful of live
findings per sweep) are the residue — and they are already harvested by the
post-mining intake, not by a histogram.

**Li et al. ICSE 2024 transcription — REFUTED, measured.** The obvious
cheap source (`arXiv:2310.11738`, 8,095,259 warnings over 94,715 crates.io
latest releases) carries its per-lint distribution ONLY as Figure 2, a bar
chart with **no printed value labels** (verified by extracting the embedded
2728×1932 image and reading it — both a vision-model pass and a direct
read; only the top-1's 4,887,114 is printed, in body text). The "detailed
data in the supplementary" is not on arXiv (no ancillary files) and no
public dataset was found. Bar-height reading is estimation, and
"inventing per-lint counts from a shape claim is the thing this axis exists
to refuse" (the standing refusal in `corpus_coverage.rs`, now upgraded from
assumed to measured). The figure's NAMES are legible, and they carry their
own finding — see "the ecosystem head is opt-in" below.

**So: measure it ourselves.** Population = crates.io real code NOT curated
by this stack. The crates.io list API supports only `sort=downloads`
descending, and API-blocks pages > 100 ("unavailable for performance
reasons"), so the accessible window is the **top-10,000 crates by total
downloads**. Sample: five strata (rank bands 1001-1100, 3001-3100,
5001-5100, 7501-7600, 9501-9600), 40 rows each, interleaved round-robin —
200 candidates, filters `not yanked ∧ max_stable_version present ∧
downloads ≥ 1000` (mechanical rule only; the exact pages are recorded in
the manifest's `selection_rule`).

Measurement: plain `cargo clippy --quiet --message-format=json` — **default
lint level, no `-W`/`-A` flags**, lib+bins, host target — over each
extracted `.crate` latest stable release. Only `clippy::*` codes tallied
(the namespace law; rustc rows excluded). A crate with any hard error is
excluded from the histogram with its reason recorded. Dependency warnings
are cap-lints-suppressed as usual, so counts are the crate's OWN code —
same population semantics as Li et al.

Captured 2026-09-16T19:52Z, clippy 0.1.98 (48a229ceae 2026-09-01) — the
SAME toolchain as the coverage report's derivation, so this source needs no
cross-toolchain warning. Data:
`.benchmarks/data/094_t01/manifest.json` (rows `[lint, count, crate_spread]`,
per-crate verdicts, `dataset_blake3 079cfddf78ee0d55b8eafd56343a2519068a3b1b9cbdbea624923f09c890a5e7`).

## Results

200 sampled → **184 compiled clean** (16 compile_error, 0 timeouts) →
**761 warnings, 105 distinct default-firing clippy lints**.

The join (`FreqEvidence::FiredDefault`, tier 1 — same-level evidence ranks
above the opt-in-biased local capture, below dual-evidence):

> **of the 448 uncovered lints that FIRE by default, 74 now carry evidence**
> (was 0). Ranked head: `needless_borrow` 40/12-crates, `missing_safety_doc`
> 37/3, `legacy_numeric_constants` 23/5, `needless_borrowed_reference`
> 15/5, `too_many_arguments` 13/3, `manual_range_contains` 12/3,
> `doc_overindented_list_items` 11/5, `doc_lazy_continuation` 8/4,
> `partialeq_to_none` 9/3, `clone_on_copy` 9/3 …

### The concentration law — counts without spread mislead

**The nominal top-1 is a single-crate anecdote**: `write_with_newline` 248
firings came from exactly ONE crate (`nano-gemm-codegen-0.2.1`, which alone
carries 294 of the sample's 761 warnings — 39%). **Median warnings/crate is
0**: 132 of 184 crates are default-level clean; 52 carry warnings; the
distribution is a spike plus a thin tail (top-1 crate 32.6% of all
warnings; top-5 lints 51.6% — Li's long-tail shape survives, but the head
is thin). That is why the manifest's row schema is
`[lint, count, crate_spread]` and the spread column is part of the dataset
identity: a count concentrated in one crate ranks as an anecdote, not a
population. `needless_borrow` (40 over 12 crates) is the robust head row;
`write_with_newline` (248 over 1) is a mining lead DESPITE its rank, not
because of it.

### The ecosystem head is opt-in — the disjoint finding, confirmed at source

Li et al.'s Figure 2 top-20 names are almost entirely
pedantic/restriction-group (`default_numeric_fallback`, `as_conversions`,
`arithmetic_side_effects`, `unwrap_used`, `cast_*` …) because their run used
each project's OWN `[lints]` configuration — and it is TEAMS THAT CONFIGURE
CLIPPY whose warnings dominate a configured run. At default level (this
bench), pedantic/restriction names appear only where a sampled crate opted
in (`expect_used` 4, `cast_possible_truncation` 6, `nonminimal_bool` 7 —
via published `[lints]` tables). The two histograms measure different
questions and are both needed; neither substitutes for the other.

### Distribution facts for the mining discipline

- top-1 lint 32.6% / top-5 51.6% of default-level warnings — flatter than
  Li's project-configured >50%/>80%, because opt-in lints dominate
  configured runs.
- `doc_lazy_continuation`: **8 firings over 4 crates** — the open intake
  item's "evidence-gated, unmet" gate is now MET by this histogram (plus
  sibling `doc_overindented_list_items` 11/5).
- The top NEW gap candidate: `write_with_newline` (parse-only-provable
  println→write-line shape; uncovered in the corpus despite being
  clippy-MachineApplicable — the P21 oracle lane cannot consume it without
  a corpus rule). Spread caveat above applies.
- `legacy_numeric_constants` 23/5, `manual_range_contains` 12/3,
  `partialeq_to_none` 9/3, `clone_on_copy` 9/3 — all uncovered, all with
  multi-crate spread.

## Limitations (each one a boundary of the claim)

1. **Band bias**: the sample is the top-10k-downloads window (API page
   block). Better-maintained than the median crate; the hyper-head
   (serde-class, mostly warning-clean AND clippy-curated) is excluded by
   the strata start at rank 1001. Li et al. sampled ALL latest releases —
   the two populations differ and their counts must never be summed.
2. **Platform axis**: 16/200 compile failures are mostly cross-platform
   (`core-graphics`, `kqueue-sys`, `wayland-sys`, `x11-clipboard`,
   `esp-idf-sys` — macOS/BSD/Linux/embedded-only on a Windows box). A
   Linux rerun compiles more crates and shifts counts; the manifest records
   each exclusion with its stderr tail.
3. **One configuration**: default features, host target, lib+bins. Warnings
   behind non-default features are unobserved (true of every single-config
   run, including Bench 061's).
4. **Build scripts run** during clippy — arbitrary code execution from
   third-party packages is inherent to measuring compiled warnings; the
   sample is bounded, named in the manifest, and was fetched once.
5. **Toolchain-pinned sample of a moving target**: clippy 0.1.98; the
   surface moves as clippy ships lints — same caveat class as the coverage
   report's derivation floor.

## The join (code)

- `corpus_coverage.rs`: `FreqEvidence::FiredDefault(u32)` (tier 1),
  `join_evidence` gains the third source with documented precedence
  (fired+study → `Both` regardless; any `fired_default` → `FiredDefault`,
  shadowing a lone local/study count — the shadowed count remains in its
  manifest, never summed), `CoverageInputs::default_histogram{,_provenance}`,
  `firing_with_default_evidence()`, render arms for the second provenance
  line + stale rows + the cross-tab breakdown.
- `bin/heal.rs`: `load_default_fired_histogram` — same OPTIONAL contract as
  the Bench 061 loader; absent manifest renders "default-eco: NONE joined",
  never a silent zero.
- 3 new tests (gap-closure + render, disjoint-still-live-when-absent,
  stale-row counting); `lib 1694/0` at default features; clippy clean on
  the write set. Score-bench not run: no proposer or corpus entry changed —
  the fixture corpus and drafting are untouched, so flatness is by
  construction.

## Extend protocol

Re-run the sampler against fresh crates.io pages (or wider strata via the
data-access dumps when the API block is bypassed) and regenerate the
manifest with the same schema; re-pin the blake3 in this doc; re-run
`cargo heal --coverage-report`. The population floor that matters is
`compiled_clean` — a rerun that lands far below ~150 crates is a sampling
regression, not a quieter ecosystem.
