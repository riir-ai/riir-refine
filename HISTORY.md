# HISTORY.md — riir-clippy

> Archive created 2026-09-06: compacted out of `AGENTS.md` (3,883 lines →
> compact operational form). NOTHING was deleted — every section removed from
> AGENTS.md is preserved here VERBATIM (original AGENTS.md line ranges noted
> per section). Operational rules live in AGENTS.md; this file is the
> historical record. Numbering stays monotonic — issue/plan/bench/batch
> numbers are never reused, and historical citations to them name findings,
> so they remain valid.
# HISTORY.md — riir-clippy

## Issue 097 resolved — linking↔confusion PoC: NO-GO, the fusion idea retired as measured-refuted (2026-09-12)

The katgpt-rs Issue 757 follow-up gate ("no plan until a PoC on the
oracle-labeled fixtures exists") ran as `examples/linking_confusion_poc.rs`
behind `linking_confusion_poc` (the `gw_corpus` defend-wrong pattern):
93 rule clouds over the 101-span fixture canon (M=32 salted rename variants,
n=33/cloud, 8-D ModellessEmbedder), 2,158 within-domain pairs, 3,333
retrieval queries at 86.3% top-1 self rate (the confusion ground truth:
9 pairs pooled, 4 base-canon).

**Both config classes refute.** At the paper's config (k=8) the instrument
degenerates: k/n ≈ 0.24 vs the paper's ≤ 0.004 regime, 99.9–100% linked, |link|
up to 51 — zero binary discrimination. At the sensitivity control (k=3, k/n ≈
0.09) linking gains a 55.6% base rate and shows ANTI-enrichment (confusion mass
×0.48 pooled on linked vs unlinked; Fisher p=1.0 both arms). **The Spearman
trap is named in the record**: ρ reads 0.90/0.60 across the arms — invalid
under 99.6% ties at zero confusion, and it moved with the tie structure while
the confused-pair set stayed fixed. Do not re-derive it as a positive.

Mechanism: the paper's premise is smooth multi-exemplar class manifolds; the
fixtures give 1 exemplar/rule — rename ORBITS, combinatorially thin and
entangled under PCA-3D at any sane k. The manifold assumption fails on this
corpus exactly as the katgpt-rs Issue 757 follow-up feared. A corpus-scale boundary,
not a detector defect (katgpt-rs Bench 717 holds at its designed regime). Reopen
trigger: a real-repo mined span set (≥ hundreds of REAL spans/rule). The
`linking_fold_detector` stays opt-in with no consumer; the katgpt-rs Research 391
deferred guides stay deferred — their unblocker needs a different consumer
class. Reproduce: `cargo run --release --features linking_confusion_poc
--example linking_confusion_poc`.

## Orphan-report serde boundary — the riir-ai Issue 934 follow-on repair (2026-09-12, `d2e39d1f`)

The first consumer-repo landing produced the instrument's first measured
false-positive class, and the repair landed the same day. riir-ai Issue 934
(`08322e814`, the Plan-126 residue adjudication) found
`ConversationEntry.deadline_tick`/`.failed_at_tick` flagged stillborn while
being LIVE YAML-output schema: `add_auto_fail` populates both and the
`g4_2`/`g5_2b` tests pin their serialized presence. The derive reads fields
invisibly to the syntactic member-read tally — the known Serialize-only
exclusion, now in a Ser+De struct.

**The boundary is the ATTR, not the derive pair.** The probe evidence was
the riir-ai Issue 934 full flagged sample: `BehaviorConfig` itself derives BOTH
(round-trips YAML) and carried every true positive (`alarm_radius`,
`boids`, `initial_goal`) with plain `#[serde(default)]` attrs — a
Ser+De-pair exclusion would have killed the instrument's canonical case —
while the only two `skip_serializing_if` fields in the flagged set were
exactly the two false positives. Semantics: `skip_serializing_if` marks
conditionally-emitted OUTPUT-schema surface (the derive reads the field
when the condition holds); plain serde-default fields are persistence
mirrors whose functional read is what the report measures.
`#[serde(skip)]` stays a candidate — the derive never touches it, so a
never-member-read skip field is genuinely dead.

Implementation: `SerdeStructMarker` now also collects the per-struct
`skip_serializing_if` field set; `zero_external_read_fields` retains it
out of BOTH candidate paths (Default-impl and literal). Fixtures: the
exact `ConversationEntry` shape (regression pin) + the exact
`BehaviorConfig` Ser+De plain-default shape (the FN guard) — 39/39.
1247/0 lib; clippy `-D warnings` clean on both touched files (the 12
residual `--all-targets` findings are the known sibling
frontier/rustc_errors lane). AGENTS.md bullet + the `.distill` intake row
updated in-commit (`d585aae1`).

## Plan 126 — orphan-report: intake class C1, the config knob orphaned by optimization (2026-09-12)

The LAST open post-mining intake class, closing the 888-derived class-C
family (A/B Plan 124, C2 Plan 125). `cargo heal --orphan-report [PATHS…]`:
the review/sweep-time instrument the intake mandated — a two-leg REPORT
(current-state zero-external-read fields via the shared `config_audit`
scan core + git-history evidence of vanished read-shaped occurrences),
never a per-compile lint. Ungated, zero new deps (syn + git subprocess),
report-first; refuses exit 1 on an empty scan or a blind history leg (the
frontier-report law — never a confident green zero).

**Two findings that reshaped the instrument (both measured, both recorded
in the intake row):**

1. **The canonical case forced a serde-literal candidate extension.**
   `BehaviorConfig` has no `impl Default` — the designer surface is
   `#[serde(default)]` + handwritten literal tables (`hardcoded_config()`),
   so the A/B/C2 candidate net could not see the intake's OWN canonical
   case (`alarm_radius`). The orphan leg additionally collects
   constructor-init candidates from serde-backed structs (any `#[serde(…)]`
   on the item, or a derive naming Deserialize; Serialize-only excluded —
   the derive reads those fields invisibly and every one would read as a
   stillborn FP). Classes A/B/C2 deliberately keep the narrower net —
   their plans' measured rates stand.
2. **The intake's "reader that vanished" narrative is syntactically false
   for the canonical case.** Repo-wide pickaxe over ALL riir-ai history:
   `.alarm_radius` member-read occurrences never changed before the fix —
   the const-table fn was born 2026-06-06 (Plan 225 T2) and the config
   field 2026-06-08 (riir-ai Plan 245), two days LATER, with the values already
   copied into the table. The knob was BORN unwired, not orphaned by a
   refactor. The instrument fires it honestly as `[stillborn]` — first
   row at the pre-fix control tree (worktree of `88c51824b^`, sampled 9/9
   rows TRUE repo-wide). The ORPHANED machinery stays for real vanished
   readers: current riir-ai (2613 files) yields exactly one —
   `ReplayVerificationResponse.server_hash` (6→1 read-shaped occurrences
   across `23389c801` "refactor(570): dissolve riir-ffi"; the survivor is
   a test assert) — VERIFIED TRUE.

**Validation:** self-scan 1 TP / 0 FP (`PavDataConfig.ground_truth`); the
pre-fix control fires the canonical case; current-riir-ai stillborn sample
8 TRUE unwired knobs (civ `boids`/`initial_goal`/
`use_predator_weights` — the post-`alarm_radius` remainder of the same
designer surface; engine `default_tau_reliable`, `summary_kind`; rest
`server_hash`) + 3 benign-true (repr(C) Pod padding — the documented
benign subclass) / 0 FP. 37/37 fixtures (incl. a REAL-git e2e:
init→wire→optimize→orphan in a throwaway tempdir repo). 1241 lib green;
clippy clean on every touched file (healed first; the repo-wide `-D
warnings` residue is PRE-EXISTING in the 093/frontier lane's committed
files). Those stillborn rows are findings for their OWNING lanes —
recorded, not deleted (the instrument never edits).

**En-route lessons:** (a) `git rev-parse --show-toplevel` returns the
PHYSICAL path on macOS — canonicalize scan paths before `strip_prefix` or
every pathspec is silently dropped (the first e2e run read as BLIND);
(b) scanning the repo root itself yields an EMPTY rel — map it to the
whole-tree pathspec `.`, not to a dropped probe; (c) the probe-lattice +
pickaxe pair needs BOTH directions: the lattice proves the delta, the
pickaxe names the commit — neither alone convicts.

## Plan 125 — config-audit class C2: the self-referential config struct detector (2026-09-12)

The post-mining intake's top open class, consumed at the idle window (the
mining queue itself had NOTHING staged — B143 consumed the last PERF-SEARCH
row; this was the queue's correct next unit). Extends the Plan-124
`--config-audit` instrument with the C2 sub-shape: a whole struct whose
every Default-impl field has zero EXTERNAL reads — reads confined to its
own validate-shaped (`bool`/`Result`-returning) methods and test context.
Classes A/B require zero reads, so the internally-read orphan (the
PlasmaConfig shape) was invisible to them.

**The landing's own bug, caught by the probe:** syn 2 impl methods are
`syn::ImplItem::Fn` — the `visit_item_fn` visitor hook NEVER fires inside
impl blocks, so the validate-shape context (and with it the whole own-impl
tally) was silently empty and the C2-positive fixture read as live. The
temporary tally-probe printed `field_reads_in_impl: {}` over a fixture with
a validate() method — the fix is `visit_impl_item_fn` (same logic, shared
`sig_returns_validate_shape`). The lesson generalizes: any per-impl-item
context in a syn visitor needs the impl-item hook, not the item hook.

**First-contact yield (507 real files):** 1 verified TP / 0 FP —
riir-games `PetAiConfig`, the whole-struct orphan behind the C2 class:
extracted (riir-ai Issue 589's "thread them through" arc) for a consumer
constructor pattern that never materialized (zero `PetAiConfig` references
in riir-mmorpg-examples), `melee_reach` 0 reads anywhere, the other six
fields read only by the `> 0.0` sanity test. Same precision profile as
Plan 124's A (1 TP / 511 files). The struct stays (deletion is the owning
lane's call — the report is suggest-only); the finding is the record.

**C1 stays open** (orphaned-by-optimization needs git HISTORY — a reader
that vanished — a review-time/sweep-time heuristic, not a per-compile
scan; the intake row carries the shape).

## Issue 91 T1.5 — the per-attempt ring: `--fix-compile` attempt sequences to `.heal/fixseq/` (2026-09-12)

The instrumentation prerequisite both remaining rustc_errors-side tasks
blocked on, landed the same day as the T4/T8 emission halves (the same
class: emission-first, analysis-when-data-exists).

**Why:** T1's mining verdict — the trajectory kv records per-FAMILY aggregated
counters, NOT per-attempt sequences; the fix-compile side had ZERO recorded
trajectories. T2's oscillation detector (error A → fix → error B → fix →
error A = saddle-circling) and Issue 093's per-run strategy posterior both
need the ORDERED attempt sequence, which no shipped surface held. One
instrumentation, two kernel consumers (katgpt-rs 593 SaddleEscapeGate + 592).

**What:** `AttemptRecord { seq, code, span_hash, strategy_id, outcome,
errors }` on `FixCompileReport`, pushed at ALL five loop outcome sites (kept /
no-progress revert / the two io reverts / span-drift revert — the `applied` +
`reverted` Vecs lose the interleaved order; the ring keeps it).
`span_hash` = blake3(`"file:line"`) first 16 hex — stable across runs and
machines; the driver's canonical path resolution feeds it, so symlink forms
(/tmp vs /private/tmp) cannot fork the identity. `evolve::dump_fixseq`
writes one `run-<ts>.jsonl` per write-mode run into `<repo>/.heal/fixseq/`
(the `.heal/misses/` sibling; idempotent README; the dump_misses law —
warnings never gate, attempt-less runs write nothing). Review mode makes no
attempts and writes no file, by construction.

**E2E (live, this session):** a fresh /tmp crate with the E0502 borrow shape
→ `e0502-deref-removal` kept, `errors [1,0]`, exactly one ring row + the
README; the clean re-run wrote no second file.

Validation: lib 1223/1223 @ default (+3: span-hash stability/discrimination,
ring serde round-trip, dump ordered-jsonl + README-once + attempt-less
no-op), bin lane clippy clean, live e2e above. T2 remains open on
ACCUMULATION (real fix-compile runs to replay); T3 stands (analysis-only,
no data needed).

## Issue 089 T4+T8 — the evidence-emission halves: band-mass snapshots + per-rule bench rows (2026-09-12)

Both remaining Issue 089 tasks were blocked on EVIDENCE that only future
runs accumulate — but until this landing, nothing accumulated it: neither
the band-mass snapshot writer nor the per-rule bench emission existed.
Their emission halves are now live; T4/T8 stay open only on ACCUMULATION
(time + future generations), with the clock started 2026-09-12.

**T4's half — `frontier_snapshots.jsonl`:** every `--frontier-report` run
appends the just-measured band-mass to the COMMITTED append-only
`.benchmarks/frontier_snapshots.jsonl` (the score-history law: the committed
file is the projection). State-change dedup: an unchanged store at the same
generation appends NOTHING (ts-only differences are not rows) — one row per
measurable state, not one per report run. The `generation` field is the
FRONTIER (max generation ever run in the cwd, NOT the latest lane): lanes
interleave (a v1 rerun landed between the v8 run and the first snapshot —
latest-lane semantics would have recorded a spurious 8→1→8 oscillation on
the T4 x-axis, caught live in this session), so a new row means "a new
generation EXISTS" — the boundary the correlation samples. Rows carry:
ts/sha (the store discriminator — different repos' stores share the file,
no machine paths committed), generation frontier, population/seen/
band_count/band_mass/saturated, and the T5 per-feed split. Best-effort by
law: a failed append prints one stderr line and never fails the report.
First row: gen-8 frontier, band-mass 5.6%, saturated 2 (`use_self` 172,
`nan-unsafe-sort-comparator` 133), feeds all-organic 18/18. Dedup validated
live on the immediate re-run ("unchanged since the last row — not
appended"). The ≥3-generations GO evidence clock is RUNNING; the gate
itself stays unwired until the band-mass ↔ next-generation yield
correlation is measurable on the file.

**T8's half — `rule_outcomes` on every bench row:** the α grid-fit blocker
was that `score_history.jsonl` carried AGGREGATE heal rates per run —
nothing could be replayed per candidate. Every row now carries per-rule
`fixtures/resolved/persist/broken/created/edits/shaped`, attribution by the
T7 `shaped_half_counts` law: single-target fixtures only (a ran fixture
whose `vocab_before` is exactly one lint); multi-target composites count in
the aggregate axes, never here — the count prints loud per run ("N
multi-target fixture(s) unattributed"). The `persist`/`broken` fork is the
T4 triage vocabulary: `broken` = `FixtureOutcome::parses` false (the FULL
output-ok signal — syn-parse ∧ oracle-ran ∧ compiles), `persist` = output
ok but the target still fires. `edits` is the log-length proxy the R(n)
tie-break consumes. Same additive serde law as `ood`/`strata`/`shaped`
(instrumentation, not scored; the Warm-tier `ScoreRunRow` mirror drops it
— the schema-identity test's exceptions list extended to three). First
live row (#98, v1 lane): 4 rules attributed — `deref_addrof` 1/1,
`filter_next` 1/1, `map_unwrap_or` 2 fixtures 1 resolved + 1 PERSIST
(cross-checked against the console persist map: "map_unwrap_or x1"),
`needless_return` 1/1 (edits 2); 24 multi-target skipped. The α fit itself
stays blocked until generations of per-rule rows accumulate.

**En-route (DRY):** `git_sha` + `local_iso_timestamp` (+ private
`civil_from_days`) moved UNGATED from `score_bench::report` to
`frontier_report` (the snapshot writer needs both without the bench
feature; the bench re-exports them so every `sb::report::*` call site is
stable) — the moved `civil_from_days` also carries the module-level
`cast_possible_truncation` allow it used to inherit from report.rs's
module-level block.

Validation: lib 1220/1220 @ default (+3 snapshot tests), 1452/1452 @
score_bench (+2 per-rule tests), `generation_v8_freeze` e2e green, clippy
clean on lib (both feature states) + the full `cargo-heal` bin lane; live
runs: `--frontier-report` (snapshot #1 appended + dedup) and `--score-bench`
(row #98, first `rule_outcomes`). Issue 089 T4/T8 rows updated — blocked
on accumulation, not on emission.

## Issue 089 T7 — the shaped 0.5 verdict: generation 8 + the Elo-tie / Beta-half-count feeds (2026-09-12)

The optional third-track task, landed as a full generation. "Target cleared
∧ new diags introduced" is now a FIRST-CLASS verdict — priced 0.5 (half win)
in the bench, represented as a TIE in Elo and a HALF-COUNT (+0.5 r AND +0.5
f) in the Beta feed — with its own append-only score-bench generation.

**The creation mechanism (why the shapes exist at all):** `manual_map`'s
span-preserving rewrite splices the Some-arm body VERBATIM into a BORN
closure (`match o { Some(v) => Some(f(v)), None => None }` → `o.map(|v|
f(v))`) — the closure expression did not exist pre-fix, and the born shape
fires lints OUTSIDE `FIXABLE_LINTS` (VERIFY-only / pedantic), so the bounded
fixpoint cannot dissolve them; they SURVIVE as `created`. Anti-precedent
honored: v7's `gen7-flat-nrl-format` WAS a created-chain until Plan 097 T1's
universal `*__elem` splice dissolved it — the v8 freeze confirms the splice
class creates NOTHING today (the `gen8-splice-nrl` atom measured empty and
is excluded ON RECORD, with `gen8-birth-method` (`|v| v.len()`) and
`gen8-birth-traitmethod` (`|v| v.to_string()`) — inherent/trait method calls
do not trip `redundant_closure_for_method_calls` in the bench configuration).

**Generation v8** (`specs_v8` / `generation_v8`, `--score-bench --generation
8`): 7 authored atoms, freeze-adjudicated 2026-09-12 — 4 ADMITTED (all
closure-birth: `fncall`/`pathcall` birth `redundant_closure`; `clone` births
`map_clone` — measured beat the claimed `redundant_closure_for_method_calls`,
the oracle wins both directions; `identity` (`Some(v) => Some(v)` →
`o.map(|v| v)`) births BOTH `map_identity` AND `single_option_map`, and its
freeze label carries TWO targets — `manual_map` + `needless_match`, the
outer rewrite SUBSUMING the inner fix in one pass), 3 EXCLUDED as
dissolved shapes. Freeze e2e (`tests/generation_v8_freeze.rs`, the v7
convention + two NEW fences): the INNOCENCE fence (claimed-created lints
must NOT fire pre-heal — a created lint that already fires is a persist
mislabeled as creation) and the SHAPE gate (heal must clear the target AND
leave the created set non-empty; a dissolved atom is EXCLUDED with the
reason, never silently kept — SHAPE-NEGATIVE is a first-class freeze
verdict). Always-on gates in `generations.rs`: parse/single-target,
claim-innocence coherence, cargo-less chain-truth (all admitted atoms = 1
pass), the closure-birth `.map(|` textual canary (a future `manual_map`
change that simplifies the splice reds BEFORE any oracle run), and the
freeze-hole check over BOTH record tables (`FROZEN_LABELS_V8` + the new
`V8_CREATED` measured-created record, drift-gated by the freeze re-run).

**Scoring** (`score_bench`): `FixtureOutcome::shaped_verdict()` — 1.0 clean
clear, [`SHAPED_VERDICT`] = 0.5 target-cleared-with-new-diags, 0.0 otherwise
(broken output re-scores 0 by the existing after_verdict contract);
`BenchScore { n_shaped, shaped_credit, shaped_rate, shaped_half_counts }`;
`HistoryRecord.shaped` (the additive serde law — old rows read `None`, the
Warm-tier mirror drops it, SVG/saturation ignore it). Attribution law:
half_counts credit SINGLE-TARGET fixtures only — the identity atom's
`needless_match` resolution is deliberately NOT credited (subsumed by the
outer rewrite, not fixed by its proposer — crediting it would be
mis-attribution; the single-target law prevents this by construction).
Gen-8 admission is SHAPE non-vacuity (`n_shaped ≥ 1`), deliberately NOT the
M5 heal_rate comparison — every v8 target clears by construction, so the
plain gate would refuse the generation for measuring exactly what it exists
to measure. First live run (row #97, 2026-09-12 — the citation read #98 until a
2026-09-12 static re-read of `score_history.jsonl` corrected it; #98 is the
later gen-1 v1 rerun, the one carrying the first `rule_outcomes`): 4/4
shaped, **shaped_rate 50.0% vs heal_rate 100.0%**, created 5 lints across 4 atoms, half_counts
`manual_map`×3; the run report prints the per-atom shaped table with
drift-check against `V8_CREATED`.

**The rating feeds** (the task's second half): `elo::{update_pair_scored,
EloTable::play_scored}` — the tie representation, delegating to the
katgpt-core `update_scored` f64 primitive (substrate-first: it already
existed with draw tests; ties at equal ratings are bit-exact no-ops, the
s=1/0 endpoints are bit-identical with `update_pair`, zero-sum throughout —
pinned by 4 new tests). `frontier_report::{shaped_half_rows,
render_shaped_half_rows}` — the "Beta (half-count)" fold: each shaped
verdict adds +0.5 r AND +0.5 f, p̂ drifts toward 0.5; a rule whose only
evidence is shaped sits AT the band edge (exactly 0.500, never above — the
admission-band-ceiling-by-construction property, pinned by test). Wired
into `--frontier-report` via a plain-serde read of the latest gen-8 jsonl
row (no feature dep; EMPTY-loud when v8 has never run). The LCB/admission
math stays on the store's INTEGER feed — doubling counts to fit u32 would
fake Beta concentration (Beta(1+2r+s, 1+2f+s) ≠ Beta(1+r+s/2, 1+f+s/2));
stated in the section footer, the report-first law. Live first read:
`manual_map` r 0→1.5, f 0→1.5, p̂_folded 0.500 — the store has no organic
`manual_map` row, so the fold's band-edge property is on live display.

Validation: lib 1447/1447 @ score_bench (14 new) + 1217/1217 @ default (6
new); the freeze e2e green twice (first run = the probe, second = the
drift-gated re-run); clippy 0 new findings on lib/tests @ score_bench, 0 @
default, 0 @ the full release bin lane (`score_bench,latent_retrieval,
kat_service,kat_transport,fix_verify`); the live bench + frontier runs
above. En-route: `run_bench_with` restructured under the line budget
(`RunAccum` + the extracted `warn_label_drift`); the k-ladder section now
skips gen 8 (its axis is the shape, not width/depth).

## Issue 089 T6 — the stratified score-bench read + the D-formula decline (2026-09-12)

Split verdict on the difficulty-proxy task: the closed-form D is DECLINED on
the record (both readings insufficient — mining-time D has no per-rule data
path, healing-time D measures exposure volume, not difficulty, and would
restate the n column), while the stratified-read half LANDED and paid for
itself on its first run.

The stratified read (`score_bench::{provenance_class, ProvenanceStratum,
stratify}`): every clippy-lane bench run's heal rate split by fixture
provenance class — `repo` (`repo@sha` families, organic mined), `authored`
(`authored-composite:*`), `construction` (the generated ladders — the same
word the pair vocabulary uses), `other` (fail-closed catch-all). The fixture
join REUSES the ood mining join (`ood_set_family`): v6's frozen bench rows
 carry `construction:gen6:…` metadata, but their ids live on the raw span
table with the real `repo@sha` provenance — the strata classify v6 as REPO
(guarded by test; without the join the v6 ladder would have read
construction). Printed after every clippy-lane bench + persisted additively
on the jsonl row (`HistoryRecord.strata`, the ood law: instrumentation, not
scored — SVG + saturation ignore it, the Warm-tier `ScoreRunRow` mirror
drops it, old rows deserialize as `None`). The gen-4/5 lanes write no
strata (single-class ladders — a one-row stratum would restate the
aggregate).

**First measurements (live, 2026-09-12):** v1's 96.2% aggregate is
authored-dominated — authored 98.0% (27 fixtures, 51 findings, 50 resolved)
vs repo 50.0% (8 fixtures but only 2 findings — 6 are clean/decline spans).
The v6 all-repo ladder reads 95.2% (17 spans, 21 findings, 20 resolved).
So the healer holds ~95% on real-mined material at depth (v6) and ~98% on
authored composites; v1's tiny repo surface is sample-noise territory (2
findings). The per-CLASS trend across generations is now measurable — every
bench row is generation-tagged, which is exactly what the T5 outcome-feed
per-batch blocker lacks.

En-route fix the lane required: the score_bench BIN lane had been E0631-
broken (`v5_is_removal_rule` passed as a bare fn filter over `String`
items) — a lane nobody compiles (the Issue-513 class); fixed with the
closure wrap. 4 new tests: classifier families, the v6 mining-join guard,
stratify arithmetic/order/edges (clean/excluded/orphan), the additive-serde
law (pre-T6 rows deserialize `strata: None`; a stratified row round-trips).
Lanes: lib 1433/1433 at `score_bench`, 1211/1211 default; clippy `-D
warnings` clean at `score_bench` and `score_bench,latent_retrieval`; the
full bin lane (`latent_retrieval,rust_perf,clippy_lints,kat_service,
score_bench,rustc_errors`) green; two live bench runs appended (v1 gen-1
#77, v6) with strata on the rows.

## Issue 089 T5 — provenance weighting: the priced pair namespace + the enforced rating fence (2026-09-12)

The learnability-frontier instrument gained its provenance axis (the
TaskPilot/FrogNano +2.06-organic-vs-+0.00-injected effect, made measurable at
the feeds that exist today). Four coordinated halves:

1. **The vocabulary + weights** (`src/frontier_pairs.rs`, with the pair
   types): `PROVENANCE_ORGANIC` (feature-churn mined — no miner ships yet,
   the vocabulary lands first so the day one lands its rows are priced
   correctly), `SYNTHETIC_PAIR_WEIGHT = 0.05` (ε — near-zero per the paper,
   NOT zero: construction truth is fence-verified and bootstraps pools with
   no organic evidence), `provenance_weight()` (unknown tags → 0.0
   fail-closed). The module is UNGATED since this landing — the type half
   was always dependency-free (the 09-10 E0433 fix already cfg'd the corpus
   half); the any() gate was the transitional shape. `frontier_report`
   (ungated) consumes the vocabulary from every feature build.
2. **The fence, load-bearing** (`traj_store::rate_pairs`): only
   `construction` rows feed the BT fit. The PairRowDoc contract has said
   "any other provenance must never feed a rating" since Issue 072 —
   vacuous while every row was construction. Now enforced: non-construction
   rows are excluded + counted loud. The semantic: organic rows are
   curriculum-priced 1.0 precisely because they are OBSERVED, and observed
   wins are never rating evidence (the healer must not grade its own
   behavior — the non-circularity law).
3. **The `--frontier-report` tail**: the pair-namespace provenance section
   (count/share/weight/weighted per class; EMPTY-loud when no pair rows —
   both live stores honestly show 0) + the outcome-feed split
   (`{code}:{strategy}` keys = synthetic/generated-case vs every other key
   = organic/real-repo; band-mass measured PER FEED — what T4's admission
   gate will consume; a band living only in the synthetic feed is a
   curriculum artifact). Live measurements: riir-ai 21 organic/0 synthetic
   (0.0% band-mass), this repo 18 organic/0 synthetic (5.6%) — the band
   mass is all-organic on both stores.
4. **The honest blocker, stated in the report tail**: per-BATCH causal yield
   (the paper's effect size) needs generation/batch tags on rows the store
   deliberately does not carry (construction rows are byte-deterministic,
   no timestamps; learn rows are cumulative counters) — the T2 snapshot
   precedent. The split is per-FEED health, never presented as the
   curriculum effect.

Tests: 6 new lib tests (weights fail-closed pricing, the rate_pairs fence
with a hand-put organic row, summary canonical-order + share/weight
arithmetic, split attribution by key shape incl. measured-empty feeds,
render needles incl. EMPTY-loud). Lanes: lib 1211/1211 default + 1333/1333
latent_retrieval; clippy `-D warnings` clean at default, no-default,
latent+rustc, all-features (lib); the bin lane
(`latent_retrieval,rust_perf,clippy_lints,kat_service`) green; frontier PoC
A/B 4/4 (the required-features catch from T3 applied).

## Issue 088 resolved — the kat service-plane spin-out: every task + fleet follow-up landed, file removed at close (2026-09-10 → 2026-09-12)

The KAT service plane (protocol tier + client plane + transports, 25 modules)
became the private sibling leaf `gist-rs/riir-kat` (single crate; riir-auth
optional is its only sibling dep; zero katgpt-core). riir-clippy is now a
consumer: `riir-kat = { path = "../riir-kat", default-features = false }`
(non-optional), root `lib.rs` re-exports every moved module at its historical
path, and the three healer-coupled keepers stayed root (`kat_promotion`,
`kat_sync_payload`, `kat_pending_estimator`). The redact-floor pin went
cross-crate as `tests/kat_issue088_cross_repo_pins.rs`.

Landing trail: riir-kat first commit `90a990e` (2026-09-10) + the riir-clippy
consumer-rewiring phase-2 commit; full `ci_feature_guard` GREEN 2026-09-10
(GUARD_EXIT=0) after the sweep surfaced + fixed 8 latent rot classes
(`412f2323` 8-combo E0433, `c127a819` 23 orphaned kat_* files — the phase-2
commit message's deletion claim was wrong, caught by the owner's grep,
`dd9d0f7c` 5a clippy + too_many_lines, `5c5c47df` corpus-split walk needle,
`b7f25c02` dist Phase-1 pin, `03f0ffcc` kernel/slice-b vacuity, `44ac9cc1`
P12 scratch manifest, `f9cef9cc` --mine explainer pin).

Fleet follow-ups — all closed 2026-09-11: katgpt-rs `repo_set.txt` + AGENTS
count 17→18 (`ff5ab9f9`); the owner-greenlit dapps/dao protocol-tier adoption
(riir-kat `3482a3c` lifted the two-sided composers into the feature-free
protocol tier with their goldens; riir-dapps `f2498d2` Issue 082 mirrors
retired; riir-dao `7823daa` units twin retired + live grant cross-pin) —
boundary contract green at 18 repos / 273 edges; the SYNC-payload mirror
deliberately stays dual-pinned (canonical = this repo's healer-coupled
`kat_sync_payload` keeper — the 088 non-goal); the 4090 box cloned
`E:\git\riir-kat` @ `a80985e`. Non-goals held: no public publishing, no
API change at the riir-clippy surface. Issue file removed at close per the
noise-reduction rule (this entry + BOUNDARY.md prose + riir-kat's own
HISTORY are the record).

## Issue 090 resolved — latent-only test lane: 17 domain-panic tests gated on the calibration state (2026-09-11, 23e051bc)

The latent-only lane (`cargo test --lib --no-default-features --features
latent_retrieval`) compiled three draft test modules whose bodies need the
matcher AND the clippy+perf fixture state: 17 tests panicked at
`LatentMatcher: no domains enabled`. Found by the 2026-09-10 each-feature
rot-gate sweep; compile half fixed same day, runtime half tracked here.

The fix (23e051bc): all three modules (`draft::latent_matcher::tests`,
`draft::headroom_probe::tests`, `draft::retrieval_eval::tests`) now gate on
`all(test, latent_retrieval, clippy_lints, rust_perf)` — the exact predicate
`retrieval_eval`'s `cross_domain_agreement_rate_report` already carried
in-file, and the state the fixture set + regression floors are calibrated
on (blended floors sit ABOVE clippy-only precision; `per_domain` rows are
positional 0=clippy/1=perf). The issue's sketched `any(single-domain)` gate
would have been WRONG: the tests assert `DomainId::Clippy` rows and the
perf seed (`overlay_replaces_the_covered_domain_and_keeps_the_rest`), so a
`latent+sec`-style lane would still fail them post-gate — the matcher
builds with sec, then the clippy-row asserts panic. clippy+perf is what
the tests actually need, which was the issue's own governing phrase.

Validated: latent-only 232p/17f → 230p/0f (two passing tests also drop
from THIS lane — `mutate_shape_keeps_embedded_tokens` (pure) and
`router_spectral_poc_report` (router report) — both still run in every
clippy+perf+latent lane); calibration lane (default+latent) 1301p/0f with
24 of the modules' tests present by name (5+6+13; `kernel_mutation_*`
stays out until kernel_heal joins); clippy clean at both feature sets.
Standing caveat, out of scope: the latent-only lane carries ~15 pre-existing
domain-less dead-code warnings (unused `index_domain` macro, unused
`HealDomain` imports, `mut`-not-needed on `router_domains`) — the lane has
no clippy guard, which is the same premise this issue existed to close.
No guard-layer count moves: the guard's lanes (5b default, 5c all-features)
always satisfy the new gates.

Issue file removed per the noise-reduction rule; this section is the record.

## Issue 085 — the `dist` domain: the promotion-evidence narrative (2026-09-09 → 2026-09-10)

Moved out of the issue header 2026-09-12 (noise-reduction rule): the issue
keeps a crisp status + the single remaining gate; the changelog lives here.

**Phase 1 landed 2026-09-09** (`79dcf9b`) — the 6th retrieval domain, a
hand-written section-aware `Cargo.toml` scanner (no `syn`, no `toml` crate),
suggest-only (`FIXABLE_DIST = []`), opt-in feature `dist`. Premise: this
tool's own shipped binary rode plain `--release` through v0.1.4 (11.35 MiB
stripped) where the knob set measures 4.34 MiB (-61%) — found by the OWNER,
not by any agent session. Initially parked NO-GO in the post-mining intake
(`.distill/001_mining_queue_snapshot.md`, 2026-09-09, kept and marked
SUPERSEDED); the owner's pushback corrected the reasoning — the "no hermetic
oracle" blocker was wrong by the repo's own `rust_perf` precedent (fixes
verify by `cargo test` re-run; the GAIN is proven ONCE at class level).

**2026-09-10 (zero-contention diagnosis)** — both build blockers named in
the T-list were REPAIRED at HEAD: (1) the E0631 `v5_is_removal_rule`
signature mismatch was `ce74bac`'s healer `redundant_closure` point-free
rewrite of a load-bearing `&String→&str` coercion, compiled to nothing at
default features (`#[cfg(feature = "score_bench")]`) — restored to the
closure form by Batch 140 `d2d2388`; (2) the Batch 138/139 kernel_verify
`.parses` all-features conflict was fixed by `91493a1` (cfg-adaptive
`verdict_passes` accessor in `corpus/tests.rs`).

**2026-09-10 (promotion evidence — Bench 082)** — both owed items ran at
zero contention, isolated `CARGO_TARGET_DIR=/tmp/plan085`, binary built
`--features score_bench,dist,kat_service`. (1) Score-bench **dist-ON run
#92: FLAT** — byte-identical to the dist-off tail (heal 96.2 / created 1.9 /
parse 100 / decline 100 / edits 64 / vocab 38; OOD axis bit-identical,
artifact `faa021c8…`); `FIXABLE_DIST = []` keeps the proposer vocabulary and
the OOD statistic unmoved by construction — the FLAT is mechanical, not
lucky. (2) Condition-(4) consumer scans (`--suggest --domain dist`, zero
writes): true positives on `riir-game-sdk/crates/riir-asset-converter` (3
cites, 2 explicit bins) and `riir-deployer/crates/riir-deployer` (3 cites, 6
bins); negative controls HELD on `katgpt-rs`, `riir-viewbridge`, this repo.
Two static-cite disagreements exposed the scanner's blind spots: `riir-gm-tool`
a MISSED true positive (bin autodiscovered from `src/main.rs`, no `[[bin]]`
section — **B1**) and `riir-chain/crates/riir-chaind` FIRED 3 cites where a
negative was expected (a member with `[[bin]]` cannot see the root's
strip+thin+1 — **B2**).

**2026-09-10 (B1 + B2 landed — `759e209d`, Bench 082 §6 addendum)** — both
blind spots CLOSED in `src/domains/dist/context.rs`: `DistContext::derive`
(one bounded ancestor walk, ≤32 levels, resolved once per manifest) exposing
`autodiscovered_bin` (B1: `src/main.rs` or directly-nested `src/bin/*.rs`,
gated on `[package]` via the shared `ships_bin` helper) and
`root_profile_knobs` (B2: strip/lto/panic presence from the nearest
`[workspace]` ancestor, consumed by the shared `effective_knobs`
self-or-root resolver); `member-profile-ignored` UNCHANGED; the panic rule
keeps its daemon caveat. Corpus/routing bytes UNTOUCHED (detector triggers
only) → no score-bench re-run owed. Scan table re-run (same repos, suggest
lane, zero writes, `/tmp/plan085b`): gm-tool 0 → 3 (B1 closed); chaind 3 → 1
(rules 1+2 silent; the panic cite honest); deployer member 3 → 2; viewbridge
xtask 0 → 1 (NEW marginal — B1 exposes the real bin target, recorded
honestly); every stable negative held at 0. `dist_gates` 7 → **13/13
green**; clippy `--features dist --all-targets` clean on all touched files.
Incidental repair: `heal.rs`'s `dist_scan_context` (deleted) carried a stray
stacked `#[cfg(feature = "docker")]` — it only compiled when BOTH features
were on. Honest caveats (Bench 082 §6.4, now also on the domain doc page):
`autobins = false` unread, `src/bin/<dir>/main.rs` + out-of-dir `[[bin]]
path` still invisible, knobs name-agnostic, a member's own knobs still
satisfy rules 1–3.

**Pre-staged cites (2026-09-10 static pass, since CONFIRMED/CORRECTED by the
scanner)** — the static manifest read predicted `riir-game-sdk` first (root
declares NO `[profile.release]`, ships two real bin crates) and
`riir-deployer` as the weaker cite; the real scan confirmed both, MISSED
gm-tool (B1) and over-fired on chaind (B2). Recorded because it is the
precedent: a static manifest read is not the tool's verdict.

**Post-Phase-1 corpus growth:** Batch 143 added a 5th rule,
`large-api-dependency-mostly-unused-hint` (BuildTime family,
`-Zhint-mostly-unused` measure-first advice) — the issue's Phase-1 table
listed 4 until the 2026-09-12 status-closure pass folded it in.

## The domain swap — the client half + the config surface, post-flip state (2026-09-10, riir-dapps Plans 023+024)

The riir-dapps-side record carries the server half (the export/import move,
the flip, `4da667f`'s setup/teardown fixes). This is the client repo's half.

- **The flip HAPPENED 2026-09-10**: the apex `https://ai.gist.rs` (= the
  `DEFAULT_SERVICE_URL` const, never moved) IS MAINNET now — fresh ledger;
  `devnet.ai.gist.rs` is its own worker hosting the fleet's MOVED ledger
  (BLAKE3-identical, burn-acks + watermarks carried over). The fallback pair
  (default→devnet ONLY) is now load-bearing in the other direction: a DEV
  client can never fail over onto mainnet, and the default IS mainnet.
- **Client config surface** (`cf236f1a`, Plan 023 S3): `cargo heal config
  set --url <alias|url>` / `config get` — pinned alias map
  devnet/testnet/mainnet, LOUD set, source-revealing get,
  `RIIR_HEAL_KAT_SERVICE_URL` > config > default, per-command re-pointing
  via `service_url_for_command()` while the ambient lease phase stays
  env-only. Fleet state post-swap: M3 + 4090 both `config set --url devnet`
  (devnet round-trips green — sync, refill, burn-ack v3, the soft gate).
- **AGENTS.md sweep** (`60cc19a9` pre-landed the S2/S3 rows; the flip-state
  halves landed after the swap): the service-environments bullet now
  describes the post-flip world ("the apex IS mainnet"), the fallback-chain
  bullet names the new load-bearing direction, the fleet-state line records
  devnet.ai.gist.rs as the moved-ledger host.

## Issue 081 resolved — burn wire unit mismatch: end-to-end closed, rescale STOOD DOWN (2026-09-10)

Resolved-record for the now-removed issue file (removed per the
noise-reduction rule; full narrative in git history,
`git log -- .issues/081_burn_wire_unit_mismatch.md`). The client half
was already recorded here; this entry closes the whole arc.

- **Incident** (2026-09-09, owner's live e2e): the burn client sent its
  micro-KAT book value unconverted in the whole-KAT `burn_kat_total`
  wire field — the first settlement clamp-wiped the account's 100 KAT
  grant and poisoned the server watermark at 2,323,993.
- **Client fix `e9eca71d`**: `wire_kat_total = charged_micro /
  1_000_000` at the single sync boundary + the sub-KAT accumulation
  rule + the cross-repo unit golden `3dd14dd` (this repo's
  `kat_burn_client.rs::cross_repo_unit_golden_book_micro_to_wire_to_debit`
  mirrored by dapps `tests/kat_burn.rs` — the symmetric-mock /
  asymmetric-unit gap is now a drift tripwire on both sides).
- **Server migration surface**: dapps `791fed5` —
  `POST /admin/burn-watermark/rescale` (bearer + typed ack + CAS +
  whole-scale guard) + `tests/burn_wm_rescale.rs` (7 tests).
- **Execution STOOD DOWN (dapps HISTORY 2026-09-10, runbook branch 2)**:
  the CAS curl answered `200 idempotent_no_op:true,
  before_kat_total:14` — the watermark row was already WHOLE-SCALE at
  14, the poison healed by the shipped client fix itself (a later
  accepted push re-answered whole-scale and advanced the ack), and the
  clamp-wiped grant was already refilled (99,999,998 KAT, the faucet
  arc). The guard refused without touching the row — the designed
  safety. Replay-is-the-read satisfied trivially.
- **Supersessions**: the fleet-wide per-account pass (item 2) stays
  conditional on the deployment growing past the owner's account; the
  server-truth `contributed` field (Issue 080, fleet v3 live) removed
  the residual gate surface this incident's ack-poisoning used to
  threaten. Proposal 005 prose carried the dated DESIGN CHANGE
  annotations (item 3, done 2026-09-09). Mirror issue file removed in
  riir-dapps at `45c8191`.
- **Lesson** (recorded at the fix, still standing): a money unit has
  ONE name per side, and a contract change must re-pin its e2e the same
  day, not just its unit goldens (Issue 087's finding generalized
  this).

## Issue 087 resolved — kat_burn_stack e2e: stale harness units, not the account plane (2026-09-10)

Resolved-record for the now-removed issue file (removed per the
noise-reduction rule; full narrative in git history,
`git log -- .issues/087_kat_burn_stack_autoreport_skipped.md`).

- **Finding** (surfaced during Issue 080 validation): `cargo test --test
  kat_burn_stack` failed 3 tests at clean HEAD `fcc478c` — the burn
  auto-report returned `Skipped` before any dial, mock server received
  zero requests. Reproduced in an isolated worktree, so committed-state,
  not WIP.
- **Root cause NOT the account plane** (the issue's own suspect was
  refuted): commit `e9eca71d` (2026-09-09 11:21, Issue 081's client fix —
  the whole-KAT wire conversion at the sync boundary) changed
  `run_burn_sync_at`'s skip predicate from the raw micro book to
  `wire_kat_total(charged_kat) <= ack`, while this harness (`f3a7cf28`,
  landed ~11 h EARLIER) bills only 11 micro-KAT per run —
  `wire_kat_total(11) = 0 ≤ ack 0` is a LEGITIMATE `Skipped` before any
  dial. The conversion itself is correct and cross-repo-pinned; the
  lesson generalizes Issue 081's: a contract change must re-pin its e2e
  the same day, not just its unit goldens.
- **Fix `fbedf6d2` (harness-side, no assertions relaxed):** the fixture
  now bills exactly 1_000_000 micro-KAT (1 whole KAT) per run via a
  comment-only `a.rs` (AstChunker's whole-file single-chunk fallback;
  billing is raw-byte parse-independent per the T0.5 contract) with an
  in-test `count_word_tokens == 1_000_000` self-assert; the three stale
  assertions re-pinned to the WIRE unit (`sent == wire_kat_total(y)`,
  `ack.burn_kat_total == wire_kat_total(charged)`, exact stacked-line
  format; signature over the wire number). Coverage is STRICTER: exact
  wire totals 1→2 KAT, real dials proven.
- Validated: 3/3 at the required-features set AND at `--all-features`
  (the ci_feature_guard Layer 5c configuration); `kat_soft_gate_cli`
  13/13; clippy clean on the touched file. Recorded follow-up (coordinator
  call, not owed): a cheap dedicated `--test kat_burn_stack` row in a fast
  local gate would shorten the next between-windows red stretch (Layer
  5c/5d already cover it; the ~1-day red window was a manual-run gap).

## Issue 080 resolved — the sticky-grace marker retired, server truth only (2026-09-10)

Resolved-record for `.issues/080_sticky_grace_server_truth.md`
(now removed per the noise-reduction rule — the closure block's one
load-bearing fact is folded into this record; full narrative in git
history, `git log -- .issues/080_sticky_grace_server_truth.md`). The
two-stage arc is complete: T3
landed the truth-wins swap (`587132b`, 2026-09-09) and the DATED REMOVAL
landed once the fleet was v3 (dapps Issue 027 live; owner account's
contribution fact verified server-side via the live v3 burn ack,
`"contributed": true` + `first_commitment_hex`). Commit
`143c5a8`:

- `src/kat_contribution.rs` deleted (+ its `pub mod` decl) — the locally
  forgeable `contributed_once.json` input is gone entirely: nothing reads
  it, nothing writes it, a pre-removal leftover has ZERO effect. The sync
  success path no longer marks anything.
- Gate predicate: `sticky_grace(ack) = ack.contributed == Some(true)`
  (server truth ONLY, both evaluation sites in `src/bin/heal.rs`).
  Anti-brick preserved where it belongs — at the ack: corrupt/missing →
  empty → `NeverSynced` → warn-only.
- Tests re-pinned (`tests/kat_soft_gate_cli.rs`): the forge-closure
  regression kept; three arms INVERTED to prove the marker dead (`None` +
  marker + depleted → REFUSED; v2 ack file + marker → REFUSED; marker
  alone → REFUSED) and the never-synced + leftover-marker arm pins the
  permissive direction. Header truth table rewritten.
- Docs: AGENTS.md soft-gate bullet (the marker sentence rewritten —
  transitional → retired), README soft-gate section,
  `kat_soft_gate.rs`/`kat_burn_client.rs` docs, Cargo.toml test-row
  comment. Transition edge (pre-fleet contributors: fact predates the v3
  server + depleted + quiet outbox): mitigation (1) ACCEPTED — a
  historical contributor queues any one fix (trajectories accumulate
  naturally) and syncs; the fact records; the next drain lifts. Population
  = the owner's own fleet; revisit only if the deployment grows.

## Issues 077 + 082 resolved — margin-weighted mining + the OOD axis (2026-09-09)

Resolved-record for the two now-removed issue files (full narratives live in
`.benchmarks/080_margin_mining_ab.md`, `.benchmarks/081_ood_fresh_family_convergent_validity.md`,
and `.docs/08_benchmarks/entropy_gap_ood_axis.md`; removed files: git history).

- **Issue 077** (margin-weighted rule mining + entropy-gap OOD axis,
  arXiv:2604.26841; consumes katgpt-rs Issue 740 / `katgpt-core::regime_probe`):
  T1+T4 `1994a8a` (score_bench forwards the katgpt-core regime probe — zero
  local entropy arithmetic; the OOD row is always-on + the additive `ood`
  JSONL field; first measured row run #84: gap −0.729 nats, d −1.11 →
  anomalous_negative), T2 `2e3ce9a` (`draft::margin_weight` — the e^{−2M}
  sigmoid-shaped score-delta law, never softmax; concentration premise
  MEASURED on labeled material: the lowest-margin half carries 91.4% of total
  mass), T3 `095686a` (`margin_mining` opt-in + the pre-registered `--ab`
  protocol; verdict **DEMOTE** — tie at every budget, 40 compiled raw carves
  100% lint-clean: retrieval-similarity margins do not select lint-bearing
  spans; miner default stays count-desc, no generation frozen), T5 `80e8667`
  (per-set provenance families — #84's anomalous_negative is a set-COMPOSITION
  fact: v1 = 4 families mostly authored, gen6 = 5 real).
- **Issue 082** (OOD fresh-family pairing — the convergent-validity
  experiment that tested 077's composition attribution): `b3636b5` —
  composition held constant (real vs real), provenance held disjoint (gen6
  reference vs `riir-neuron-db`@`2f30800` / `riir-dapps`@`c4c6ffe` carves);
  verdict **R1 — CONVERGENT VALIDITY DEMONSTRATED** (gap −0.7608, d −1.026 on
  BOTH fresh families) — the composition attribution was tested and held.
  Honest caveat in the record: the fresh signal is all-or-nothing (0/48 fresh
  carves fire ANY proposer rule); promotion is NOT on this axis —
  `regime_probe` stays opt-in (no default-path consumer exists).
- **Issue 080 stays OPEN** (IN PROGRESS by design): client half landed
  (`587132b`, the truth-wins gate); the dated removal of `kat_contribution`'s
  legacy marker read waits on fleet v3 (server half = riir-dapps Issue 027).

## Issues 083 + 084 resolved — the green-zero class and the telemetry-key consumer (2026-09-09)

Resolved-record for the two now-removed issue files (removed per the
noise-reduction rule; full narratives in git history,
`git log -- .issues/083_cfg_gated_tests_report_a_green_zero.md
.issues/084_telemetry_json_key_rename_consumer.md`).

- **Issue 083** (16 `#![cfg]`-gated test targets had no `[[test]]` row, so
  naming one printed `ok. 0 passed`): fix `2c7be78` — all 16 rows added,
  each mirroring its file's `#![cfg]` header verbatim, and verified in BOTH
  directions: sufficiency via 13 exact feature-set builds (13/13, 0 fails,
  never a superset), population unchanged (102 targets / 67 gated;
  `SILENT-NOW` 16 → 0, `w/ req-f` 35 → 51). `load-bear` was 0 and stays 0.
  Workspace standing after game-sdk 028 (`2380fc7`) and dao 003 (`f3a0578`):
  177 SILENT-NOW over six repos, owned by those repos; katgpt-rs's 42 is the
  pinned ratchet. The classifier-vocabulary caveat that fell out of this
  work is katgpt-rs `.issues/741` (still open, that repo).
- **Issue 084** — filed as 082 (`07dece1`), **renumbered 084 before removal**:
  the OOD issue had already consumed 082 that morning (removed `f714fea`),
  so the telemetry filing was a silent reuse — the standing fix is
  renumber + citation rewrite, applied here. (`git log --follow` resolves
  the rename; consumers citing "clippy `.issues/082`" were rewritten to 084
  in riir-dapps `.issues/033` + riir-dao `.issues/003`.) Substance: fix
  `97d878e` — `kat_wetrun::parse_supply` reads `*_micro` first and falls
  back to `*_kat` (the dao-soak `u_micro(new, legacy)` pattern), struct
  fields renamed to `*_micro` byte-neutrally (offsets + `ENCODED_LEN = 40`
  untouched), unit prose corrected, the G-WET supply line rendered through
  `render_kat`, and the legacy arm pinned by a dedicated unit test.
  Verified: `--lib kat_wetrun` 12/12, `--test kat_wetrun_gwet` 11/11,
  clippy clean at the five-feature set. This closed the consumer interlock
  on riir-dapps `.issues/033`'s key-drop task (deploy-gated there).

## Plan 117 — `cargo heal --refill`, the dev/test faucet refill (2026-09-09)

Plan: `.plans/117_kat_refill_cli_devnet_faucet.md`. The depleted-account
ergonomics gap the 075/079 arc left open: the gate's refusal said "top up"
but the only CLI surface was `faucet --request` (opens a row, tells you to
come back) and the only immediate credit was the once-per-key welcome grant.
`--refill` composes the shipped primitives and adds the one missing piece —
the soft-gate input refresh, so a credited account's gate lifts from SERVER
truth without waiting for the next burn push.

Design verdicts worth keeping:

- **Not a new money surface.** Every wire byte is shipped: the faucet POST
  (golden-pinned both sides since Plan 066), the wetrun's bearer crank POST,
  the public balance read. The transport COMPOSES `HttpPayreqTransport` +
  `HttpWetrunTransport` — zero HTTP re-derived (`src/kat_refill.rs`).
- **The client adds no host gate, deliberately.** The wetrun's
  `classify_service_url` names the production default `heal.gist.rs`
  Unclassifiable — and that host IS the devnet front, so a client-side
  admission rule would break the command's primary use case while adding
  nothing: the faucet's gate is the DEPLOYMENT's cap config (a zero-cap —
  production — deployment has no faucet at all), and the crank's
  authorization is the bearer token itself. Prod safety is structural:
  prod never distributes a crank token and never enables a faucet cap.
- **Server truth wins on the gate input** (the issue-080 lesson applied to
  the balance leg): `record_server_balance` writes whatever the read
  answered, including a depleted value — a refill that reads 0 GATES the
  machine honestly instead of preserving stale optimism.
- **Exit 0 iff the final synced balance is positive.** A "daily cap used"
  refusal with fuel already in the account is a SUCCESS — the goal was
  fuel. The four ledger refusals still surface verbatim (the payreq
  refusal-body rule rides the composed transport).
- **The crank leg is fail-soft.** A refused/failed crank is a rendered line
  (`skipped_treasury` included — the Issue-010/025 diagnosis, surfaced not
  hidden); the row still settles at the next hourly alarm.

Tests: 7 unit (`kat_refill::tests` — ack refresh purity, exit table, happy
path, refusal-continues, pinned-zero, poll early-break, copy rule) + 4 CLI
through the real binary against a std-TCP mock (`tests/kat_refill_cli.rs`,
the `kat_burn_stack` shape — no-account refusal, live happy path with ack
refresh + bearer-on-the-wire assertion, the gate naming the refill path,
the `--refill-wait` guard). Gates: G1 zero new wire bytes + verbatim
refusals + server-truth-only refresh; G2 bounded poll (default one read,
interval 5 s, monotone budget); G3 all 13 `kat_soft_gate_cli` rows + 3
usage-parity rows green with the refusal copy updated deliberately; G4 n/a
(one-shot CLI). No new feature flag — rides the shipped release set
(`kat_transport` since `v0.1.2`, `56bd9bf`).

Blocked upstream (unchanged by this landing): devnet treasury funding ≥ the
grant floor + riir-dapps Issue 025 (units mismatch owner call). Until then
`--refill` works and the crank report's `skipped_treasury` is the honest
answer.

## Both shell gates could have reported a PASS on an abort (fixed 2026-09-07)

`scripts/ci_feature_guard.sh` (what `rust.yml` runs) and
`scripts/ops_nightly.sh` (the launchd nightly) each had `set -euo pipefail`
plus a cleanup trap and no completion sentinel. Measured on macOS
`/bin/bash` 3.2.57 — and ONLY there (katgpt-rs Issue 735): bash 4.4 through
5.3, dash and busybox ash all preserve the status. When 3.2 aborts on an
**unbound expansion** (`set -u`) or an **`eval` syntax
error** it enters the EXIT trap with `$?` **already 0** — so a trap whose last
command succeeds (`rm -f "$RUNLOG"`, `release_lock`) makes the abort exit
**0**. Everything after it silently does not run and the caller reads a pass.
`trap 'rc=$?; cleanup; exit $rc'` does not repair it: the rc it saves is
itself 0. Only a **completion sentinel** — a flag set on the script's own last
line, checked by the handler — catches it.

This is the same class as the note at `ops_nightly.sh:465` ("the silent exit 0
that hid this for a day"), one level up: there a *step* failed silently, here
the *whole script* could.

Both repaired, each verified against its REAL extracted cleanup preamble:
unbound abort → 1, deliberate verdict failure → 1 unchanged (`exit 1` for the
guard, `exit 2` DEGRADED preserved for the nightly), clean completion → 0.
`acquire_lock || exit 0` is untouched — it sits before the trap.

Not hypothetical: seal-remake's `ci_feature_guard.sh` could not fail past its
layer 13 for months for exactly this reason (seal-remake `26a18191`).
Workspace picture + the detector:
`katgpt-rs/scripts/trap_exit_launder_audit.py`, katgpt-rs Issue 734.
`shellcheck` does **not** find this — pointed at a script carrying the live
defect it reports one `SC2001`, and with `-o all` its only remark on the fatal
line is `SC2250` (brace style).

## Batch-mining batch records + Role/domain/pipeline full record (was AGENTS.md lines 23–3043)

Contains: the full Role section (per-domain corpus narratives — clippy_lints
Batches 89–91, rust_perf Batches 48–112, kernel_opt Batches 1–117), the shared
L0–L4 pipeline, Plan 005 T3.5/T3.5.4/T3.5.5, resolved issues 014–040 + 036/037,
**Batch-mining batch records (Batches 48–117)**, the **heal-sweep campaign
records (slices per repo: katgpt-rs, riir-train, riir-neuron-db, riir-chain,
riir-ai)**, and the **score-bench run history** (runs #1–#3 + the score-bench
discipline records).

## Role

**Multi-domain modelless code healer** — a pluggable framework behind the
`HealDomain` trait (Plan 001) that heals code across multiple domains using a
drafter+pruner+corpus pattern. This is a **developer tooling repo**, NOT a
game concern, NOT a training repo, NOT a chain repo.

Five retrieval domains ship (Plan 001 + Plans 018/030; `rustc_errors` is a
6th, driver-based rather than retrieval):
1. **`clippy_lints`** (default-on) — heals `cargo clippy` warnings. The
   original domain; **49 corpus lints** (Batch 91, 2026-09-05: the P9
   clippy-1.98 new-lint batch — +`byte_char_slices` FIXABLE,
   +`manual_slice_fill` FIXABLE, +`manual_checked_ops` VERIFY,
   +`chunks_exact_to_as_chunks` VERIFY [MSRV floor: std 1.88, documented];
   all four suggestion-only in real clippy — `--fix` applied 0/17 on the
   dao run; Batch 90: +`items_after_statements`
   FIXABLE, +`manual_let_else` FIXABLE, `option_if_let_else` REFUTED — the
   rewrite output re-enters `single_match_else`'s territory → fixpoint
   ping-pong, `.plans/051`; Batch 89: +`use_self` FIXABLE,
   +`suboptimal_flops` VERIFY; `missing_const_for_fn` proposed then KILLED —
   `.plans/050`) across style/complexity/perf/pedantic/nursery; 38 `--fix`
   eligible + 8 `--verify` tier + 3 self_evolve-only.
2. **`rust_perf`** (default-on since 2026-08-13, Phase 2) — heals rust perf
   issues. **101 rules** (Batch 112 ->101 layout — the nnethercote byval-move memcpy-threshold arch-regime rule; Batch 109 ->100 complexity — the post-mining P10 measurement-honesty rule; Batch 101 ->99 dispatch; Batch 99 ->98 dispatch; Batch 94 ->97 io; Batch 91 ->96; Batch 86 ->94) distilled from the `rust-optimize` skill checklist +
   nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs
   profiling guide (Research 032) + online-search distills (Batches 48/53/61)
   + the thermite external distill (Batch 59) + the oxc article distill
   (Batch 61) (complexity,
   allocation, layout, concurrency, simd, arithmetic, iterators, dispatch,
   caching, io, bevy, wasm). The **perf-bench
   Pareto validator** (Bench 013 promotion gate + Bench 015 Phase 4
   finalization + Bench 016 Issue 012 expansion + Bench 017 Issue 013 data-structure-choice
   expansion + Bench 020 Issue 014 alloc-axis expansion, `rust_perf_bench` feature)
   is the promotion gate — GOAT G1+G5+G-det PASS (the alloc-count axis: exact
   integers, bit-identical across 3 repeat runs on 52/52 benchable rules).
   34 of 91 rules carry `context_notes` annotations recording the
   bench finding (10 verdict-class — Bench 015/016: 2 ParetoDominated + 4
   regime-fragile/Mixed in the default config; Batch 48: `filter-count-to-any`
   ParetoDominated + `iter-copied-deref-map` Mixed; Batch 53 / Bench 038:
   `powi-vs-powf` all-Tie (LLVM lowers powf(2.0) ≡ powi(2) in release — the
   folk wisdom only holds for debug profiles / exponents ≥ 3) +
   `soa-hot-field` latency-bound (a serial f32 add-reduction hides the layout
   win; SoA pays on throughput-bound field consumers); 2 metric-mismatch-class — Issue 014:
   `tokio-guard-across-await` + `bounded-channel-backpressure`, NotBenchable
   because the cost metric is contention latency / queue depth, not
   wall-clock — consumers should treat annotated rules as
   context-dependent).
3. **`kernel_opt`** (opt-in, Phase 3) — heals GPU kernel optimization. 552 rules
   (Batch 119 ->552 ss16118/low-latency-nccl near-SoL collectives distill; Batch 118 ->550 antirez/ds4 PR #991 Qwen3.8-Flash-Next distill; Batch 117 ->544 antirez/ds4 Metal tensor-prefill delta distill; Batch 116 +addendum ->539 PrismML fork ternary + verify-batch distill; Batch 115 +addendum ->532 qwen38-mtp diff-history walk; Batch 113 ->522 llama.cpp v0.4.0 delta; Batch 114 ->518 omlx ane/gdn prefill; Batch 111 ->516 remainder+rider sweep; Batch 110 ->505 dispatch; Batch 108 ->494; Batch 107 ->490; Batch 106 ->482; Batch 105 ->477; Batch 104 ->466; Batch 103 ->464; Batch 102 ->459)
   (Batch 119: 2 ss16118/low-latency-nccl near-SoL-collective rules from Research 143 (Plan 098; the DEVICE-side peer-poll half of barrier elimination — the host-orchestration half already lives in barrier-free-per-dependency-wait-from-loop-counters; corpus 550 -> 552, category E 43 -> 45; source ss16118/low-latency-nccl @ 5357eff325eddf978137de7140195a5568fa8a11, BSD-3 NVIDIA, shallow clone in .raw/low-latency-nccl removed at landing + the paper surface arXiv:2607.16100 'Every Microsecond Matters' [ETH+NVIDIA]; single-GPU workspace so no lane mapping — the B110 precedent: corpus rules serve USER code on multi-GPU hardware we do not own; PROVENANCE corrections at the pin: the repo names the sentinel protocol POISON [ncclLLPoisonBuffer/Async fills, ncclLLBufferIsPoisoned polls, the recv Reset arm re-poisons — no ncclLLBufferInitSentinel exists], the sizing helper is ncclCalcMinReductionBufferSize [LL doubles the buffer — the 2x-scratch claim verified in code at ll_buffer.cu:74], LL128 ENUMERATED 'NOT YET IMPLEMENTED' + // TODO in the recv path so the LL128-atomic caveat cites the PAPER; honest finds in-rule: the poison table maps by dtype SIZE, the 'Floating-Point Type Overrides' header is followed by NO specializations, stale usage comments claim 0x7FC00000/0xDEADBEEF vs the table's 0xFFFAFFFA-class NaN patterns, the 4-byte poison IS a representable i32 [-327686]; sentinel-readiness-over-separate-flag E [dtype-poison init + poll-non-sentinel; the four constraints — never-equal-sentinel, reset-before-reuse [the recv Reset arm IS the protocol; without it the reader consumes stale-as-fresh: the ABA the LL epoch band is immune to], peer-store-visibility, single-writer slots => determinism preserved], ll-flag-in-data-packed-poll E [the {flag,data} one-vector-store regime twin; epoch band 2..=255 with 0/1 reserved [reset writes 0; advanceEpoch +3 at 255], wrap-aliasing => LL REQUIRES multi-buffering ['(subBuffer % roundRobinFactor) * pitch'], both-flags-match ~0u retry for split slots; both entries carry the LL128-atomic determinism caveat — atomic-ADD accumulation is non-deterministic FP, gamma_{N-1} ~ 3.8e-6 @N=64 FP32, addition-only, NVLink cache-line atomicity, forbidden for bit-identity/replay-gated consumers]; rider: B58 += the bidirectional peer-exchange credit face ['each receive is implicit permission for the next send' + the repo rotation anchors; phrase-needle pins intact]; gates: batch119_sentinel_never_equal_and_reset_before_reuse [NaN-payload deliverability + the int-collision pin + the ABA/reset arm + all-slots readiness] + batch119_epoch_band_wraps_and_split_slot_matches [1000-advance band sweep never touches 0/1 + the wrap-aliasing pin + rotation offsets + split-slot match/retry + reset-flag-0]; GOAT: batch119 gates 5/5 FIRST-SHOT + kernel_heal lib 1455/0/1i debug 1456/0 release + default 1142/0 + combined 1574/0 release 1572/0/2i debug floors HELD zero re-pins + retrieval re-run kernel 100/100/100 blended 94.7/96.9/97.7 of 131 [byte-identical to Batch 118] + clippy 0 in default/kernel_heal/kernel_heal+latent lib + all-targets + score-bench #66 FLAT 96.2/1.9/100/100; count pins walked incl. the batch116 test's own count pin + the ops session's corpus_history.jsonl/dashboard regen artifacts left dirty as theirs); Batch 118: 6 antirez/ds4 PR #991 (Qwen3.8-Flash-Next) rules from Research 142 (Plan 097; source ivanfioravanti/ds4-metal branch qwen3.8-flash-next @ 236cb2a5, MIT, force-pushed once and HEAD re-verified == pin at mine time; upstream main unchanged at 9ab7053 = the Batch 117 pin so the PR branch IS the delta; two read lanes [kernel surfaces / handoff+io] + coordinator synthesis; the campaign QSA/GQA-MMA kernel generation verified at 35ce0b0 [removed by the Flash-Next rewrite] + HEAD successors; ds4_qwen4.c SUPERSEDED per Makefile:446-449 but the PLE gather code IS present; corrections vs the verdict: '7 sessions' is SIX [14->19], 'B98 NSG ladder' was a mis-mapping [B98 = tiny-rows-scalar-thread-per-row-downshift; the NSG content rides B95]; role-scoped-store-must-match-compute-guard A, tile-amplification-precedes-staging-pass H, operand-precision-gated-by-consumer-invariance N, compute-role-split-by-accumulator-pressure A, speculation-scheduler-break-even-flip D, running-threshold-prune-compaction-aliasing-guard I; + 1 rust_perf io rule parallel-disjoint-pread-hides-cold-page-latency [annotated NotBenchable per the B94 precedent]; + 7 riders: B24 xor-butterfly 8-lane sub-simdgroup twin, B95 GDN gated-delta-scan second family + NSG cooperation-radius invariant, B77 hc-inject derived-sum threadgroup-grain member + low-rank activation reuse, B57 hc_chain canonical + slot-partition complement, stripe-sms-l2-semaphore-reduce monotone-counter all-arrive GRID barrier + residency deadlock-freedom + recompute-to-avoid + the 9-49% fusion negative, block-reduction-separate-scratch-runtime-strides shuffle-lane unmutated-partial face [gate batch118_indexed_shuffle_ladders_must_read_unmutated_partials], smem-envelope-gate-kernel-variant short-K occupancy-first face [1.7x on threadgroup bytes alone]; gates: batch118_unguarded_store_yields_uniform_softmax [production-magnitude exactly-uniform detector + the tiny-magnitude invisibility arm]; the B22 FULL-STRING pin regenerated [1600 -> 2437 chars] + the batch116 test's own count pin walked [the 20th site]); Batch 117: 5 antirez/ds4 Metal tensor-prefill delta rules from Research 141 (Plan 090; the watch-coverage-gap batch -- ds4 had ZERO opponent-watch coverage until riir-ai 9639e7c50 added it; source main @ 9ab7053, MIT, ~14 commits past the Batch 97/98 pin 110afdd; fresh clone in .raw/ds4, removed at landing; two read lanes + coordinator spot-verification with three line-drift corrections [pack kernel moe.metal:9015 not 8987, tB read :9045, sum6 out :6693-6695]; routed-activation-pack-once-shared-operand L, device-affine-scratch-admission-with-pool-fallback A, batch-verify-rows-single-dispatch C, row-aware-indexing-reuses-static-single-token-kernel M, dense-fast-path-bounded-by-selector-coverage F; + 8 riders: B97 duplicate-pad-at-materialization complement, B35 TP non-owner additive identity + selector-padding sentinel, B23 byte-size static pin [2176/4456448/1088 imply the 32x17 MXFP4 block layout] + topology-axis widening, B57 single-pass two-phase softmax/LayerNorm twins + the four-backend one-commit audit, B58 remove-the-alloc-from-capture complement, B103 clamp-to-stay-on-bounded-gemm; NOT-LANDED as enablement-only: the MXFP4 packed-prefill arm [unproven, source refuses attribution, non-uniform 2K +9%/4K +2.8%/8K -2.3%]; measurement honesty: 5806d09's +11-23% QA table CONFLATES batching + seed schedule + MXFP4 arm -- cited whole-commit only; count-pin lesson: the batch116 test carried its OWN count pin [the 19th site] + the batch23 FULL-STRING concept needle regenerated with the B23 rider text folded in); Batch 110: 11 vLLM `libtorch_stable` rules from Research 133 / Plan 081 — the VERIFIED A-/B+ tier of a 25-candidate pool at pin 8277c42e4, Apache-2.0, never mined; two inspection lanes + coordinator byte-verification of 13 lead quotes; upstream hazard ledger in Research 133 — H1 silent tie-buffer overflow, H2 dispatch-no-default silent no-op, H3 cross-tier router divergence; the B-/C+ remainder = Batch 111; Batch 108: 4 riir-ai M3-campaign measured-GOAT rules from the in-source campaign (Plan 078; the first in-source batch since Batch 63 -- Issues 768/771/779/831/843 mined at pin 59a172623, numbers cited from Benches 774/790/804/831); Batch 107: 8 uzu + lily defers-batch rules from Research 130 — the B105 deferred queue consumed (12 candidates re-verified at the pins, 3 dropped, 1 collapsed to the B63 rider + 2 B95 riders; upstream `counts_offsets_fused` E>128 hazards extended and recorded in-rule); Batch 106: 5 MTPLX upstream-residue rules from Research 129; Batch 105: 11 uzu + lily combined Metal-engine rules from Research 127 + 128 (the cross-source combine mandate executed — the uzu/lily pairs cross-referenced in-rule, not merged; the tile-by-expert-load rider FALSIFIED at pin); Batch 104: 2 ggml-org/llama.cpp fa-vec swept-launch-table rules from Research 126; Batch 103: 4 MTPLX-PR391 + MLX-NAX host-orchestration rules from Research 124 + the B100 external-instances rider; Batch 102: 6 FlashQuant sparse-dense-fusion paper rules from Research 123; Batch 100: 8 Layr-Labs/qwen-3.8-mtp-challenge Swift/Metal arena rules from Research 120 + the B22 Metal cost rider; Batch 115: 6 qwen38-mtp submission-diff-history rules from Research 139 (6 rules + 8 riders + the collision-salvage addendum: 4 rules + 6 riders, corpus ->532; two read lanes + coordinator synthesis at pin 0863b06ac; B100's tip-TREE mine complementary, not superseded; ordinal-map tail selection, complete-coverage scatter collapse, exhaustive-domain input patch, derived-index cache legality, warm-reachable-signatures, env-gate plumbing = measurement-honesty #2); Batch 116: 6 PrismML fork rules from Research 140 (Plan 089; the fork 12 commits past the league pin e311ed38f, mined via git-show at 7113f6462, vendor checkout untouched; activation-bitplane-popcount-word-dot N, scratch-sizing-shares-dispatch-eligibility-predicate D, byte-owning-threads-non-positional-digit-codes A, fwht-wide-block-row-per-threadgroup-exchange-ladder E, shader-shape-cutover-keys-on-resource-block-ratio D, same-input-folded-transform-memo-per-build D; + 3 riders: telescoped += radix-general mantissa-bound exactness (floor(R^k·b/2^m) exact iff R^pos·max_code < 2^24 — the base-3 PTQ1_0 extraction is bit-exact WITHOUT a power-of-two argument), bridge-rung += register-budget nr1 sizing + exact-fit nr1=3 + per-type mv/mm crossover, dispatcher-eligibility += 4 inverse-direction instances incl. the head-truncated-grep transcription + the clean-tail-truncated test run; league-structural: PTQ1_0 = 1.75 bpw vs PQ2_0 2.125, bit-identical reconstruction — owner fork-repin intel only; Batch 116 ADDENDUM (2026-09-06, the katgpt-rs Issue-665 collision salvage — a parallel M3 session authored the same batch concurrently; the landed batch owns per push-wins, the duplicate yielded before commit): 1 rule + 2 riders + 2 standing numeric gates, corpus 538 -> 539 (N 35 -> 36) — swar-lane-widened-base3-dp4a-dot N (the int8 dp4a CONSUME family of the same base-3 codec: 16-bit lane widening so the radix multiply cannot carry between bytes; answers the packing rule's 'dp4a can't consume base-3' limitation; riders: the digit-LUT-vs-arithmetic per-path split [142.1-vs-143.5 dependent-load negative] + coarse-scale format retention with the FOUR-surface new-type registration completeness; guard-side mirror rider on dispatcher-eligibility: an assert widened to admit F16 lost its buffer-layout conjunct [~3.8x wdata overrun if reached] — the face the landed 4 instances do not cover); batch116_addendum_base3_mantissa_bound_exhaustive [pins the landed rider's prose criterion: exhaustive radix-3 256x5 + the radix-7 within-bound control + the radix-11 falsifiable negative that MUST diverge] + batch116_addendum_swar_lanes_exhaustive [per-lane 256x4x3-neighbor sweep + the exhaustive adjacent-pair carry-channel sweep]); Batch 98: 10 antirez/ds4 dense.metal/moe.metal rules from Research 118; Batch 97: 9 antirez/ds4 DwarfStar Metal rules from Research 117; Batch 96: 4 PrismML fork Metal quant-MV rules from Research 114; Batch 95: 3 mlx-lm gated_delta.py GDN-prefill rules from Research 111; Batch 92: 11 ktransformers-Marlin CUDA-technique rules from Research 107; Batch 1-6: 50 popcorn-distilled from Research 002-007; Batch 7: 9 burn-cubecl
   Rust-native autotune dispatch + conv-as-GEMM + layout rules from Research 008;
   Batch 8: 9 burn-cubecl kernel polymorphism + dtype/alignment specialization +
   cubecl-opt IR strength-reduction rules from Research 009;
   Batch 9: 8 popcorn fusion + dtype/quant + dispatch rules from Research 010;
   Batch 10: 12 popcorn compute + chunked reduction + polymorphism rules from Research 011;
   Batch 11: 11 popcorn linear-attention recurrence family rules from Research 012;
   Batch 12: 14 popcorn gated delta + specialized recurrence + advanced attention rules from Research 013;
   Batch 13: 6 popcorn final-pass rules (RL loss stability, complex-free RoPE, multimodal RoPE, RWKV-7 channel mixing + gate output) from Research 014;
   popcorn corpus distillation COMPLETE — every popcorn kernel file read;
   Batch 14: 4 riir-gpu production-kernel rules (norm-fold into matmul, reduction-split pre-pass, single-plane reduction, CPU-bottleneck offload) from Research 015 — the first batch mining the stack's own kernels, not external libraries);
   Batch 15: 4 riir-gpu production-kernel rules (block-contiguous weight layout, fused dual GEMV + GeGLU, orthogonal pre-rotation, water-fill tail-dim shared codebook) from Research 016 — the second batch mining the stack's own kernels (ternary GEMM + fused GeGLU GEMV + SpectralQuant);
   Batch 16: 4 riir-gpu production-kernel rules (cmma hardware unit, dequant-once-per-k-tile, dispatch axis swap, norm-decoupled low-bit scoring) from Research 017 — the third batch mining the stack's own kernels (gemm_ternary_simdgroup + forward_turboquant).
   Batch 17: 4 riir-gpu production-kernel rules (skip-all-zero-packed-words-via-or-reduce, hoist-group-invariant-scale-out-of-word-loop, split-sign-accumulators-defer-subtraction, clamp-tile-indices-to-sentinel-discard-at-write) from Research 018 — the fourth batch mining the stack's own kernels (gemm_ternary_batched + gemv_ternary_block_contiguous — the ternary scalar-ALU hot loop). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 18: 3 riir-gpu production-kernel rules (row-tile-amortize-input-load, factor-affine-quant-scale-out-of-element-loop, pin-lane-to-quant-sub-block-coalesce-byte-loads) from Research 019 — the fifth batch mining the stack's own kernels (gemv_q4k_cubecl — the Q4_K quantized GEMV family: GEMV→small-GEMM M-widening, affine dequant factoring, lane-pinned byte coalescing). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 19: 2 riir-gpu production-kernel rules (chain-dispatches-single-sync-point, fuse-residual-via-in-place-output-array) from Research 020 — the sixth batch mining the stack's own kernels (ternary_ffn_fused + gemv_ternary_residual_cubecl — command-buffer-level dispatch fusion + in-place output-array parameter-count bypass). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 20: 5 riir-gpu production-kernel rules (cooperative-block-wide-quantize-staging, byte-perm-lut-via-permutation-immediate, dp4a-int8-hardware-dot-product, threadgroup-subset-sum-lut-keyed-by-activation, base-3-trit-packing-5-per-byte) from Research 021 — the seventh batch mining the stack's own kernels (gemv_ternary_cuda_raw + gemv_ternary_cubecl LUT/trit paths — first CUDA-specific hardware-instruction cluster: __byte_perm permutation LUT + __dp4a int8 dot product + cooperative block-wide quantize staging; plus CubeCL threadgroup subset-sum LUT keyed by activation + base-3 trit packing). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 21: 5 riir-gpu production-kernel rules (cross-runtime-raw-handle-extraction, pool-buffer-aliasing-dedicated-output, same-queue-implicit-ordering-no-sync, per-dispatch-pointer-re-extraction, no-blocking-sync-in-async-pipeline) from Research 022 — the eighth batch mining the stack's own kernels (gemm_ternary_metal_zero_copy — the Metal cross-runtime zero-copy integration cluster: raw `id<MTLBuffer>` extraction from CubeCL/wgpu handles + pool-buffer aliasing hazard workaround + same-queue implicit ordering + per-dispatch pointer re-extraction + no-blocking-sync in async pipeline; patterns are generic for any wrapper-runtime→raw-native-dispatch integration). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 22: 5 riir-gpu production-kernel rules (dynamic-shared-memory-runtime-sized, ternary-bit-plane-u64-as-2x-u32, warp-shuffle-intrinsics-for-sub-warp-reduction, fused-activation-quantize-for-gemv-consumer, cuda-graph-scalar-via-device-pointer) from Research 023 — the ninth batch mining the stack's own kernels (cudarc_kernels/ — the full raw-CUDA forward port for riir-ai Issue 615: embedding + attention + elementwise + deltanet; patterns include `extern __shared__` runtime-sized smem, bit-plane ternary dequant, `__shfl_down_sync` warp reduction for ≤32-thread blocks, activation+quantize fusion for GEMV consumers, and `_devpos` variants for CUDA Graph capture). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 23: 5 riir-gpu production-kernel rules (combined-kv-buffer-keys-then-values, gqa-kv-head-sharing-bandwidth-reduction, softcap-tanh-attention-score, fold-query-tokens-for-occupancy, hardcoded-fast-path-jit-specialization) from Research 024 — the tenth batch mining the stack's own kernels (attention_cubecl/ — the `CubeCL` flash-attention decode cluster: combined KV buffer `[keys | values]`, GQA KV-head sharing via `kv_group = head_idx * n_kv_head / n_head`, Gemma's `softcap * tanh(score/softcap)` bounded logits, fold-factor query-token coalescing for speculative-decode occupancy, and hardcoded fast-path vs parametric JIT specialization; ~24% decode throughput gain on M3 Max). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 24: 4 riir-gpu production-kernel rules (widen-index-multiply-to-64bit, dual-accumulator-single-reduction-sweep, dual-grad-same-pass, broadcast-uniform-skip-reduce) from Research 025 — the eleventh batch mining the stack's own kernels (cudarc_kernels/backward.rs — the riir-ai Issue 641 per-layer backward cluster; closes `cudarc_kernels/` 6/6: 64-bit index widening for address math on large buffers, two block reductions fused into one sweep sharing loads + barrier tree, both partials of a product-rule activation in one kernel sharing one sigmoid eval, and thread-uniform values broadcast-not-reduced — the T7.2 validation catch where z came out blockDim.x× too large). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 25: 5 riir-gpu production-kernel rules (device-side-argmax-minimal-download, monotone-transform-elision-under-argmax, fan-out-shared-input-single-sync, hot-swap-weights-preserve-autotune-caches, tree-reduction-depth-over-sequential-accumulate) from Research 026 — the twelfth batch mining the stack's own kernels (the Gemma cluster: gemma2_cubecl/ + gemma4_cubecl/ — GPU argmax downloads 4 B instead of ~1 MB logits/token, strictly monotone softcap skipped under argmax-only consumers, N sibling QKV dispatches sharing one uploaded input + one batched sync, `replace_weights` hot-swap keeping autotune + JIT caches alive across LoRA steps, and plane tree-reduction depth ≈77 vs sequential 2304 for ~30× backward accuracy — the riir-train Issue 424 gradient-divergence fix). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 26: 8 riir-gpu production-kernel rules (per-slot-uniform-buffer-write-ordering, derive-params-from-global-invocation-id, merge-dispatches-into-single-compute-pass, batched-downloads-single-sync-cycle, chunk-encoder-submits-below-driver-limit, prebuilt-bind-groups-fixed-buffer-combos, host-cache-immutable-device-weights, defer-readback-until-after-submit) from Research 027 — the thirteenth batch mining the stack's own kernels (forward/ + buffer.rs — the wgpu forward-orchestration cluster, the first HOST-SIDE batch: wgpu flushes all `queue.write_buffer` calls before the encoder executes so shared uniforms corrupt (per-slot fix), per-item params encoded in the dispatch grid + decoded in-kernel (n_head × seq_len workgroups, one dispatch), dependent dispatches merged into one compute pass with in-order semantics + copies on the copy channel, N readbacks sharing one encoder/submit/map/poll (~1.28 ms per measured sync), Metal compute-pass-per-encoder DEADLOCK avoided by chunked immediate submits (~48 passes/chunk, env-tunable), fixed-combo bind groups pre-built at init (~768 `create_bind_group` calls per forward → 0), immutable weights host-cached at init (per-call download was the #1 forward bottleneck at 2 sync stalls), and readback deferred until after submit (mid-encoder downloads read stale pre-submission state). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 27: 6 riir-gpu production-kernel rules (staging-packed-uniforms-encoder-ordered-copies, identical-uniforms-share-single-slot, persistent-zeros-source-encoder-copy-clear, fuse-sibling-target-grads-single-kernel, per-layer-strided-grad-destination, snapshot-transient-grad-for-deferred-download) from Research 028 — the fourteenth batch mining the stack's own kernels (backward/ + backward_dflash/ — the backward-pass orchestration clusters, completing the host-orchestration trilogy: all uniform params packed into ONE 16-byte-aligned staging vec + ONE upload + encoder-ordered per-dispatch copies (the 1-buffer complement to per-slot's N buffers), the idempotent-write exemption (sharing is safe IFF every write is byte-identical — N uniforms + N bind groups → 1), persistent device zeros + encoder-timeline copy clears (queue writes flush before the whole encoder so layer 1 would accumulate on layer 0's values; three-way independent mining convergence), sibling-target backward fusion per KERNEL KIND (one summed d_input + one grid-sized param-grad dispatch; 6 → 2), consumer-selected layer-major strided grad destinations (the per-layer split falls out of the existing pass — no de-interleave), and transient-grad snapshots into persistent buffers for deferred download (the LIFETIME complement to defer-readback's TIMING rule). Mining also surfaced a suspected live bug in backward_dflash (uniforms written via queue.write_buffer DURING encoder build → wrong query_pos for all but the last position per chunk with chunk_size=2) — filed as a riir-ai issue. Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 28: 10 riir-gpu production-kernel rules (row-owned-register-blocked-recurrence, workgroup-equals-plane-native-reduction, smem-broadcast-scalar-then-parallel-scatter, token-flattened-elementwise-modulo-shared-weights, fused-conv-state-shift-dot-silu, softplus-piecewise-saturation-clamp, reverse-running-decay-weight, split-decay-outputs-eliminate-readback, carry-dual-layout-dead-slot, nonlinearity-forces-raw-input-buffers) from Research 029 — the fifteenth batch mining the stack's own kernels (deltanet_cubecl.rs + deltanet_chunked_cubecl.rs — the deltanet CubeCL cluster, the two largest single files in the crate: decode-time recurrence + chunkwise-parallel prefill): state-matrix row ownership with register blocking (rows independent within one decode step → one workgroup per (head,row), 4R+2W → 1R+1W global passes, ~3× memory-bound cut, 32× occupancy headroom; Issue 619), plane-sized workgroup geometry (size to EXACTLY one hardware plane + register-block width — native shuffle covers the full reduction, broadcasts free, zero barriers; the Issue 610 plane_sum-caught-1/4 hazard is the inverse), post-reduction broadcast scatter (leader publishes the scalar through the already-reduced slot, ALL threads scatter — 1 write/thread vs head_dim serial leader writes; Issue 616), mixed-arity launch flattening ([p, n_head] → one 1-D index space with per-head weights via i % n_head; 6144 → 48 dispatches), fused in-place conv shift+dot+SiLU decode kernel, far-tail softplus saturation clamp (±20, pinned to the CPU reference), dematerialized decay matrix (one running scalar, j downward, empty-product anchor — the inverse of cumsum-gate-to-decay-matrix), consumer-shaped output split (packing that breaks consumer layouts forces readback-unpack-reupload; write [H,C] + [H] directly), parameterized dual-layout carry with a provably-dead slot (prefill writes decode's conv_state natively; slot 0 stale — the decode shift overwrites before read), and the nonlinearity buffer contract (sequential in-place tricks are incompatible with batching when a nonlinearity intervenes — raw RO inputs + WO outputs; the negative space of inplace-output-alias). Corpus 183 → 193 (B 33, C 19, D 26, E 11, G 19, L 6). Mining CONFIRMED a production bug (deltanet_cross_chunk_f32 plane_sum mixes partials of different columns → block-constant output, masked by test tolerance, production-wired via prefill_deltanet_recurrence_chunked) + 3 latent/edge bugs (legacy recurrence broken for head_dim≠128 while being the documented fallback; partial-chunk carry GPU-vs-CPU divergence; GPU L2-normalize missing the CPU zero-norm guard) — filed as a riir-ai issue.
   Batch 29: 10 riir-gpu production-kernel rules (defer-readback-gated-by-live-output-size, weight-handle-cache-keyed-by-allocation-identity, first-readback-pays-queue-drain, epilogue-at-register-write-phase, delay-rms-scale-past-gemv-epilogue, pack-scalar-params-into-combined-buffer, enum-dispatch-separate-kernel-per-variant, epilogue-must-be-workgroup-local, launch-unchecked-caller-owned-size-contract, fast-math-contraction-behind-feature-flag) from Research 030 — the sixteenth batch mining the stack's own kernels (deltanet_input_proj_fused.rs + epilogue/ — the decode-path fusion cluster: Issue 602 sibling-GEMV fan-out + Plan 106 Track 3 CODA epilogue fusion; ternary_deltanet_gpu_forward.rs deliberately skipped — sibling actively working there): the readback-deferral footprint gate (Bench 437's 5 concurrent ~70 KB FFN buffers measured 16% SLOWER vs ~8 KB input-proj outputs chaining freely — deferral is NOT unconditional), pointer-identity device-handle caching (CPU weight refs → lock-free cache keyed by the allocation's (pos, neg) pointer pair, on-the-fly miss path), the queue-drain readback cost model (first read drains the burst's queue, later reads free — syncs are a function of BURSTS, not read calls), the CODA register-phase epilogue (memory-bound elementwise ops execute while the GEMV output is still in registers, riding lane 0's single store; 2→1 dispatches, 52/26 layers), the delayed-RMS commutation (RMSNorm(x)@W = r·(x·γ)@W with the honest 3→3 accounting — the win is dispatch overhead, not bandwidth), packed scalar params for the ~4-5 Array budget (u32s bitcast through the f32 pack), enum + one-kernel-per-variant under the no-generics DSL constraint, workgroup-local fusion legality (ownership scope, not just dependency), launch_unchecked + # Safety size contracts, and fast-math contraction behind a cfg_attr feature pair (strict-FP default byte-identical). Corpus 193 → 203 (A 29, B 34, C 21, D 29, E 12, M 6). First standing syn-parse gate (batch29_pseudocode_parses). Mining surfaced 2 latent cache hazards + a stale-doc-numbers bug — filed as riir-ai Issue 674.
   Batch 30: 6 riir-gpu production-kernel rules (zero-init-side-branch-frozen-base-path, counter-derived-seed-stateless-noise, predicate-gated-skip-empty-workset-step, share-pipeline-set-across-subsystems, pad-uniform-pod-to-min-binding-size, fuse-count-into-mutation-pass) from Research 031 — the seventeenth batch mining the stack's own kernels (dllm/mod.rs — the D2F training cluster: Plan 068 discrete-diffusion LM trainers + the Plan 078 RePlaid adaptive noise schedule): the frozen distillation teacher run through the SAME adapter-capable forward with B=0 (W·x + B·(A·x) = W·x exactly — one code path + one weight set for both modes, grounded on the lora/mod.rs zero-B constructor contract), counter-derived stateless noise seeding (((step << 32) | epoch) → pure-function-per-iteration streams, standalone replay, skip-independence, the same seed reaching the GPU as one uniform bit-identical to noise_corrupt.wgsl), whole-step elision on an empty workset (one host-side integer check skips forward+loss+backward+optimizer AND guarantees the count-based loss normalizer never divides by zero — guarded by construction), the Arc-shared pipeline set (WGSL validation + compilation paid once per run across loss/backward/optimizer constructors), the explicit 16-byte uniform pad discipline (explicit _pad u32 fields + size_of test pins — one fixed-size buffer helper serves all kernels, layout breaks CI instead of the GPU), and the count fused into its producing mutation (the masked count rides already-hot cache lines instead of a second O(n) scan; feeds normalization + the workset gate). Corpus 203 → 209 (C 22, D 31, E 13, M 7, N 12). Standing syn-parse gate extended (batch30_pseudocode_parses). 13 candidates dropped as duplicates/adjacent (GPU→GPU copy triply covered — the source comment cites Issue 018; per-call bind groups + naive downloads + CPU argmax = new SLOW instances of Batches 25/26 rules; host scratch reuse = cross-domain rust_perf reuse-allocation-clear; 3 RePlaid controller patterns out of kernel_opt scope, recorded in Research 031 §4). Mining bug yield filed as a riir-ai issue: unwrap_or_default swallowing GPU download errors (silently pins best_loss to 0.0 forever — the checkpoint-best gate can never fire again), the unit-incoherent RePlaid adapt() sign basis (loss EMA compared against a mask ratio), the adapt_ratios() sort orphaning tracker state, the zero-masked skip firing AFTER the expensive teacher forward in the distill loop, DistillMode::D2fWeighted dead config, and RePlaid being unwired dead code re-exported as public API with a misleadingly-named wiring test.
   Batch 31: 8 riir-gpu production-kernel rules (ragged-pack-offset-table-fused-dispatch, twin-projection-shared-delta-adapter, min-rank-inert-adapter-zero-alpha, debug-only-gpu-label-macro, sync-map-result-stack-cell, staging-reuse-grow-only-map-prefix, mapped-at-creation-init-upload, dtype-normalize-once-at-upload) from Research 033 — the eighteenth batch mining the stack's own kernels (the training-substrate cluster: lora/mod.rs + optimizer.rs + dataloader.rs + buffer.rs — the layers UNDER the dllm loop mined in Batch 30): the 12-per-adapter GEMV dispatches repacked into contiguous buffers + a prefix-sum offset table (one launch over Σ out_dim; ragged segments need offsets, a modulo cannot identify them), GQA K/V shape-twins sharing ONE adapter pair (adapters 6→5/layer, params −33%, KV delta cache halves at inference), structurally-required-but-inert adapters at rank=1 + alpha=0 (minimum legal footprint, annihilated by scale not contents — the ALLOCATION contract complement to Batch 30's B=0 VALUE contract), the cfg-split GPU label macro (debug = format!'d capture labels, release = a typed None constant), the map-async callback parking its result in an UnsafeCell under the poll(Wait) barrier (zero lock + zero alloc per readback — poll IS the happens-before edge), the loop-owned grow-only MAP_READ staging buffer (~50-500μs driver alloc amortized to zero steady-state, mapping only the copied prefix; merged from two subagent proposals), one-shot weight uploads via mapped-at-creation (no encoder, no submit, no queue-timeline ordering hazard), and bf16→f32 normalization once at ingestion (a pure bit-shift instead of a per-dispatch conversion forever). Corpus 209 → 217 (A 32, D 32, L 7, M 9, N 13). Standing syn-parse gate extended (batch31_pseudocode_parses). Mining bug yield: the CRITICAL AdamWOptimizer shared-uniform truncation (every recorded pass saw the LAST write's param_count — Mlp1.B/Mlp2.A silently trained only 25% at the 4× SwiGLU ratio in every GPU LoRA loop) FIXED as riir-ai Issue 679A (per-slot uniforms + full-length regression test verified to fail under the old aliasing at exactly 96/128) + 10 residual hazards filed as Issue 679 B1-B10.

   Batch 32a: 11 riir-gpu production-kernel rules (caller-owned-split-scratch, inplace-gate-scale-at-funnel-bottleneck, sign-branch-stable-sigmoid, incremental-multi-part-hash-no-concat, bool-fields-over-bitset-hot-reads, per-layer-grad-norm-lr-equalization, width-invariant-superset-training, superset-sized-scratch-across-variants, active-index-grad-scatter-subset-backward, inplace-grad-accumulate-deferred-postscale, fuse-loss-and-output-grad-one-pass) from Research 034 — the nineteenth batch mining the stack's own kernels (the edge_lora CORE cluster: edge_module.rs + types.rs + training.rs + adaptive_width.rs — host-side training orchestrators; first half of the edge_lora module per the 32a/32b split, 32b = self-play/arena/consolidation remains): the hot-path fn taking ONE caller-owned scratch carved by split_at_mut into intermediate+output with a borrowed-view return (trait-shaped allocating paths quarantined as cold adapters), the sigmoid gate running IN PLACE on the rank-wide intermediate AND absorbing the broadcast scale (rank ops not out_dim — placement-of-work at the funnel's narrowest point), the sign-branch stable sigmoid (exp's argument always <= 0; the naive form overflows f32 exp at x < -88.7 — the house gate-math style, matches katgpt-rs latent_to_raw_scalar), BLAKE3 hasher.update per tensor replacing the materialized a||b concatenation (the streaming update IS the concatenation — zero per-step allocation on the commitment path), the REGIME BOUNDARY of packing (per-tick-READ flags stay bools — decode ALU dominates; pack only when bytes streamed dominate — the inverse of the packed-weights rules), per-step per-layer grad-norm LR equalization min(max_norm/layer_norm, cap) combating a measured ~1000× vanishing gradient (zero-norm→1.0 rails), ONE superset weight set trained across difficulty-sampled widths serving every inference budget (anytime-inference; gate: width-1 ≥ 95% of width-4), scratch sized ONCE for the statically-known worst-case variant (caveat: exact-length consumer checks force per-variant allocs — relax to prefix-view semantics), backward in dense sub-space + precomputed active-index scatter into the full buffer (inactive params zero BY CONSTRUCTION — no masks, no atomics; the host-side shape of MoE backward), accumulate-RAW-then-ONE-post-scale (1 division total, not batch×params), and loss scalar + analytic output-gradient from the SAME diff in one sweep. Corpus 217 → 228 (A 35, B 37, C 24, D 33, H 10, L 8). Standing syn-parse gate extended (batch32_pseudocode_parses). Mining bug yield filed as riir-ai Issue 683: the AdaptiveWidthTrainer per-episode construction storm (edges.clone() + fresh TopologyTrainer + fresh ForwardCache + fresh EdgeGradients per EPISODE when only 3 schedules exist — root cause documented in-source as the exact-length cache check) + silently ignored TrainingConfig fields (weight_decay/grad_clip_norm/per_layer_lr_scale) + misreported mean_weighted_loss + the zeroed() doc causal error (B=0, not A=0) + route_into release-panic path + the additive-aggregation stale-out footgun.
   Batch 32b: 10 riir-gpu production-kernel rules (static-classification-hoist-to-routing-table, sigmoid-deriv-recovered-from-output, fuse-weight-grad-input-grad-single-matrix-visit, needless-swap-temp-hoist-to-scratch, scoped-init-at-consume-site, monomorphize-per-element-trait-calls, additive-sigmoid-gates-not-softmax-routing, commit-over-maintained-item-digests, verify-once-at-trust-boundary, derive-shape-from-payload-not-cached-field) from Research 035 — the twentieth batch mining the stack's own kernels (the edge_lora training half: topology_trainer.rs + chain_commit.rs + sigmoid_gate.rs + mod.rs — the topology-evolution trainer + chain commitment + per-edge additive gating; the 32b-1 half per the suggested split, 32b-2 = self-play/arena/consolidation remains): immutable-topology classification hoisted ONCE at construction into CSR-style routing buckets (the naive form re-scans an O(L) probe per edge per call in ALL THREE hot loops while edge_source_layers() already computes the mapping that only a cold path consumes), sigmoid' recovered algebraically from the cached OUTPUT (σ'(x) = σ(x)·(1−σ(x)) incl. through a scale factor — halves activation-cache traffic), the weight-grad outer-product write + input-grad transposed matvec read fused into ONE visit per B row (two gradient ROLES of one matrix), the false-premise defensive swap temp deleted (the comment claimed in-place aliasing but read/write hit DIFFERENT buffers — genuine ping-pong temps are loop-carried scratch state), blanket whole-scratch clear replaced by scoped accumulate-window zeroing (only ACCUMULATION targets need pre-zero; O(total scratch) → O(active window)), per-element &dyn devirtualized to generic + ?Sized (the same file's backward was already generic — the inconsistent pair was the evidence), per-edge independent sigmoid gates composed additively instead of softmax routing (no cross-edge reduction barrier; multiple edges simultaneously near 1.0 by design — the house sigmoid-not-softmax rule as a corpus rule), container commitments digesting maintained per-item digests (O(items) not O(total payload bytes)), verification ONCE at the trust boundary via a capability type (the load path paid up to 3 full BLAKE3 passes), and shape truth derived from the payload not a cached field (the merkle_root-class desync bug structurally impossible). Corpus 228 → 238 (A 39, B 39, C 25, D 35, E 14). Standing syn-parse gate extended (batch32b_pseudocode_parses + batch32b_rules_present + batch32b_concepts_covered). Mining bug yield filed as riir-ai Issue 684: the topology_trainer hot-loop allocation storm (per-node vec! on a FALSE aliasing premise ×2 sites + per-edge to_vec() borrow workaround + backward staging buffer) + the O(E·L²) routing re-derivation ×3 loops + dead/redundant scratch clears (gates_scratch/edge_outputs never read in production) + the triple-BLAKE3 load path + the 7-site sigmoid duplication + the discarded topology validation error + silent u32→u16 narrowing.
   Batch 32c: 13 riir-gpu production-kernel rules (incremental-centroid-running-sum, length-prefix-shape-faithful-hash, order-stable-topk-via-index-sort, make-unmake-over-clone-per-candidate, argmax-rng-tie-break-collect-ties, shape-gated-warm-start-slot-copy, domain-as-data-not-algorithm-fork, one-sweep-spatial-feature-fanout, deterministic-tiling-projection-tables, latent-encode-at-capture, non-saturating-outcome-reward-floor, collect-then-resolve-snapshot-batch, timeout-scalar-tiebreak-decisive-outcome) from Research 036 — the twenty-first batch mining the stack's own kernels (the edge_lora self-play/arena/consolidation half: sleep_consolidation.rs + arena_eval.rs + cross_game_edge.rs + cross_game_real.rs + {go,bomber,fft}_self_play.rs — the 32b-2 half; the edge_lora module is now FULLY MINED): the greedy clustering loop recomputing each centroid from scratch per comparison (full member sweep + fresh Vec per (item, cluster) pair) replaced by a maintained (sum, count) per cluster (the numeric-aggregate twin of digest maintenance), payload-only multi-part hashing feeding part LENGTHS into the stream first (shape-blind otherwise: a=[1,2],b=[3] collides with a=[1],b=[2,3]), top-K retention via sort-index-truncate-re-sort (items never move, canonical output order, NaN-safe comparator), candidate search make/unmake instead of whole-state clone per probe (bit-identical rollback, zero per-candidate alloc), argmax with collect-epsilon-ties + one seeded RNG draw (vs max_by's hidden first-max priority or softmax's normalization), warm-start slot copy gated on the full shape tuple, cross-game training as DATA conversion over the unchanged loop (no forked trainer — correctness inherited), 12 spatial statistics from ONE board sweep (classification indices + signed indicator + geometry-derived denominators), a modelless fixed projection as construction-time tables (modulo schedule + even-spacing-with-jitter phases — same seed rebuilds the identical encoder), latent encoding snapshotted at capture (peak memory moves × hidden_dim, not moves × full state), terminal outcomes ramped off the extremes (0.15 + 0.70·clamp — losses keep non-vanishing gradient weight), simultaneous-turn ticks split into collect-against-frozen-snapshot then resolve-in-fixed-order (no information leak, parallelizable select), and timeout falling through to a maintained-scalar HP tiebreak instead of a flat draw. Corpus 238 → 251 (A 42, B 42, C 27, D 39, E 15). Standing gates extended (batch32c_pseudocode_parses + batch32c_rules_present + batch32c_concepts_covered; awk orphan-rule_id pre-compile scan clean — the Python-generator landing produced a clean first compile). Mining bug yield filed as riir-ai Issue 685: the quadratic centroid storm + swallowed failed move + wasted encodes on Pass moves + warm_start weight duplication + per-call dataset deep-clone + the false-premise sigmoid comment (named sigmoid, computes an affine ramp) + dead init-touch loop + Wait-vs-continue mismatch + per-tick nested-Vec grid rebuild + the adapter-skeleton triplication family (7 duplicated shapes × 3 game adapters — the self-play half of the Issue 684 sigmoid dedup family).
   Batch 33: 7 riir-gpu production-kernel rules (grid-over-output-thread-private-sweep, buffer-set-struct-two-tier-dispatch, slot-offset-partial-buffer-write, immutable-operand-upload-at-set-creation, selective-projection-effective-rank-head, derived-scalar-host-computed-in-uniform, rademacher-signs-over-gaussian-projection) from Research 037 — the twenty-second batch mining the stack's own kernels (the spectralquant calibration cluster: calibration.rs + rotation.rs + qjl.rs — the calibration path Research 016 explicitly deferred; the spectralquant/ directory is now FULLY MINED; bench-only today, sole consumer spectralquant_bench.rs): the output-owned grid + thread-private seq_len sweep (zero cross-thread reduction, no partials, no reduce kernel — the dual of split-K), the buffer-set struct as reuse unit with a two-tier dispatch API (preferred entry only writes uniform content; raw entry self-allocates for cold callers), slot-offset delta writes mirroring the kernel's base index (d_h×4 bytes not the whole tensor), immutable operands uploaded ONCE in the constructor (dispatch never sees operand bytes again), selective projection on the effective-rank head only (operator + FLOPs scale with d_eff ≈ 4–6, not d_h = 128 — ~21× on both; sibling of water-fill: bits-by-importance vs projection-work-by-importance), config-derived scalars computed once on host + shipped in the uniform (CPU reference and kernel consume bit-identical f32 by construction; WGSL sqrt is 2.5-ULP not correctly rounded), Rademacher ±1 signs over Gaussian (one coin flip per entry, ±1.0 exact in f32 so every product is exact; conditional-negate strength reduction licensed but unrealized — honest caveat in the rule). Corpus 251 → 258 (A 44, B 44, C 27, D 40, E 16, N 14). Standing gates extended (batch33_pseudocode_parses + batch33_rules_present + batch33_concepts_covered; 8 clippy doc warnings fixed pre-commit; drive-by fix of the stale M-category (7 rules) comment → 9). ONE seed REJECTED on verification: two-stage tile→reduce is NOT what covariance.wgsl does — the producer writes no tile partials, so n_tiles > 1 would reduce uninitialized memory (filed as riir-ai Issue 686 H1 instead of becoming a rule — encoding it would have put a bug in the corpus as wisdom). 11 further candidates dropped as duplicates/thin. Mining bug yield filed as riir-ai Issue 686: the n_tiles>1 silent-corruption path + layer_offset dead in eigenbasis_rotation.wgsl (multi-layer rotation silently reads layer 0) + total_layers dead + per-dispatch bind-group creation in all four dispatchers + create_uniform_buffer doc-lie + duplicated host/kernel stride contract + 2-barriers-per-position smem staging + dead code + tautological test + latent shared-uniform ordering hazard.
   Batch 34: 9 riir-gpu production-kernel rules (exact-op-parity-for-analytic-backward, mirror-ring-store-sliding-window-contiguity, split-dispatch-expose-fused-intermediate, preallocated-activation-slots-inplace-save, hybrid-cpu-elementwise-gpu-throughput-sync-points, pre-transposed-consumer-layout-handles-build-once, shape-matched-sibling-weight-grads-share-input-loop, op-major-backward-loop-inversion-batch-sequence-axis, backward-helper-expose-shared-norm-statistic) from Research 038 — the twenty-third batch mining the stack's own kernels (the Gemma-4 TRAINING cluster: gemma4_cubecl_train/mod.rs + backward_gpu.rs — hybrid GPU training forward + Phase 2.5 GPU backward; the training counterpart of Batch 25's inference-side Gemma cluster): the training forward evaluating the EXACT op the analytic backward differentiates (standard tanh not fast_tanh — forward value + derivative from two different functions = systematically biased gradients), the 2·sw mirrored KV ring making any ≤sw window contiguous by construction (read contract raw-indexed, valid while seq_len ≤ 2·sw), the fused chain that discards its intermediate forcing a producer re-launch (ship a split entry returning both outputs from ONE execution — gemma2's dispatch_attention_wo_split is the proven precedent), activations written into pre-sized [seq][dim] slots at the moment of production (forward loop alloc-free regardless of save count), the hybrid CPU/GPU boundary with exactly 4 syncs per layer per position (GPU takes only throughput-bound GEMV/attention; elementwise runs CPU-side in the sync gaps), backward's W^T materialized ONCE as resident handles under the frozen-base contract (~2× memory, triple cross-model recurrence: gemma2 + gemma4 + gemma4_q4k), shape-matched sibling weight-grads sharing ONE walk of the shared input (wk+wv over normed_attn, gate+up over normed_mlp — honest caveat: L1-resident CPU inputs make it loop-overhead halving), the op-major backward loop inversion (positions free inside each op stage — attention backward sits BETWEEN stages — so per-position GEMVs become sequence-batched GEMMs, 7/layer/step not 7×seq_len), and the backward helper exposing the norm statistic it already computed (rmsnorm_backward must return inv_rms, not hide it from the gamma-grad path — 2-3× reduction waste at 7 sites). Corpus 258 → 267 (A 46, B 44, C 28, D 43, E 16, H 11, L 9, N 15). Standing gates extended (batch34_pseudocode_parses + batch34_rules_present + batch34_concepts_covered; 8 clippy doc warnings fixed pre-commit incl. a NEW lint variant — a leading `+` on a wrapped doc line parses as a markdown list item). TWO seeds resolved by verification: the softcap backward does NOT recompute tanh — it algebraically recovers it from the cached POST-cap logits (already sigmoid-deriv-recovered-from-output; falsified by the subagent), and the dual-KV-cache "pattern" was REJECTED as wisdom (avoidable 2-3× duplication — filed as Issue 687 H9). Mining bug yield filed as riir-ai Issue 687 (13 hazards): the mirrored-ring backward OOB panic for seq_len > 2·sw (index algebra verified — correct below the boundary, no assertion anywhere), backward-handle staleness under replace_weights + the self-contradictory LoRA doc (base-only transposes drop the ΔW^T·g term under the merged-weight convention — found independently by both subagents), Phase 2 processing zero-gradient positions (only Phase 1 skips), the ~2.06M-alloc/27GB-per-step + 345,088-round-trips-per-step storms (quantified at real 12B dims), the CPU weight-grad accumulation being the ACTUAL per-step bottleneck (wte grad 17× the largest layer grad; the header's GEMV claim is per-op true, per-step misleading), the 24-line byte-identical CE-backward duplication, the dispatch_attention_only re-run, dual CPU-side KV caches, the RoPE freq-table dual derivation (currently identical — drift = silent gradient corruption), the norm-statistic recomputation, the naive strided transpose at 12B scale, + the fast_tanh header misattribution (GPU inference uses standard tanh — grep-verified).
   Batch 35: 8 riir-gpu production-kernel rules (mask-sentinel-matches-reduction-identity, smem-staging-gated-by-cross-thread-reuse, kernelize-host-chain-after-sync-batching-floor, block-reduction-separate-scratch-runtime-strides, saturating-nonlinearity-recovery-breaks-near-cap, adapter-grad-join-mirrors-forward-delta-insertion-point, side-consumer-readback-alias-live-handle-for-dependent-dispatch, quadratic-attention-intermediates-recompute-from-linear-saves) from Research 039 — the twenty-fourth batch mining the stack's own kernels (the Gemma-2 TRAINING cluster: gemma2_cubecl_train/mod.rs + attention_gpu.rs — the Issue 430 Option B fused causal attention kernel + the hybrid LoRA training forward/backward; the training counterpart of Batch 25's inference-side Gemma cluster; resident.rs 1,639 deferred to a later batch): masked lanes PARTICIPATE in a block reduction by writing the identity element with the sentinel REWRITTEN between reductions of different identity (neg-inf for max, 0.0 before the sum; non-reduction consumers bound at the valid extent), smem staging gated by cross-thread reuse (stage iff within-workgroup reuse > 1 — the single-use K row streams from global in the SAME dot loop), the kernelize-past-the-batching-floor decision (a host-serviced segment floors at 2 syncs after all batching; kernelize when intermediates are chain-local + no host work in the gap — the 52 = 26 layers x 2 decomposition verified post-batching in resident.rs), block reductions into SEPARATE scratch with strides from the runtime extent (both failure modes — in-place + hardcoded unroll — shipped and failed G1 on 4090 before the fix), the saturation boundary of derivative-recovery-from-cached-output (1 - t^2 = exactly 0 once the output rounds to 1.0 in f32; the codebase recomputes the RAW argument at attention but keeps the recovery at the LM head where 9 x cap ~ 270 is unreachable), the adapter-grad-join placement mirroring the forward's additive delta insertion point (additive CPU-side LoRA deltas on GEMV outputs make the base transposes IMMORTAL by construction — no invalidation surface; the merged convention silently drops the delta-W^T g term, Issue 687 H2's class), reading an intermediate for side consumers while aliasing the still-live handle for the dependent dispatch (the re-upload burns ~53 MB/step at seq 256 — the Q4_K sibling is the standing proof), and saving Theta(N x d) attention tensors while recomputing the Theta(N^2) chain in hoisted scratch (the recompute also yields the raw pre-softcap argument the saturating backward needs). Corpus 267 → 275 (A 49, B 45, C 29, D 43, E 17, H 12, L 9, N 16). Standing gates extended (batch35_pseudocode_parses + batch35_rules_present + batch35_concepts_covered; 4 doc-markdown backtick warnings fixed pre-commit; a SIXTEENTH count-pin site surfaced via test failure — the kernel_categories_counts_match_plan_003_table per-category expected-counts table). ONE coordinator seed KILLED by verification ("thread role rotation" is ambient fused-kernel structure, not decision content — subagent falsification now 3-for-3 across batches) + ONE seed CORRECTED (the 52 syncs is the POST-batching floor, not pre-batching — that is what makes the floor rule distinct from the existing batching rules). 12 candidates dropped as duplicates/thin (the predicted dedup pressure from the Batch 34 sibling + Batch 25/26/29 neighbors: the grid -> derive-params, GQA/softcap -> B23, the sweeps -> B33, params/launch -> B29, h-recompute -> the B34 helper rule). Mining bug yield filed as riir-ai Issue 688 (15 hazards): the 3-way sync-count doc inconsistency + the stale "8-step unrolled" module doc, ~1.1 GB/step alloc churn from the unused _into variants + the engine doc-lie claiming the GPU path uses them, ~82 MB dead activation slots, the per-launch params buffer (26 device allocs/step for 9 never-changing floats), the 256-constant triplication, un-asserted q_dim/kv_dim derivation, the KV-cache liveness contract, per-layer grad-buffer churn, 262 MB EngineActivations per step, the 2.6x handle-memory doc understatement, the h-recompute ~9.7 G MACs/step, + the sliding-window divergence note.
   Batch 36: 7 riir-gpu production-kernel rules (constant-scale-folded-into-upload-copy, side-branch-operand-residency-matches-mutation-lifetime, activation-checkpoint-as-handle-clone-deferred-batch-readback, training-forward-shed-decode-loop-shape, sibling-dispatches-before-any-readback, independent-cpu-work-fills-dispatch-read-window, grad-terms-merge-before-shared-link-jacobian) from Research 040 — the twenty-fifth batch mining the stack's own kernels (the Gemma-2 RESIDENT training cluster: gemma2_cubecl_train/resident.rs — the Plan 481/482 all-GPU LoRA training path (per-position GEMV forward + batched GEMM forward + batched backward); completes the Batch 35 cluster, gemma2_cubecl_train/ now FULLY MINED): a hyperparameter-derived CONSTANT scale (LoRA alpha/rank) folded into the byte-level upload COPY so no kernel ever applies it (fold-into-copy, never the source — the backward re-derives from the unscaled original), an aux computation riding a GPU hot path keeping ITS OWN operands device-resident at the step boundary (the middle rung between config-lifetime and call-local; the hybrid CPU-LoRA path paid ~70K sync round-trips and ran 4.8x slower than pure CPU), the checkpoint medium during compute being the device-handle clone (a refcount bump that cannot stall) with the tail read_batched collapsing 364 syncs to 26, shedding the inherited DECODE loop shape in the training forward (the incremental KV cache is the structural blocker; make attention whole-sequence — ~55K dispatches -> ~182), queueing ALL independent sibling dispatches BEFORE the FIRST readback (the interleave serializes completely; applies with DIFFERENT input buffers), hoisting read-independent CPU work INSIDE the dispatch->read window (the LoRA param-grad half is the canonical hoistable work — visible-but-unexploited in source), and SUMMING multiple loss terms at a shared nonlinear link's INPUT with ONE Jacobian application (adding a term after the Jacobian is silently WRONG, not just slower). Corpus 275 → 282 (A 49, B 45, C 30, D 48, E 17, H 13, L 9, N 16). Standing gates extended (batch36_pseudocode_parses + batch36_rules_present + batch36_concepts_covered; 7 doc-markdown backtick warnings fixed pre-commit). FOUR coordinator seeds falsified by verification — the highest count yet (F4 the per-position path already caches its cos/sin table via RopeCosSinCache + Batch 17 had explicitly dropped cos/sin-table rules; F5 the env-var A/B gate is GOAT-gate process, its kernel content already in the B35 floor rule; B2 the ping-pong swap is needless-swap-temp fix(2)'s loop-carried scratch; B4 the scratch hoist is a 3-rule composition) + 2 premise corrections (F3 the per-position path's "ONLY sync point" doc-lie — 93,440 unbatched reads; B1 q/k/v consume different inputs so fan-out's premise doesn't apply). 1 weak candidate dropped (phase-disjoint buffer aliasing — 9 KB local win). Mining bug yield filed as riir-ai Issue 689 (15 hazards): the ~10.2M-transcendental CPU RoPE recompute per forward from not saving k_post_rope (P2, cheapest fix — one handle clone), the 93,440-read unbatched legacy readback + its doc-lie, the 262 MB g_logits_all fresh alloc + dead zero-fill per backward step, the comment-lie g_attn_out_vecs per-layer allocations (26 x seq_len Vecs/step), the nested<->flat reshape round-trip ~327 MB/step, read_handle fresh-Vec ~1.2 GB/step download churn (the read_handle_into fix substrate already in-tree at kimi_k3), missed CPU/GPU overlap windows, the rmsnorm scratch->dest copy API asymmetry, + 7 more.
   Batch 37: 8 riir-gpu production-kernel rules (kv-window-device-resident-ring-append, lm-head-loss-positions-only, verbatim-upload-on-format-match, stream-drain-free-source-during-build, identical-tensor-handle-clone-zero-reupload, sparse-gather-scale-at-read-not-table-prepass, tiled-host-transpose-before-quantize, materialized-fast-path-bulk-slice-accessor) from Research 041 — the twenty-sixth batch mining the stack's own kernels (the Q4_K TRAINING cluster: gemma4_q4k_train/mod.rs + weight_buffers.rs — the QLoRA frozen-Q4_K-base + f32-LoRA-delta Gemma-4 trainer; forward_batched.rs + backward.rs deliberately deferred to a future batch on predicted dedup pressure): a KV cache mutated by append + consumed every position lives ON the device as a persistent ring (only the NEW row crosses the bus — append-vs-snapshot transfer; the sliding path re-uploads ~40 GB/step), the loss-support set (pos >= n_in-1) gating the vocab-head GEMV (prompt positions' lm_head + readback + save are provably dead via the backward's d_logits.is_empty() skip — ~127 MB/step dead), on-disk quant format == device target format → cast the mmap + upload raw blocks (NEVER dequant-to-f32-and-requantize — the 4090 OOM class; proposed independently by BOTH agents, merged on collision), a one-time build draining the source per unit (peak host memory O(unit), not O(model) — the build that lets a 32 GB box build 12B GPU buffers), a tensor legally absent from the export because identical to a sibling → clone the device handle (refcount bump, zero re-upload — the GGUF V-shares-K case), a sparse read set + shared unscaled consumer selecting the GATHER as the scale fold point (the deliberately INVERSE regime of B36's upload-copy fold — encodes the fold-point DECISION), cache-blocked host transpose (naive inner-c store strides rows*4B = 15 KB — every store a cold line, each 64B dest line written 16x), and a bulk Option<&[f32]> accessor branching ONCE per step (the materialized arm runs direct-index + rayon, not per-row enum dispatch). Corpus 282 → 290 (A 52, C 31, D 50, L 10, N 17 — note sparse-gather landed in C beside its inverse twin constant-scale-folded-into-upload-copy, not the proposed B). SIX seeds falsified by verification (highest-per-agent yet: F1 rope tables ARE cached once — 48 layers share 2 via rope_table_idx; F3 LoraSite::none() is the correct frozen-base skip, not B31's inert adapter; F4 gammas built once at construction; F5 the 3 backward constructors field-identical — no merkle_root-class miss; F4' the setup path is pure enqueue, 0 round-trips; F5' the GEMV path already ships packed Q4_K + in-kernel dequant = the B15 direction) + 2 confirmations with regime corrections that became rules (scale-at-gather is CORRECT for a sparse read set; quantize runs once but is paid TWICE: forward + transposed backward ≈ 14 GB on a 24 GB 4090). 1 rule dropped as dedup (variant-dedup-constant-table-by-index — share-pipeline-set-across-subsystems + B17's rope-table-drop precedent). Mining bug yield filed as riir-ai Issue 690 (~19 hazards): 43,008 blocking reads/step on the unbatched path (the file's own Bench 433 doc), the dead lm_head for prompt positions, the dual-KV-cache quadruple-write ~402 MB/step, the O(S^2) full-attention window collects ~537 MB/step/layer (rows already contiguous — pure waste), ~40 GB/step sliding-window re-upload, the 2x GPU weight residency, one-time dead zero-inits 7 GB + 48 GB, two doc-lies, + more, with a 6-step fix order. GOAT: 327 kernel_heal + 201 default tests, clippy 0 in all 4 states (45 doc_lazy_continuation + 8 backtick warnings fixed pre-commit — root cause: line-initial "+ " in //! list-item continuations parses as a NEW markdown list item, flagging every subsequent line; plus the B36 gotcha again: bare lm_head/Q4_K/QLoRA/RoPE/.rs filenames need backticks). Generator lesson: TWO slice-anchor misplacements caught by the count-pin test (the B/C and L/M confusion — slice-tail anchors are only safe when extracted from the enclosing KernelCategory::X block, never by line-number proximity); ONE malformed edit_file op (schema-foreign "replace" key) silently truncated a header sentence — caught + restored; a git stash near-miss (self-caught, pop verified complete — the correct comparison is git show HEAD:file, never stash).
   Batch 38: 4 riir-gpu production-kernel rules (env-flag-resolved-once-not-per-dispatch, flat-activation-slots-as-readback-target, transient-spike-pins-persistent-scratch, transposed-consumption-accumulate-not-gather) from Research 042 — the twenty-seventh batch mining the stack's own kernels (the Q4_K TRAINING cluster, second half: gemma4_q4k_train/forward_batched.rs + backward.rs — the layer-major batched forward, the 43,008 → 337 dispatch fix, + the per-position and batched backwards; the cluster is now FULLY MINED): std::env::var resolved ONCE at construction or behind a cheap guard, never per dispatch/layer/token (97 lookups/step in one trainer; one instance IN FRONT of the autotuner's deliberately lock-free papaya cache hit — the env lock poisons the exact property the cache was chosen for; the correct live-toggle form exists at gemma2_cubecl/mod.rs L1353-1358: cheap guard BEFORE the env read), nested [pos] → Vec<f32> activation slots force a scratch + scatter two-copy tail after every batched readback (flatten the container to [seq × dim] so the slot IS the readback target — the container layout IS the copy count; distinct from preallocated-activation-slots which retains the nesting and therefore the scatter), a one-shot resize(batch × vocab) ≈ 134 MB pinning a persistent RefCell scratch at ~8× its recurring max (grow-only is right for RECURRING sizes and wrong for a non-recurring spike — transient oversized phases use a local buffer dropped at phase end), and z = Bᵀ·g against row-major B via i-outer row accumulation instead of k-outer column gather at stride rank × 4B (up to 16× read amplification; the forward/backward asymmetry on the SAME buffer is the tell; summation order changes — fine for grads, NOT for bit-identity gates). Corpus 290 → 294 (A 55, D 51). ONE subagent rule KILLED by coordinator math (zero-gradient-position elision: K/V cross-position coupling re-introduces non-zero gradient at prompt positions after the topmost layer — support queries attend to prompt keys — so only the topmost layer's pre-attention chain + the head can skip; the SAME flag applies to Issue 687 H3's ~50% magnitude claim, which is ~1/48 — 687 flagged for its owner, not modified). Dedup kills: the backward's per-layer RoPE powf table rebuild (third strike — B17 drop + B36-F4 + B37-F1), sibling-batch region packing, resize-churn, rayon-attention direct-write (a 2nd in-file instance of materialized-fast-path), LoRA-delta add-pass fusion (B19), the RefCell-scratch API shape (ambient in 3 rules). Mining bug yield filed as riir-ai Issue 691 (headline: BOTH backward variants have ZERO in-tree callers — the 941-LOC file is unreached, its math unpinned; then dead x_normed compute in both Phase 1s, 48× per-layer RoPE table rebuilds while the trainer caches bit-identical tables, the ~94 MB/step clone storm, ~380 MB/step fresh output-Vec churn + ~380 MB/step redundant hidden re-uploads (5×/layer, create_from_slice per dispatch — with the Bench-437 nuance that upload-sharing ALONE was never tested), the 134 MB scratch pinning, the dead attn_scores field, the strided LoRA z-gather at 2 sites, 2 doc inconsistencies). GOAT: 330 kernel_heal + 201 default tests, clippy 0 in all 4 states. Generator lesson re-learned: bare json.dumps WITHOUT ensure_ascii=False produced \u2014 escapes in Rust string literals → 33 compile errors (the 32c lesson, now twice-observed — treat ensure_ascii=False as non-optional); a 17th count-pin site surfaced (the stale "282-rule" comment in kernel_categories_cover_all_corpus_rules_exhaustively — B37 missed it).
   Batch 39: 9 riir-gpu production-kernel rules (autotune-cache-extract-outlives-owner, append-cache-rewind-overwrite-in-place, slow-path-bootstraps-raw-buffer-hotswap-cache, entrypoint-variants-share-layer-body, elementwise-offload-needs-device-side-anchor, tiny-param-buffer-rebuild-over-cache, readback-destination-passing-free-fn, variant-owned-output-buffer-clone-escape, pipeline-layer-boundary-device-handle) from Research 043 — the twenty-eighth batch mining the stack's own kernels (kimi_k3_gpu_forward.rs 2,601 LOC — the KimiK3 full-model GPU decode forward: hybrid CPU/GPU stack, 7 forward entry-point variants, dual weight-update paths, MLA KV cache rewind for the Ouro loop, + the dense SiTU FFN helper family; the largest single unmined file): salvage the autotune cache across owner destruction via Arc inject/extract (re-bench ~5-10 min vs upload ~64 ms — the ownership-lifecycle complement to hot-swap), re-execution workloads OVERWRITE the append-only cache via rewind_to(pos) never re-append (O(max_seq) not O(iters × max_seq); recurrent-state layers are structural no-ops), the in-place weight-update two-tier bootstrap (first call runs the slow allocate path + harvests raw wgpu buffers into ORDER-keyed slots; subsequent calls queue.write_buffer in place — training weights change identity every step so pointer-keying is useless), versioned entry points multiply but the per-layer loop body stays shared per axis (forking the whole body guarantees divergence — observed TWICE in one file: a resident fork that initially called the OLD unbatched kernels + two saved forwards whose kernel-variant selection diverged under the same env vars), a GPU op sandwiched between upload+readback is compute-offload ONLY (the win requires a device-side anchor consumer — found INDEPENDENTLY by both mining agents), 3 f32 = 12 bytes rebuilds beat caching below a cache line, GPU readback helpers should be destination-passing free fns (dst: &mut [f32] not -> Vec<f32>; the free-fn form also splits the struct borrow — the API-level half of the Issue-689 churn fix), enum-variant-owned output buffers force per-layer-per-token clones out of the match arm (hoist to the parent + free fns over disjoint fields; merged from both agents — the free-fn extraction alone does NOT fix the clone), and the Handle IS the pipeline layer-boundary currency (host materialization becomes a layer-internal detail — later fusion composes without boundary changes). Corpus 294 → 303 (A 60, C 32, D 53, K 5). TWO rules merged from independent two-agent convergence (the anchor rule + the clone-escape rule). 13 dedup kills/falsifications (update_weights = B25 verbatim; OURO_* env reads = the B38 instance; inv_rms double-compute ×6 = B34 forward-side; the 4-buffer drain = B36+B26; the saved clones are checkpoint captures, NOT waste — B34 instance). Mining bug yield filed as riir-ai Issue 692 (14 hazards: the dead 640 MB embed_handle uploaded at construction + retained forever despite the riir-train Issue 402 comment proving it dead; the per-layer-per-token FFN clone/fresh-alloc storm (6+4 sites) across ALL variants; the two saved forwards' ~310-line divergence with LOST OURO env dispatch; contradictory sync-count docs 7→3 vs 3→1; gate+up duplicated GPU upload vs the doc's sharing claim; vocab logits double-copy ×4 sites; + 8 more). GOAT: 333 kernel_heal + 201 default tests, clippy 0 in all 4 states. The O() expressions joined the doc_markdown gotcha list (O(max_seq) needs backticks in //! prose).
   Batch 40: 7 riir-gpu production-kernel rules (host-scratch-mirrors-slot-order-for-coalesced-writes, write-plan-derived-from-cache-slots-not-recomputed, cpu-source-builder-skips-device-roundtrip, expert-loop-two-phase-split-batch-across-iterations, rmsnorm-backward-gamma-scope-correction-term, per-position-scalar-factored-gemv-collapse, foreign-accumulator-bridge-copy-hoist-to-outer-loop) from Research 044 — the twenty-ninth batch mining the stack's own kernels (kimi_k3_gpu_backward.rs 4,332 LOC — the KimiK3 GPU backward: the LARGEST FILE in the crate; hybrid CPU/GPU gradient stack + dense SiTU FFN / MoE / MLA / KDA backwards; the backward half of the Batch 39 forward): the host scratch layout IS the write-call count (size to the SUM of slot lengths + carve with split_at_mut in cache slot order so the batched writer coalesces adjacent device buffers; the anti-instance sits 60 lines from the positives in the same file), the mirrored hand-maintained slot list IS the bug factory (boot pushes slots on handle-presence while the fast path recomputes lengths on handle-AND-source-presence — the guards drift + the only tripwire is a debug_assert compiled out in release, where shifted writes land in WRONG buffers + the tail goes OOB; derive the plan from the cache's own size_bytes), when the host owns the source bytes the *_from_cpu twin deletes the per-matrix read_one syncs entirely (batching is the lesser fix; GPU-source twins are boot-only), a per-iteration helper encapsulating launch→read→CPU→launch→read hides cross-iteration independence (split at the CPU-dependency point + batch each half across ALL experts: 2+2k_r+2 syncs to ~3), in RMSNorm backward gamma multiplies ONLY the direct dy term never the normalization correction (the slip is invisible at near-unity gamma init under percent tolerances — found LIVE WRONG in the production MoE backward while the same file carried the correct form), factor data-dependent per-position scalars out of GEMVs by linearity but keep scaled and unscaled consumers on separate accumulators (the fused form is the live d_attn_w wrong-math bug), and bridging to a foreign crate's whole-struct gradient accumulator via full-state copy-in/out per token is a gradient ping-pong (hoist the bridge to the outer loop). Corpus 303 → 310 (A 64, D 54, H 14, N 18). ONE proposal dropped (rayon parallel transforms — thin vs rust_perf generic guidance). PRUNER root-cause fix shipped (the L2 wrap strategy glued the closing brace into a trailing line-comment producing false parse failures; hardened with boundary newlines — the corpus found a pruner bug). Mining bug yield filed as riir-ai Issue 693 — the biggest of the mining program: TWO coordinator-verified critical wrong-math bugs on the production training path (MLA d_attn_w spurious attention-weight factor L2576-2584 + MoE latent-norm gamma over-application L1537 — masked by near-unity gamma init + loose tolerances + two-wrongs-agree parity), a NaN-VACUOUS parity suite (the LCG test generator leaks random bits into the f32 exponent field — the [2,4) comment is false; both max-statistics silently skip NaN making 4 of 5 tests pass vacuously), one ACTIVELY-FAILING KDA test (diff 4.95e27, empirically observed), the feature in NO CI gate, + 15 more hazards. GOAT: 336 kernel_heal + 201 default tests, clippy 0 in all 4 states (1 doc_markdown fix: 2+2k_r-style math with subscript underscores needs backticks).
   Batch 41: 6 riir-gpu production-kernel rules (concurrent-disjoint-write-buffer-parallelizes-uploads, aliased-slot-allocate-replace-fallback-severs-binding, pool-slot-write-coalescing-owns-only-declared-ranges, pool-reservation-token-outlives-raw-view, gpu-type-bridge-borrows-not-clones, superseded-primitive-machinery-zombie) from Research 045 — the KimiK3 backward SEQUENCE production caller (kimi_k3_gpu_backward_sequence.rs 1,328) + weight_buffer_cache.rs 459 (the write substrate; the ORIGIN of the B39 hotswap-cache pattern); corpus 310 → 316 (A 69, E 18). Issue 693 H1/H2 production routing pinned (both wrong-math bugs inherited via the batched twin); hazards filed as Issue 694 (orphaned-write fallback on cuda_backend — GEMVs read boot weights forever; zombie LM-head ~640 MB/step + ~1.3 GB dead VRAM; Ouro GPU-source update path ~237 read syncs).
   Batch 42: 4 riir-gpu production-kernel rules (refine-loop-invariant-prefix-elision, softmax-entropy-via-shift-identity, runtime-identity-gate-cached-verdict-elides-side-projection, expected-embedding-transposed-gemv-resident-head) from Research 046 — the Gemma-2 D2F block-causal decode cluster (gemma2_d2f/mod.rs 1,579 + gemma2_d2f_sc.rs 800, now FULLY MINED): the denoise loop re-forwards the PROVABLY-invariant prompt prefix every step (masking closes it; mutation-set closure proves it — O(steps × seq) → O(seq + steps × |S|)), ln(e_t) = logit_t − max so the entropy sum needs ONE transcendental not N, content-classified inertness cached write-through at the mutation boundary gates whole-stage elision (the gate must cover producer AND consumer), and the soft-token expectation x̂ = Σ p_v · wte[v,:] factors as the tied lm_head transposed GEMV on the already-resident table (not a 2.36 GB/position/step host mirror walk); corpus 316 → 320 (A 70, B 46, D 56). 18 falsifications (the file is a museum of already-mined anti-patterns). Hazards filed as Issue 695 (KV GPU→CPU→GPU round-trip 1.7 GB/block, invariant-prefix recompute ~227 GMACs, 46.6K blocking reads, hardcoded softcap divergence, truncate panic, the unreachable trained-SC path + the ungated x̂₀ producer).
   Batch 43: 5 riir-gpu production-kernel rules (mutable-input-scratch-residency-write-per-call, topk-select-nth-over-full-vocab-sort, post-scatter-renorm-support-set-only, stack-array-row-view-bridge, empty-slice-sentinel-launders-shape-violation) from Research 047 — the Weaver speculative-decode GPU corrector cluster (weaver_gpu_corrector.rs 1,449 + weaver_gpu_dflash.rs 576, katgpt-rs Plan 436 Phase 4; the weaver_gpu/ kernel dir is a SEPARATE cluster, still unmined): per-iteration create_from_slice input churn → scratch-resident handles written per call via client.write (the file's own batched twin is the positive contrast), O(V log V) full pair sort → select_nth_unstable partition + K-prefix sort over a u32 index buffer (composes with order-stable-topk), dense re-sum + rescale after a K-scatter → sum-during-scatter + support-only rescale (bit-identical by IEEE — +0.0 adds exact, 0×inv = +0.0), Vec<&[f32]> heap collect → a fixed [&[f32]; 64] stack bridge (the speculative-decode idiom at 5+ in-tree sites), and a defensive &[] sentinel type-checks past the shape contract into a swallowed Err (the negative space of launch-unchecked); corpus 320 → 325 (A 73, B 47, D 57). 23 falsifications (riir-ai Issue 468 P0/P2 already shipped several rules' fixes — the per-depth vs batched paths sit side by side as naive/opt contrast). Hazards filed as Issue 697 (the 3-layer silent no-correction chain: undersized capture → &[] row → HiddenShape Err → let-_= swallow; the release-mode embedding OOB window ~2.1 GB from a debug_assert-only length check; ~29 alloc requests/call on the hot path; test-only reachability with no CI row).
   Batch 44: 9 riir-gpu production-kernel rules (row-dot-split-inner-axis-block-reduce, fuse-indirect-gather-into-dot-consumer, share-cube-size-const-between-kernel-and-launcher, stash-kernel-internal-intermediate-in-smem, hoist-bit-address-out-of-packed-row-sweep, fold-tile-transpose-into-dequant-scatter, bit-cursor-unpack-to-width-specialized-byte-batch, single-sweep-minmax-for-quant-scale, serialize-container-canonical-order) from Research 048 — the Weaver GPU KERNEL substrate + the KVarN quantization cluster (weaver_gpu/mod.rs 1,657 — the 19-dispatch kernel implementations behind the Batch 43 corrector: GpuWeaverWeights upload/download, pos-emb add, batched SwiGLU, causal MHA with smem softmax, indirect embedding gather, dot-per-row, softmax-over-K with the riir-ai Issue 468 P0 probs_offset write-shift + kvarn/ — kvarn_zone_weights.rs + kvarn_dequant.rs + kvarn_quant.rs, the game-side CPU reference + zone-scale registry for KVarN KV-cache quantization, post-Issue-042 orchestration over canonical katgpt-kv primitives; Lane B cross-verified every fix-shape against the canonical kv_cache.rs/var_norm.rs): one-thread-per-output-row pins 128 threads to a single workgroup at production + 16× read amplification → split the reduction axis (workgroup per row, lane-strided, block-reduced — the INVERSE regime of grid-over-output-thread-private-sweep), a materialized D*K*h gather whose only consumer re-reads it in a dot → gather in-loop via the device-resident index array, the workgroup-size literal restated in 4 kernels vs 6 launchers is a silent stride contract → one shared const, a global write + re-read nobody outside the kernel observes → smem stash + drop the buffer from the signature, the per-row unpack call re-derives an identical bit address (pos_in_tile invariant) → decompose once per position, materialize-then-transpose = 2 reads + 2 writes per element → scatter the consumer layout at production, a software bit cursor re-derives what a per-width arm makes compile-time-constant → match bits {8/4/2/1} byte-batch arms, two folds scan the group twice for the (lo, hi) pair every RTN scale prep needs → one joint sweep (the canonical var_norm.rs comment documents the identical fix), and HashMap iteration order leaks into serialized bytes breaking BLAKE3 commitment stability → sorted keys; corpus 325 → 334 (A 75, B 49, C 33, D 59, E 19, L 11). 22 falsifications (the file's own documented trade-offs — thread-0 softmax at tiny K, naive-per-head MHA at seq_len ≤ 16 — plus 8 dedup kills + the canonical katgpt-kv positives). Hazards filed as Issue 698 (the legacy KVarN round-trip DROPS the VarN inversion — scales discarded at kvarn_quant.rs L160, masked by vacuous tolerances; the CPU/GPU auto-route doc-lie; the drifted unpack fork corrupting non-pow2 widths; GpuWeaverScratch dead with known-wrong sizing; the dot-per-row perf cliff).
   Batch 45: 3 riir-gpu production-kernel rules (scratch-depth-matches-shape-invariance, softmax-denominator-hoist-out-of-weighted-v, frozen-cooperand-hoist-out-of-perturbation-loop) from Research 050 — the SHINE hypernet meta-LoRA cluster, first half (hypernet/meta_lora.rs 732 — the CPU reference bidirectional forward for memory extraction, Plan 098/104 + hypernet/train_meta_lora.rs 601 — the CPU finite-difference Meta-LoRA trainer): a layer × position × head nest allocates every scratch at the deepest convenient point though all shapes are bound by (seq_len, dim) fixed for the whole call — allocation depth must match shape invariance (the tell: two same-shaped residuals at different depths, attn_residuals per-layer vs mlp_residual per-position; scores alone is L×S×n_head allocations), the softmax-weighted V accumulation divides by the denominator inside the innermost (d × t) nest — scores[t]/sum_exp recomputed head_dim times per key position; hoist inv_sum = 1.0/sum_exp once per (position, head) so the PV loop multiplies never divides (the in-tree optimized twin is riir-engine attention.rs attention_head pass 3 — the file re-implemented it naively one crate away), and an FD/perturbation sweep rebuilds the FROZEN co-operand per candidate while the callee re-derives its own sweep-invariants per eval — materialization cadence must match mutation cadence (build a LossPrelude at sweep entry: frozen mem + context target + pre-converted adapters; complements make-unmake which fixes the VARYING operand); corpus 334 → 337 (A 77, B 50). 9+ falsifications (representation-conversion-clone = the immutable-operand-upload family's CPU twin — mechanism is not decision content; FD unflatten-per-perturbation = make-unmake instance; loop-order swap = ambient matvec-coalesce; per-position GEMV = op-major-loop-inversion + deliberate CPU reference; powi already hoisted; FD O(P) structure = documented trade-off). Hazards filed as riir-ai Issue 699 (lora_buf sized by model_config.lora_rank vs adapter rank 128 → silent get_unchecked_mut OOB with real configs' lora_rank: 0, masked by a 4==4 test coincidence; GQA kvd < out_dim slice OOB; apply_meta_lora dead-but-public + 16× scaling + same-bytes-transposed A/B layout vs the production path; FD clone storm quantified 1.4 MB/step test scale, ~636 GB/step extrapolated; baseline ctx-8 hardcode + single-sample GOAT estimator).
   Batch 46: 3 riir-gpu production-kernel rules (size-gate-covers-every-cost-factor, declared-max-array-taxed-per-thread-under-runtime-bound, shared-operand-side-pair-batch-index-decomposition) from Research 051 — the MaxSim cluster (maxsim/dispatch.rs 449 + spectralquant.rs 581 + both WGSL kernels + the Benchmark 002 crossover bench; size-gated CPU/GPU MaxSim late-interaction scoring Plan 085 + the fused SpectralQuant dequantize+MaxSim single-pass scorer T5): a CPU/GPU size gate reading a partial work product ld*batch while CPU cost scales with lq*ld*dim*batch silently transposes the crossover at unbenched shapes (the pinned 256 bakes in BOTH implementations' current overheads — two in-tree instances incl. gpu_m2p rows*num_heads), a WGSL function-scope array sized to a hard max array<f32,256> but runtime-bounded by d_eff pays the DECLARED max in indexable thread-local per thread (fix: pipeline override constants, one pipeline per bound class), and a [batch x lq x d_h] pair API bakes per-pair replication of the shared query side into the batch geometry (decompose the launch index (query_idx, doc_idx) — share-by-indexing, not replication; the CPU substrate twin maxsim_score_packed already does it). 14 falsifications incl. the d_eff unpack-bounds hypothesis KILLED by kernel read (unpack bound IS d_eff — tail dims never unpacked). Hazards filed as riir-ai Issue 700 (validate() never checks compressed_k; the documented-but-unenforced +1-padding one-word OOB read executing in tests; d_eff bounds unchecked anywhere; the FALSE no-katgpt-dep premise hiding katgpt_core::simd::maxsim_score one feature-forward away; the bench's own crossover finder warning against the shipped constant; the 800K-pair doc example ~133 GB infeasible; zero production callers). GOAT: 354 kernel_heal + 201 default, clippy 0 both states (15 doc fixes — the B37 line-initial + RECURRED). Generator NEW lesson: Python evaluation-order truncation — open(p,"w").write(apply(...)) truncates at open-time BEFORE apply() runs (live-catched, restored from HEAD; compute content first, then open).
   Batch 47: 3 riir-gpu production-kernel rules (per-key-projection-hoist-out-of-query-sweep, multi-head-out-proj-consumes-head-concat, shared-producer-prefix-extract-once-per-input) from Research 052 — the SHINE hypernet SECOND half (hypernet/shine.rs 402 + routing.rs 268 + types.rs 269 + m2p_transformer.rs 611 + gpu_m2p.rs 568 + kernels/m2p_attention.wgsl 173 — the end-to-end context-to-LoRA pipeline + expert routing + the M2P alternating-attention CPU reference + its size-gated GPU dispatch twin; completes the hypernet module alongside Batch 45's meta-LoRA pair; meta_lora/train_meta_lora were ACTIVE sibling WIP — quotes drawn only from clean files): K/V projections re-derived inside the query loop multiply projection work by seq (O(seq*H^2) -> O(seq^2*H^2); the optimized twin projects ALL QKV once at sweep entry in the same module family; row attention at default M=425,984 is arithmetically infeasible), the attention out-projection consumes the CONCATENATION of all heads' outputs — a per-head thread degenerates to scalar-times-rowsum (attn_val times the summed wo row) silently, masked by identity-weight fixtures whose rows all sum to 1.0 (two-phase fix: per-head scratch + full-row projection), and two sibling methods sharing an expensive pure producer (extract_memory_states — a full frozen-backbone forward) pay it twice under one composed call (context_to_lora_with_span_width — extract once, thread the artifact via *_from_grid variants); corpus 340 -> 343 (B 51, D 61, E 20). ~17 dedups (workgroup(1,1,1) = B44/B23/B28 composite; per-column weight re-upload = B33/B28/B26/B40; the size gate = B46's pre-recorded 2nd instance; Vec churn = B45 depth; powf freq table = 3rd-strike cos/sin; routing norms = norm-decoupled; shape metadata = derive-shape-from-payload). Hazards filed as riir-ai Issue 701 (the WRONG out-projection math in both WGSL kernels AND the CPU fallback + the print-only pseudo-parity check comparing the two wrong arms; release-mode stack OOB at default head_dim 512 vs the [f32; 128] array; WGSL array<f32, 256> scores overflow at seq > 256; the ~251 GB default memory grid + the TRIPLE-inconsistent 13/15/19H provision constant with 99.96% of the grid discarded at default; structurally-dead all-zero LoRA modules at toy scale via the min() clamp; row-attention dispatch exceeding the 65535 workgroup limit; zero production callers — bench-only). GOAT: 357 kernel_heal + 201 default, clippy 0 both states (10 doc fixes: the B37 line-initial + struck a 4TH time + 7 pre-existing KERNEL_CATEGORIES slice-header backticks from the B46 split — whole-crate clippy after any corpus-layout change is the standing check). The 2-lane split (compute pair vs pipeline/routing) resolved cleanly; both lanes' quotes re-verified by the coordinator in clean files.
   Batch 49: 1 riir-gpu production-kernel rule (runtime-arg-loop-bound-demotes-arrays-to-local-memory) from Research 055 — the Bench-686 measured-lesson distillation (NOT file mining: riir-ai Issue 706 / Bench 686, the rmsnorm_quantize_f32 single-pass + register-unroll fix on the 4090 cudarc decode path): a correctly-sized per-thread array whose per-element loops are bounded by a RUNTIME kernel arg (constant at every call site — a LAUNCHER fact the kernel cannot see; the host passes a Rust const as an int arg) is demoted to local memory — the compiler cannot unroll a runtime-bounded loop and a dynamically-indexed per-thread array has no register form, so every access is a local-memory (spill) round-trip and every array the loop touches inherits the demotion; fix = hoist the bound into the kernel as a compile-time constant + #pragma unroll (statically-indexed arrays promote to registers; the ascending accumulation order is preserved so outputs stay bit-identical; keep the (i < len) tail-guard for partial tiles); the pathology is INVISIBLE by default — it hides under existing global traffic and surfaces only when a neighboring change adds same-address-space traffic: the naive v1 (gamma prefetch only) measured 0.6-1% SLOWER in paired interleaved A/B (trading L1-hot global reads for more local-memory traffic is a LOSS); evidence: kernel −18.4% (nsys 5,460.9 → 4,455.7 ns avg, 20,702 launches, same-code-drift controlled), ppl bit-exact 5.4681/3.7778/0.6909; distinct from B46 declared-max-array-taxed-per-thread-under-runtime-bound on three axes (SIZE-declared-max footprint tax vs BOUND-driven per-access local addressing; oversized vs exactly-sized array; pipeline-override size specialization vs const-bound hoist + unroll); corpus 343 → 344 (A 80). GOAT: 574 kernel_heal + 415 default + 9 retrieval-gate tests (floors held), clippy 0 both states, all-but-ternary combo check passes — the full CI guard layers 3/4/5 on the 4090 box are blocked by the PRE-EXISTING un-gated metal dep in riir-gpu (diagnosed via cargo tree -i core-foundation; filed as riir-ai Issue 707 with the one-line fix; RESOLVED 2026-08-16 - M3 T2 verified metal_tensor_gemm + bench_663 interop green, riir-ai c15cb947d). 4090 toolchain note: the repo pin resolves to the GNU host whose mingw linker is absent — every build on this box needs cargo +1.97.0-x86_64-pc-windows-msvc.
   Batch 50: 0 rules — the file-mining ZERO-RULE batch (Research 056; 8 files across 2 lanes: small leaves game_mux/domain_latent/context/cpu_reference/depth_tier/elf + standalone twins forward_dflash/forward_flashprefill + 5 WGSL kernels verified). Every candidate was a positive instance (context.rs IS B21 construction-side, domain_latent.rs IS B33, elf's sigmoid IS B32a, flashprefill mean_k IS B35-applied) or an ANTI-instance (forward_dflash's stale uniforms are B26's live ref_shape — the SAME bug class Batch 27 filed in backward_dflash, forward twin; the two-pass softmax is popcorn online-softmax-streaming's ref_shape; the one-thread-per-row reductions are B44's) — 28 total drop-table rows across both lanes. Corpus stays 344. The batch's VALUE is the hazard yield: 31 hazards filed as riir-ai Issues 708 (forward_dflash is an UNFINISHED MVP presenting as a forward pass — two no-op stubs (rmsnorm L1027-1047 + attention L1212-1225, comments admit) + two live stale-uniform wrong-math classes (uniform_qkv.pos = seq_len-1 for ALL Phase-A dispatches via per-call queue.write_buffer + one shared uniform_matmul across wo/w1/w2/lm_head) + tests asserting only nonzero+finite on garbage logits, feature-gated dflash_training opt-in) + 709 (game_mux demux math UNJUSTIFIED — softmax after the weighted sum + non-negative hash distributions → h_i·h_j > 0 cross-contamination, with a live riir-games consumer + is_finite()-only tests; cpu_reference f64 doc-lie (all f32 accumulators) + lora_forward doc omits lora_b.wgsl's BASE GEMV (parity trap); elf rms_norm is L2-normalize mislabeled (sqrt(d) scale factor, collides with cpu_reference's true RMSNorm in-crate) + elf embedding-SDAR is a silent zero-loss stub at its riir-train consumer). Coordinator verified all 9 load-bearing claims against source before filing. Recommendation encoded in the queue: the kernel file-mining program approaches its natural end — next (final) file batch = kimi_k3_gpu_backward_batched + attention_q8kv_cubecl (expect 0-2 rules), then declare the riir-gpu corpus COMPLETE and shift to rust_perf + issue-driven work.
   Batch 51: 3 riir-gpu production-kernel rules (batched-read-slot-membership-decided-at-launch D, unpack-loop-indexed-by-container-word A, dequant-placement-by-component-traffic-share N) from Research 057 — the FINAL riir-gpu file-mining batch (kimi_k3_gpu_backward_batched.rs 1,398 + attention_q8kv_cubecl.rs 967; corpus 344 → 347, A 81 / D 62 / N 19; the riir-gpu corpus is now COMPLETE — ~115 files mined, the remainder is the sibling-blocked ternary_deltanet cluster + tests + thin leaves judged out-of-scope at B50). Rule 1: a batched readback whose push site guards on a local Option PAIR while the extraction site guards on the config flag that merely IMPLIES it drifts when flag=true + handle absent (constructible through the real builder) — every slot shifts by one + guaranteed OOB at the tail; a READ list has NO persistent state to derive from (unlike B40's write-side derive-from-cache), so the fix is decide-once-at-launch or a recorded tag vec (the file's own dead sync2_layout is the unwired fix; its t6_meta is the wired positive). Rule 2: a byte-packed unpack loop indexed by the LOGICAL element re-loads the same u32 4× + runtime div/mod + runtime shift per element — index by the CONTAINER word (one load serves 4, shifts become immediates); honest scope: L1-hit loads, win = L1 bandwidth + issue slots (distinct from B44 bit-cursor width-dispatch + B44 invariant-hoist + popcorn pack-side cousin). Rule 3: a quantized persistent operand splits dequant placement PER COMPONENT by traffic share — VALUES stay packed + dequant in registers (4:1 bytes × every-step reads; within-workgroup reuse 1 so B14's tile-staging doesn't apply), SCALES pre-decode f16→f32 at upload (sidecar is 1/16 the traffic); encodes the tension with B31 (CORRECT for the sidecar, WRONG for the payload — normalizing the whole operand at upload is the naive shape). Coordinator verification REFUTED Lane B's orphaned-module CRITICAL (the module IS wired: lib.rs L895-898 cfg cubecl_runtime + 3 production launch sites in gemma2_cubecl/dispatch.rs) — the subagent's repo-wide grep silently failed on the multi-root workspace; every CRITICAL reachability claim gets a coordinator grep now (Research 057 lesson 1). Issue 693 closeout verified: H1/H2 fixes PRESENT in both twins + the LCG generator fixed at all 3 sites + the stale B40 corpus note refreshed (entries_gpu_05.rs). Bug yield filed as riir-ai Issue 710 (15 hazards: the sync2 guard drift w/ OOB panic path, the 9-site MLA CPU weight-grad backlog capping Issue 586's conversion, the t-invariant k_c recompute, the production-wired q8kv multi-tile path with ZERO test coverage at n_positions > 256, dual n_positions source of truth, + 10 more). GOAT: 577 kernel_heal + 415 default + 9 retrieval tests (kernel 100/100/100 self-variants, blended 77.8/87.3/92.9 — ALL above the Issue 022 floors), clippy 0 both states (2 doc-markdown backtick fixes — the .rs-filenames-in-//! gotcha, 5th recurrence), all-but-ternary combo clean. Generator lessons: category-membership anchor verified by ROW INSPECTION not rule-text memory (rademacher-signs lives in B, not N — first pass mis-filed dequant-placement into B; caught by row-span check), concepts-covered case-sensitivity struck a 4th time (NO vs no). The idle-loop mining queue shifts to rust_perf + issue-driven work; kernel_opt stays open for measured-GOAT distillations (the B49 path) + the ternary_deltanet sibling-WIP unlock.
   Batch 52: 1 rule (phase-split-per-position-data-liveness-row-storage, D) from Research 059 - the first measured-GOAT distillation AFTER corpus-complete (the B49 reopen path (c), same-day): distilled from the riir-ai Issue 708 H4 fix where the dflash forward's Phase-A project_q wrote a single-row [n_embd] q buffer per position so Phase-B bidirectional attention could only ever see the LAST position's Q - invisible while the consumer was a no-op stub, and NOT a stale-uniform bug (the right value was NEVER STORED; distinct from B26/B27 wrong-VALUES). Fix shape: per-position row storage ([seq_len x dim] q_cache + row-pos writes riding the existing pos-in-uniform + backward reading row pos). The fix itself landed as riir-ai `42b564759` (dflash forward FUNCTIONAL: per-pos uniform slots H1, per-site matmul uniforms H2, real gamma-less rmsnorm H3, real bidirectional concat attention H4 + q_cache/attn_out_cache, H6/H7/H8/H11 dead-work deletions, 3 backward tests un-ignored; G1 CPU-reference parity max_rel=5e-6, 565/565 feature tests). Corpus 347 -> 348 (A 81 / D 63 / N 19). GOAT: 600 kernel_heal + 135 corpus tests (3 new batch52 gates), clippy 0 both states (2 doc_markdown fixes - the [x y]-bracket family needs backticks in //! prose), retrieval floors HELD (kernel 100/100/100, blended 95.2/97.6/97.6). Landing lesson: multi-line doc-bullet anchors must match the FULL bullet, not its first line (the B52 header initially split B51's bullet - the same interleaving quirk B49/B51 already exhibit in that header, pre-existing).
   Batch 54: 1 rule (regularizer-above-noise-floor-coupled-to-damping-cap, B) from Research 061 - the SECOND measured-GOAT distillation after corpus-complete and the FIRST from riir-train (the Plan 339 T5.1 handoff's strongest candidate, pre-verdicted so refuted ones wouldn't be mined): a reduced-precision matrix-polynomial iteration (NS / inverse-PSD-sqrt over a TF32 Gram) on rank-deficient input gets spurious null-space eigenvalues from SIGNED per-GEMM format noise (~1e-3 TF32), not from the eps regulariser (cond~1e12 decay is CLEAN - exact/near rank deficiency with O(1) columns is the trigger). Three coupled facts, none sufficient alone (Bench 470 decision table, 26 adversarial cases): (1) eps must sit ABOVE the format's noise floor - eps=1e-5 is 100x below TF32 GEMM noise, "a regulariser quieter than the thing it regularises"; (2) eps is capped by the damping budget (normalisation makes lambda_max = 1+eps; gamma budgets exactly gamma-1 of overshoot past the polynomial's basin [0,1] - the coupling eps <= gamma-1 must be ASSERTED in code, 10x the budget diverges to sigma_max ~1e24 on the CONTROL); (3) raising eps alone shrinks singular values at high d_min (0.36) - the repair is a second FULL-coefficient-schedule pass with the Gram re-formed from the DATA iterate (after pass 1 S ~= I so eps is no longer large vs the spectrum; rank-1 lambda_min/lambda_max -1.9e-1 -> -1.1e-9). Measured: shipped 16/26; restart alone 14/26 (WORSE - a fresh Gram gets fresh signed noise); eps=1e-3 alone 26/26 but convergence 0.3639; eps=1e-3 + 2 full passes 26/26 AND production 0.9587 > 0.9454. Two refuted variants encoded INSIDE the rule as decision content: Gram-domain restart P = X*P0*X (algebraically identical, no extra tall GEMMs - but 4 digits of cancellation in TF32's 3; broke the control 16/26 -> 6/26) + partial-schedule restart (re-applies the aggressive a=7.42 prefix to an already-orthogonal spectrum -> fixed point sigma ~= 1.7 that more passes don't improve). Guard corollary: `t < 1e-20` is FALSE for NaN - an overflowed Gram sails through and poisons the weights permanently; write `!t.is_finite() || t < threshold` (landed unconditionally in riir-train). Corpus 348 -> 349 (B 51 -> 52). GOAT: 735 kernel_heal + 549 default lib, clippy 0 both states, retrieval floors HELD (clippy top-1 89.6 vs 87.5 floor; 13 retrieval tests green). Also fixed the B49/B51/B52 header interleave in corpus/mod.rs (the orphaned B49 content rejoined to its stub - span-marker replacement, the recommended method for that header).

   Batch 55: 2 rules from Research 062 - the THIRD measured-GOAT distillation after corpus-complete, from riir-ai Issue 716 / Bench 691 (q8kv sink guard, landed `e794b0033` the same day) + the Bench 642 / Issue 639 re-hit: (1) `outlier-span-escapes-quantization-via-lossless-sidecar` (N 19 -> 20) - 1-2 massive-activation channels (x100/x300 on sink pos 0 + delimiter) collapse non-outlier NEIGHBORS in the same 32-block 58x (attention output error 584x/18,844x over uniform, concentrated at the poisoned blocks 88x/299x); the rows are KNOWN A PRIORI (attention sinks = oldest fixed positions, hybrid schedule boundaries from config - no detection), so the known span escapes quantization into a LOSSLESS f32 sidecar ([keys || values]) while the q8 buffers keep the rows as DEAD DATA so strides never shift; kernel derives row count from the sidecar allocation length / stride; A/B: sidecar BEATS Hadamard rotation on BOTH axes (rotation spreads MA to absmax ~MA/16, neighbors regain only ~8x resolution - nonzero - at strictly larger kernel cost; sidecar exact on the guard span 4.781 -> 0.000698 = 6,850x, full sidecar == f32 kernel at 0.00000000) at 0.96x perf (f32 reads replace byte-dequant loops); S=0 bit-identical to the unguarded launch. (2) `buffer-length-derived-from-allocation-not-binding` (D 63 -> 64) - a TWO-OCCURRENCE root-caused class: cubecl's `KernelLauncher::process_buffer` derives the kernel-visible length from `handle.size_in_used()` (the ALLOCATION), NOT the bound `BufferArg` length (validation-only in the checked path; `launch_unchecked` never sees it); over-length half (Bench 642: `let dim = input.len()` made rmsnorm reduce x^2 over FOLLOWING rows - fix = both-ends binding trims `offset_start(start).offset_end(tail)`, filed class-wide as Issue 639) + zero-length-over-a-real-handle half (Bench 691: `from_raw_parts(query_handle.clone(), 0)` derived sink_rows=1 from the 2048-f32 allocation, silently reading the QUERY buffer as position-0 K/V, uniform error 0.000268 -> 0.12 - fix = a REAL 4-byte allocation that floor-divides to the semantic zero, 1 f32 -> 1/2/1024 = 0; contract is EXACT-sized, oversizing CHANGES semantics); probe corollary: a benign-case-only probe (the LAST row where remaining allocation coincidentally equals dim) is WORSE than no probe - it converted an open question into a wrong answer. Corpus 349 -> 351 (N 19 -> 20, D 63 -> 64). GOAT: 738 kernel_heal + 549 default lib + 163 corpus tests, clippy 0 both states (2 doc_markdown backtick fixes - `handle.size_in_used()` + `BufferArg` in `//!` header prose), retrieval floors HELD with ZERO decay (kernel self-variants 100/100/100, blended 94.6/97.7/97.7 of 130 - identical to the Batch 53 re-pin). Record: Research 062.

   Batch 56: 2 rules from Research 063 - the FOURTH measured-GOAT distillation after corpus-complete, from riir-ai Issues 719 + 714/720 (both resolved same-day on the 4090 box): (1) `concurrent-context-init-needs-process-wide-lock` (D 64 -> 65) - the init-outside-publish-on-success pattern does NOT serialize construction (publication is atomic, construction is not): every first-caller that misses the fast path runs the full driver-level sequence (Instance/Adapter/Device + CubeCL server registration) concurrently, which wedges NON-DETERMINISTICALLY at the driver level on 4090/Windows/Vulkan (7-thread race: 4 wedged forever inside `request_device`, another run 1/7 inside `init_device`; stuck point MOVES between runs; full lib suite froze at 101/604 with every worker blocked behind first-call init) AND every loser leaks its device + server for process lifetime (no deregistration - the partial reintroduction of the Issue-714 46-device leak); the CROSS-PATH half is load-bearing: multiple constructors each with their OWN OnceLock run the SAME sequence and race each other - ONE process-wide init Mutex guards all of them, acquired only after the fast-path cache read (steady state = one atomic load), DOUBLE-CHECK after acquiring, release before Err (failure not cached), poison de-fanged; regression gate = same-path + cross-path race tests; post-fix 605/0/2 in 1664 s - the first green parallel-mode full suite on the box. (2) `superseded-format-constructor-live-fields-only` (A 81 -> 82) - a constructor accepting an OPTIONAL second weight format while unconditionally uploading the primary f32 set uploads DEAD WEIGHT (the WGSL Q4_K ctors: ~9.7 GiB f32 dead on ~1.4 GiB Q4_K, 14,089 -> ~3,500 MiB peak; the cubecl half: 23.8 GB peak on 24 GB); the dead-set proof is a DISPATCH-SITE audit (every bind of the primary buffers sits inside the superseded format's match arms), never the ctor alone; fix = tiered live-fields-only variants (`skipped()` / `norms_only()`) with zero-size placeholder TRIPWIRES (accidental bind fails validation LOUDLY, not silently-garbage; raw wgpu accepts size 0 - B31's minimum-legal-footprint applies to substrates that validate non-empty), wired INSIDE the shared ctor (`new_internal` when `weights_q4k.is_some()`) so callers cannot forget; `forward_layer_by_layer` REFUSES Q4_K rather than silently mixing; shipped TWICE back-to-back (714 cubecl half-skip, 720 the WGSL Q4_K ctors) + the Issue-690 2x-residency hazard - a ctor-format pair is the trigger; measured Bench 692: LOGITS_HASH bit-identical, -10.3 GiB device + -2.2 GiB host. Corpus 351 -> 353 (A 81 -> 82, D 64 -> 65). GOAT: 760 kernel_heal + 568 default lib + 166 corpus tests, clippy 0 both states (6 doc_markdown backtick fixes - `request_device`/`init_device`/`Q4_K`/`skipped()`/`norms_only()` in the new `//!` header block), retrieval floors HELD with zero decay (blend 94.6/97.7 identical to the B53 re-pin). Generator: all anchors unique on the FIRST dry run (second consecutive); entries_gpu_06.rs now ~950 lines - Batch 57 opens entries_gpu_07.rs. Record: Research 063.

   Batch 57: 2 rules from Research 064 - the FIFTH measured-GOAT distillation after corpus-complete, from riir-ai Issue 715 (resolved+removed 2026-08-17/18, commits 9f9e68402 + 93d853582; the fix commits predate B56's queue scan but were never evaluated by it - found by this session's git-log re-scan): (1) `smem-slot-role-reversal-needs-barrier` (A 82 -> 83) - a shared-memory scratch whose slot ROLES reverse between phases needs a barrier at each reversal; the two canonical windows in an online-softmax tile loop: (a) intra-iteration - every thread reads the reduction result smem[0], then thread 0 overwrites that slot with its weight (a slow warp reads a weight as the tile max; threads disagree on the softmax shift; length-independent) and (b) loop back-edge - the last phase reads every smem[p] while the next tile's Phase 1 writes smem[tid] (reachable only n_tiles >= 2); measured 27B / 256-token probe, same binary back-to-back: NONDETERMINISTIC before (first divergent token 71), BIT-IDENTICAL (3/3) after; both barriers in BLOCK-UNIFORM control flow (the early returns are uniform - no divergent-barrier deadlock); five kernels shipped the races (3 CubeCL + 2 cudarc CUDA twins) - the CubeCL fix did NOT reach the cudarc arm (it dispatches hand-written CUDA; grepping one source tree proves nothing about which kernels a given arm dispatches); recurring class: Issue 612's smem-corruption fix removed the destructive reduction but left both windows open; the corpus's entire prior barrier content is perf-side (eliminate/fuse) - this is the correctness complement marking the two reversals that can NEVER be removed. (2) `tile-loop-back-edge-needs-multi-tile-bitexact-gate` (D 65 -> 66) - a loop that trips twice only past a size threshold hides every back-edge defect from fixtures that fit inside ONE iteration: every prior decode test used n_positions <= 200 < the 256-wide tile, so the back-edge never ran and the race was untestable BY CONSTRUCTION (tests passed because the code under test never ran); the gate needs BOTH halves: a fixture crossing the boundary (n_positions=300 -> n_tiles=2) AND a bit-exact reproducibility assert over repeats of the SAME binary (a tolerance-only assert cannot see a race in the low mantissa bits - two runs agreeing to 1e-5 still differ bitwise); validated BOTH directions: fails run 0 with max_diff=0.0586 barriers-removed, passes with them - a gate that cannot fail proves nothing; second instance of the fixture-population class: Issue 710 H10 (q8kv multi-tile cross-tile rescale + tail-tile guard, zero coverage at n_positions > 256 until a 2-tile fixture was written). Dropped with reasons: the RoPE table-dim defect (3rd-strike rope territory; driver-contract not kernel-shaped; pos-0 identity noted as a tell), the reduction-tree copy-paste slips (typo class), the twin-propagation as standalone (folded into rule 1; B39 carries the structural half). Corpus 353 -> 355. GOAT: 766/0/1 kernel_heal + 571/0/0 default lib, clippy 0 both states (zero post-write fixes - all //! backticks pre-emptive), retrieval floors HELD with zero decay (kernel 100/100/100, blend 94.6/97.7 identical to B53/B55/B56). Opens entries_gpu_07.rs per the B56 handoff. Count-baseline note: B56's recorded 760/568 predate the sibling's c85ff1c fixtures (3 tests via the d8e614d rebase); true HEAD baseline was 763/571. Generator: all anchors unique on the FIRST dry run (third consecutive clean landing). Record: Research 064.



   Batch 58: 8 rules from Research 065 - the ternary_deltanet sibling-WIP unlock (reopen trigger (a)), the LAST never-mined riir-gpu orchestration file: `ternary_deltanet_gpu_forward_cudarc.rs` (5,691 lines, the raw-CUDA decode-forward twin; unblocked when the Issue 717 speculative seam + Issue 715 barrier fixes LANDED - tree clean, file committed 08-18 11:30; the CubeCL twin `ternary_deltanet_gpu_forward.rs` STAYS BLOCKED on open Issue 721 T3, which rewrites its chunked-recurrence kernels - and 721's Non-goals exclude the CUDA port, so the cudarc twin was collision-free). Heavy two-lane convergence: 4 mechanisms found independently by both subagents, merged by the coordinator. The 8 rules: `async-htod-host-source-outlives-copy` (A) - an async HtoD memcpy reads the host pointer at GPU-COMPLETION time; a stack-local source dangles and the GPU silently reads the dead stack slot (wrong token/pos, no error); persistent host staging fields + slice::from_ref; the HOST-side complement of the readback-timing family (B26/B27 pin device transients for deferred DtoH). `captured-graph-bakes-buffer-addresses` (D) - state mutators split into address-stable (in-place memset_zeros - the graph SURVIVES) vs identity-changing (alloc/free - the graph MUST be dropped for lazy re-capture, else replay runs against freed pointers); the inverse-facing half of B25's keep-caches-alive. `output-buffer-rotation-pool-multi-forward` (A) - K pipelined forwards writing the same output field: swap the field's handle with pool[i] per dispatch (O(1) ownership exchange, zero bytes moved); ONE sync, K+1 position-aligned reads; the field KEEPS the last buffer between cycles (never un-rotated mid-run) so the next cycle's out[0] is the live pre-speculation prediction. `mid-pipeline-device-alloc-is-implicit-sync` (D) - cuMemAlloc can implicitly synchronize the device: an allocation inside a no-intermediate-sync dispatch sequence is a HIDDEN BARRIER; pre-allocate the cycle's whole working set at construction sized to the static max (SPEC_MAX_K=8) - nice-to-have pre-allocation is actually a correctness precondition for pipelining. `capture-region-dispatch-only` (D) - the captured region records GPU work ONLY: host effects (pos += 1) run ONCE at capture and NEVER on replay (silent frozen position); syncs/dtoh are not capturable; preconditions: event tracking off BEFORE any allocation, non-default stream (null not capturable), pre-upload. `occupancy-grid-cap-refuted-uncapped-default` (E) - the Issue 705 / Bench 684 measured NEGATIVE never distilled at resolution time (B49 came from Issue 706): the occupancy-derived cap (768 on 4090) measured a 1-2% LOSS at every size (87.1 uncapped vs 84.9-86.2 capped); wavefront locality beats the wave tail, killed twice over by the z-class exact-1-wave counter-example (706 vs 854 GB/s); the cap stays env-gated as measurement apparatus. `write-before-read-state-skips-checkpoint` (A) - a speculative checkpoint backs up ONLY read-modify state; write-before-read append-only caches (KV at position p: decode reads [0..=p], the append for p overwrites the stale entry before any read) are repaired by re-execution itself; async dtod, ZERO CPU-GPU sync, host bookkeeping snapshotted by the caller. `same-input-gemv-segments-single-launch` (D) - same-input GEMV groups batched into ONE multi-segment launch (<=4 segments); nsys measured 497 GEMV launches/token with m=48 projections at the ~5.6 us kernel-latency floor; pad unused slots with segment 0's pointers (m=0 matches no rows - never dereferenced); the generic middle between B25's shared-upload-N-launches and B15's bespoke fused pair. 23 candidates dropped as dups/thin. Mining yield filed as riir-ai Issue 722 (13 hazards: the spec_restore_buffers/spec_unrotate doc-lie (referenced but nonexistent), set_lora release-mode silent no-op, capture-error-path stream leak, conv1d backward full-buffer download, attention-backward per-token alloc storm, uncached lm_head dequant_transpose, + 7 more). Corpus 355 -> 363 (A 83 -> 86, D 66 -> 70, E 20 -> 21). GOAT: 769/0/1 kernel_heal + 571/0/0 default, clippy 0 both states (one post-write fix round: the B37 line-initial `+ `/`- ` lesson struck a 5TH time - 3 trigger lines cascading 33 warnings + 6 backtick items; plus the B51 case-sensitivity lesson in the concepts test), retrieval floors HELD with zero decay (kernel 100/100/100, blend 94.6/97.7 - sixth consecutive batch). Generator: all anchors unique on the FIRST dry run (fourth consecutive). The riir-gpu corpus is now COMPLETE-minus-one: only the CubeCL twin remains, blocked on Issue 721. Record: Research 065.

   Batch 60: 1 rule from Research 067 (Plan 006) - the SIXTH measured-GOAT distillation after corpus-complete (reopen path (c), same-day as the fix): riir-train Issue 460 / katgpt-rs `e2853fbf` - the MLA backward reconstructed the pre-gate attention output as `attn_out_gated / gate`; `gate = sigmoid(W_g.h)` underflows to a HARD ZERO in f32 below ~-88 pre-activation and the reconstruction is 0/0 = NaN (silent: the forward stays finite - a saturated gate is finite, the gated output is just 0; the NaN surfaced steps later as an opaque NsInvSqrtFailed in the optimizer; measured 4 of 10 real sequences at random_train_init seed 42, 4 of 5 seeds; localization proof: the FIRST corrupt tensor was mla.w_g - the ONLY consumer of the reconstruction - while every other MLA grad reading attn_out_gated stayed clean). `pre-gate-recovery-by-division-cancels-algebraically` (N 20 -> 21): never reconstruct a pre-gate activation by dividing the saved gated output by the gate - every gradient consumer multiplies the pre-gate value by gate-derived factors, and gated == pre * g by construction, so the g cancels (`pre * g * (1-g) == gated * (1-g)`): bit-identical wherever the old form was finite, no epsilon, no clamp, one fewer divide; where g == 0 the pre-gate value is information-theoretically unrecoverable (gated is 0 too - 0/0 carries no information): guard public reconstructions to 0.0 + SAVE the true value at checkpoint boundaries; detection tell: the NaN localizes to the single weight matrix consuming the reconstruction while every post-gate-reading gradient stays clean - that asymmetry is the proof. Distinct from B35 saturating-nonlinearity-recovery-breaks-near-cap (derivative -> exactly 0 near cap, fix = raw-argument recompute; this is VALUE reconstruction by DIVISION producing NaN, fix = algebraic elimination), B32b sigmoid-deriv-recovered-from-output (the optimization family this makes singular-free), B32a sign-branch-stable-sigmoid (forward sigmoid hygiene). Bench 475 repro: 6/10 -> 10/10 usable backward passes, all 6 finite-difference gradient checks pass. Corpus 363 -> 364. GOAT: 778/0/1 kernel_heal (baseline 775 + 3 gates) + 577/0/0 default, clippy 0 both states, retrieval floors HELD (kernel 100/100/100, blend 94.7/97.7/97.7 of 131 - top-1 improved 94.6 -> 94.7). Plan-first discipline ADOPTED with this batch (owner call): Plan 006 records the task set (created retroactively for B60 - the implementation predated the discipline; every future batch creates its plan BEFORE implementation; both the distill + riir-clippy skills gained the plan step in the same commit). Record: Research 067.

   Batch 61: 2 rules from Research 068 (Plan 007 - the first plan-FIRST batch, created BEFORE any corpus edit) - the oxc "Pursuit of Performance on Building a JavaScript Compiler" distill (oxc.rs/docs/learn/performance, fetched via idle-queue item 11 online perf search; the first batch fed by a FRESH online search after B48/B53/B59). `keyword-map-to-match` (dispatch 1 -> 2): static-keyword `LazyLock<HashMap<&str, V>>` dispatch -> plain `match` on string literals - **ParetoDominant 5.23-5.58x**, zero allocs; the mechanism is oxc's own refutation: they tried to beat the match with a gperf-style perfect hash (V8's approach), COULD NOT, then found in `--emit=llvm-ir` that LLVM already lowers string-literal matches to a switch on string LENGTH + one inline compare per length group ("the compiler is smarter than us") while the HashMap pays SipHash per lookup - the folk wisdom "string match is O(n) comparisons, use a hash map" is refuted for STATIC key sets (literal-initialized maps only; dynamic keys keep the map; proposer deferred - key-set extraction). `usize-index-to-u32` (layout 5 -> 6): usize index/offset fields in high-volume structs -> u32 - **ParetoDominant 1.81-1.99x**, zero allocs; the oxc Span change (2xusize 16B -> 2xu32 8B, +5% on large parser files) isolated in a bandwidth-bound span-sum bench; the 8B struct wins ~2x even at 1K where both arrays are L1-resident (twice the elements per cache line); domain guard: values must genuinely bound below 2^32; proposer deferred (type ripple - the B58 caller-gate class). 8 other oxc candidates DROPPED at dedup: box-enum-variants + Box<str> + SIMD-skip + BufWriter + jbecker buffer-reuse were DUPLICATES of existing corpus rules; arena-AST, string-interning-sharding, string-inlining were architectural/external-crate (no bounded fix-space). GOAT: 595/0 default lib, clippy 0 default + rust_perf_bench states, retrieval floors HELD (12/12 retrieval tests, perf@1 94.3%, blend@1 92.1% structural - both new rules surface in suggestions without crowding fixtures). Generator lessons: the B37 line-initial `+ ` markdown-list lesson struck a 6th time (rewrapping a header paragraph moved `+ bounded-channel-backpressure` to line start -> 3 doc-lint warnings); NEW LESSON - edit_file fuzzy matching silently DROPPED a `#` from an unrelated raw-string terminator (`}"` vs `}"#`), un-terminating the division harness 100 lines away and swallowing the new keyword fn into its string (E0425 + phantom E0308s) - raw-string `#` terminators are load-bearing; re-read file state after edits near `}"#` boundaries. Corpus 63 -> 65. Record: Research 068.

   Batch 62: 2 rules from Research 070 (Plan 008 - the microGPT-C CPU-SIMD distill; github.com/vixhal-baraiya/microgpt-c, MIT, 616 stars, inspected 100%; the note's verdict-correction record owns three initial-assessment errors - a 2-5x GOAT-anchor bar the corpus doesn't have, a no-consumer claim the grep refuted, and yield-1-2 treated as disqualifying when it IS the norm). `transposed-weight-register-gemv` (simd 5 -> 6): row-major per-row dot + horizontal reduce -> transposed weights, per-input broadcast FMA into output-vector accumulators (the ryg 2015 transposed-lane Mv analysis; microGPT-C `mv16_blk16v(4)` shape, portable-Rust form) - **ParetoDominant 1.18x / 2.09x / 6.08x** (NIN 16/64/4096 at NOUT 64, M3 Max). **The research's wash-at-large-nin expectation was WRONG-DIRECTION**: in strict-FP Rust the row-dot's scalar accumulator is a serial dependency chain LLVM cannot vectorize without reassociation, so it degrades FURTHER at large nin (llama.cpp's fine row-major dots use explicit multi-accumulator SIMD - not what portable scalar code gets) - the serial-chain latency, not the horizontal reduce, is the dominant cost. Architectural rule (weights pack transposed once at weight-freeze) - suggestion-quality, no proposer (the `soa-hot-field` class); distinct from the kernel_opt GPU plane-per-row family by substrate. `newton-refinement-vs-hardware-transcendental` (arithmetic 4 -> 5, annotated): Newton-refined recip/rsqrt (the GPU fast-math recipe) -> hardware single-instruction div/sqrt on CPU - **Mixed 1.09x / 1.01x, hardware never loses** (source measured 2-4% on Ryzen 5 5600H; the M3's fdiv/sqrt latency flattens the gap at 100K); the honest-annotation class (`filter-count-to-any` precedent) - the rule exists to STOP a plausible-looking GPU->CPU fast-math port; context_notes carry the regime inversion + the separate relaxed-precision-estimate axis. Proposer for the lead rule deferred (layout restructure - the B58 caller-gate class). GOAT: 603/0 default lib, 621/0 rust_perf_bench lib, 655/0+1i latent_retrieval lib; clippy 0 in default-all-targets + bench + latent states; retrieval floors HELD (blended 101: 92.1/96.0/97.0). **Drive-by pin-rot fixes (the 17th-count-pin-site lesson, batch-scoped):** Batch 61 updated the `55 are benchable` header but NOT the `fifty_three_rules_have_bench_specs` pin (stale at 53 vs 55 actual - the bench-feature lib suite was not in Batch 61's GOAT run; discovered when Batch 62 ran it properly) + 9 pre-existing pedantic warnings in Batch 59's division-test module never linted (`--all-targets` clippy had not been run on the test target since Batch 59): fixed via ilog2 x2 (behavior-identical, proven by the exhaustive sweep), `#[allow(cast_possible_truncation)]` on the two libdivide impls (the high-word extraction + magic wrap ARE the algorithm), `#[allow(too_many_lines)]` on the exhaustive gate fn, + underscore-separated literals. Generator lessons: the B37 line-initial `+ ` markdown-list lesson struck a 7TH time (hand-written doc edit - `per-row dot` then line-initial `+ horizontal reduce` - bypassed the generator discipline); the annotation-vocabulary contract test caught a missing verdict token (ParetoDominated/Mixed/NotBenchable) before landing. Corpus 65 -> 67. Record: Research 070.

   Batch 63: 3 rules from Research 071 (Plan 009 - the CubeCL twin unlock: `ternary_deltanet_gpu_forward.rs`, 5,432 lines @ origin/develop `42cc13e0d`, committed-clean; the 721-closeout unblock fired). Two-lane mining: **Lane B returned ZERO candidates — structurally empty** (the chunked-recurrence/tree-verify kernels live in sibling modules mined at B28, the spec-decode mechanisms at B58 from the cudarc twin; one inline kernel = a port of the B22 layout; zero host-side numerical math). Lane A's 3 (coordinator-verified distinct against the actual B58/B9/B19/B27/B39 texts): `same-input-gemv-weight-concat-at-upload` (D 70 -> 71; Issues 642 F3/F2 + 648 F9) — N same-input GEMVs merged by baking the weight CONCAT into the upload; the STOCK kernel dispatches it; offset-consumers pay zero extra dispatches, separate-buffer consumers ONE split; the substrate-constrained THIRD branch of the merge family (B58's segments need a multi-segment launcher API — the CUDA twin's route; the twins CONVERGED on the same problem from opposite constraints, which is itself the evidence the family needs both branches); B15's bespoke fusion is the third pole. `deferred-residual-fused-into-next-layer-norm` (C 33 -> 34; Issue 645 T3) — the post-FFN residual add deferred ACROSS the layer-loop boundary into the next iteration's fused add+norm kernel, -63 dispatches/token with ZERO new kernels; first-layer + tail special cases; the in-source composition ladder documents B19's producer-side fusion as the better rung; the scheduling analog of defer-readback-until-after-submit. `reset-state-in-place-fill-never-realloc` (A 86 -> 87; Plan 528 T0.2) — sequence reset via in-place zero-fill over process-lifetime buffer identity, never re-allocation (128 create_from_slice per reset fragmented the pool to SIGKILL under the Go arena); cursor-bounded caches skip even the fill. 11 Lane-A dedup drops + 24 Lane-B examinations all mapped to covering rules. Hazard yield: 10 findings filed as a riir-ai issue (dead wk/wv device upload — the B56 class; dual/triple weight residency; the ResidualAdd-as-copy doc-lie with CopyCubeCL sitting imported+used; the un-ported B58 rotation pool allocating K vocab buffers per spec cycle; the aliasing-UB probe shipping default-OFF). GOAT: kernel_heal 811/0/1i + default 607/0 + retrieval floors HELD + clippy 0 all-targets both states. **The 18th count-pin site**: router.rs `rules_for_expert_total_matches_corpus` (experts-cover pin derives from KERNEL_CATEGORIES — broke the moment the table grew; found by the full suite). The B51 case-sensitivity lesson struck a 5TH time (concepts needle `next loop iteration` vs the concept's `NEXT`). Corpus 364 -> 367 (A 87, C 34, D 71). Record: Research 071.
   Batch 70: 3 rules from Research 080 (Plan 020 — the FIRST kernel_opt paper distillation, the Research-076 arxiv intake channel consumed: arXiv:2608.13757, Sung/Shyamsunder/Zhang/Kroening/Choi, "A Barrier-Free Synchronization Algorithm for Multi-Engine AI Accelerators", CGO '27; Lean 4 bisimulation ~15K lines 0 admitted; measured on real Trainium 2). `barrier-free-per-dependency-wait-from-loop-counters` (D 71 -> 72) — loops of heterogeneous workers with cross-iteration deps must NOT drain+handshake+reset all workers at iteration boundaries; track a monotone entry count per worker/loop and compute each dependency's wait threshold in closed form (the three allocatable cases: producer-in-S -> `r_P - k`; producer nested forward offset-0 / backward offset-1 -> `r_P`; underflowed target = no-wait; outside the subset -> barrier fallback, stated in-rule); source evidence 10-45% latency cut on production NKI kernels + 3.3x sync-bound microbench. `shared-counter-decode-producer-progress` (D 72 -> 73) — one shared counter per (loop, worker) decoded as `N*r - (N - K)` under in-order retirement; decision content = the granularity ladder (per-instruction O(N) counters/3 ALU ops; per-loop O(L*E)/5; per-engine O(E)/O(D)), mixable per region, LICM/CSE-reducible. `masked-branchless-wait-collapse` (B 52 -> 53, category corrected from the pool's tentative A — the in-corpus `branchless-select` precedent) — the no-wait OR threshold disjunction collapsed into a single masked threshold via a 0/1 `isGT` multiply; fixed 3-op sequence, no branch on the issue path; offset-0-only loops elide the mask entirely. Honest applicability encoded: wgpu/CubeCL/Metal expose no fine-grained semaphore (cudarc CUDA events + multi-stream, host orchestration loops, worker threads are the live surfaces); the numbers are Trainium-specific evidence, the rules encode the mechanism. Drive-by (pre-existing at HEAD, proven): CI Layer 5a all-features clippy was RED from the sec landing — `floors::ROUTER_ANNOTATION_3D` declared `#[cfg(feature = "kernel_heal")]` but its only use-arm requires `not(sec)` -> dead_code under `-D warnings`; fixed by narrowing the declaration cfg (the only mismatched const of the four). Also caught by the same layer: the new syn-parse gate needed the `#[cfg(not(feature = "kernel_verify"))]` attribute every prior `*_pseudocode_parses` gate carries (`.parses` does not exist under kernel_verify). GOAT: kernel_heal 864/0/1i + default 657/0 + all-features 1115/0/13i + ci_feature_guard ALL LAYERS + retrieval floors HELD (kernel self-variants 100/100/100, blend@1 94.7, perf@1 96.2, clippy@1 89.6 — no re-pins). Corpus 367 -> 370 (B 53, D 73). Record: Research 080.

   Batch 71: 2 rules from Research 083 (Plan 022 — the oMLX external distill, Research 082's queued verdict consumed; the SECOND external batch + the first from a live serving repo: jundot/omlx, Apache-2.0 Python/MLX Apple-Silicon engine; scheduled by the 2026-08-24 idle-loop "research online to unblock blocked tasks" decision, which fetched the 3 previously-UNREAD/timed-out sources). `rotated-ring-undo-via-rebind-reference-snapshot` (A 87 -> 88) — a rotated ring cache CANNOT trim (the evicted token's slot is overwritten), so a rejected speculative verify leaves PHANTOM tokens progressively corrupting output (DeepSeek-V4-Flash, sliding_window=128; pointer rewind structurally impossible — no monotone extent); exact undo via the update path's REBIND contract: the S>=2 concat path only rebinds keys/values (no in-place setitem), so stashing pre-update attribute refs + the staged inputs + restore-and-replay-the-confirmed-prefix is exact; thread-local arming around the speculative forward only; plain refs suffice on the rebind path, detach-copy (v + 0) only where the path may setitem in place; chains single-step decode-consistent verifies by keeping the original snapshot + concatenating staged inputs. `gpu-api-thread-affinity-snapshot-metrics` (D 73 -> 74) — a background enforcer thread must never call GPU runtime APIs during active requests (corrupts command-buffer state; a wrong-thread pool clear races the async completion callbacks — the SIGABRT/prepare-count-underflow class, Research 082's #3 merged in as the flag-and-defer justification): snapshot (executor-thread-published cached sample + kernel ledger, direct-API reserved for the idle regime), flag-and-defer (atomic per-owner flag; the clear runs on the inference thread at the next step boundary through the lock-protected engine-stream-synced path — also fixes idle-only reclaim's busy-server gap), cache-once (immutable device caps resolved outside the poll loop). Pool dispositions: #4 MMA indexer killed as B16 dedup (validation evidence only); #5 Laguna fusions dropped — `optimizations.py` read at mine time is a hardware-detection STUB, the fusions are not in it (the doc-sourced C-grade evaporates on source read). GOAT: kernel_heal 879/0/1i + default 669/0 + combined 955/0/2i + all-features 1152/0/13i + ci_feature_guard ALL LAYERS + retrieval floors HELD zero decay (kernel self-variants 100/100/100, blend@1 94.7, perf@1 96.2, clippy@1 89.6 — no re-pins) + clippy 0 both states (one new doc_markdown variant learned: stylized ALL-CAPS verbs like KILLed/DROPPed in `//!` prose trip it — use plain words). Generator: the /tmp worktree needed sibling-path manifest surgery (7 relative `../repo` paths absolutized, restored pre-commit — the riir-ai worktree lesson applied to riir-clippy). Corpus 370 -> 372 (A 88, D 74). Record: Research 083.
   Batch 72 (2026-08-26, Research 084 — the frguthmann/NVIDIA compute-shader distill): 1 rule, `skip-unchanged-writes-via-dirty-encoding` (corpus 372 -> 373).
   Batch 77 (2026-08-27, Research 091 — the lucebox PR #625 external distill, Plan 035; all rules source-verified at head f28d85c, 3 provenance corrections folded in): 6 rules, corpus 373 -> 379 (A 90, C 35, D 76, G 20, M 10) + 1 B63 rider — `weight-stream-non-temporal-loads` (A: streaming-vs-reused operand regime; dual measured proof — source IQ4_XS vecdot + riir-ai T9.16 `__ldcs`/Bench 734; bit-identical by construction), `batch-width-routed-dual-tile-twins` (M: namespace-reinclude twin instantiation; the pathological mmq_x=32 valley 443/180/315 GB/s — sweep, never interpolate), `tree-verify-recurrent-state-parent-reload` (G: DFS branch transitions reload the register state shard from stored per-token intermediates — same-thread RAW, no barrier; the recurrent twin of B71's KV-cache undo), `shared-activation-quantize-memo-per-eval` (D: the THIRD same-input mechanism — quantize-memo vs B58 launch-merge vs B63 weight-concat; LIFO-at-eval-start + MAX_BUFFERS pool sizing 256->1024 + the concurrent-streams guard), `vendor-batched-gemm-contiguity-staging` (D: strided slices silently route into the vendor pinned-host staging path; contiguize + CS<=32 k-limit), `gate-prep-fold-into-consumer-via-packed-side-tensor` (C: the packed side tensor at src[9] + op_params[10] + the HARD capability REJECT); rider on B63 (placement-time stacked aliases). GOAT: kernel_heal 915/0/1i, default 699/0, all-features 1341/0/13i, clippy `--all-targets` 0 both states, FULL CI guard ALL LAYERS, retrieval floors 14/14 HELD (kernel 100/100/100). Record: Research 091.
   Batch 79 (2026-08-27, Research 093 -- the BAIR K-Search + cubek-matmul external kernel distill, Plan 037; sources verified at upstream heads -- K-Search @ 53c8fab + cubek @ c2323a2, the 3 provenance corrections carried in-rule): 5 rules, corpus 379 -> 384 (A 91, D 78, E 22, G 21) -- `first-order-affine-recurrence-associative-scan` (G: the affine recurrence IS an associative operator over pair-space, O(log N) prefix scan vs O(N) sequential -- the FOURTH recurrence strategy beside chunkwise/cumsum-matrix/streaming; spec-seeded, search-realized provenance enforced in-rule [never 'independently discovered']; REGIME: the combine cost picks the strategy [scalar pairs scan, dxd state chunks -- cross-ref the riir-ai Issue 734 chunked-GDN negative]; REL-ERROR GATE REQUIRED -- tree-order association is never bit-identical; the G4 gate is allclose-shaped [rtol 1e-3 + 100*EPSILON*scale atol] over 4 adversarial families with a non-vacuous bitwise-differs assert), `plane-specialization-load-compute-role-split` (E: comptime `PlaneFlowPartitionRule` + runtime plane-id branch; load-only planes run the whole load job, raising occupancy without accumulator registers; never specialize WITHIN a plane; underperforms on Metal), `vectorized-smem-write-no-bank-padding` (A: REFUTATION-type -- line-sized vectorized smem stores distribute across banks by construction, the +1 pad lineage is unnecessary; `SwizzleMode` encoded as the modern complement, not padding), `ordered-lhs-skip-smem-fragments-as-second-buffer` (D: the register fragments ARE the second Lhs buffer [verbatim struct doc]; ORDERING-REPLACES-SYNCHRONIZATION [Lhs 1x k-step, Rhs 2x k-step, contiguous per-plane tile groups]; the sequential-halves allocation correction rides; the B35 dedup tension resolved at authoring -- buffer-count + ownership is not the reuse-count staging gate), `interleave-loads-between-tile-matmuls` (D: kept SEPARATE from the Ordered rule at authoring -- asynchrony is INDUCED by instruction-stream placement between tile matmuls [`StageEventListener` on `TileMatmulCompleted`]; clean load/compute stage separation REMOVES the overlap). Drops: sm-divisor-cube-allocation (C+ thin) + tensor-core-opaque-store-smem-bounce (blog-only, unverified at head). GOAT: kernel_heal 923/0/1i + default 703/0 + all-features 1344/4/13i (the 4 = PRE-EXISTING environmental l4_daemon Windows-spawn failures, proven at HEAD via stash -- same counts on the pristine tree; they spawn a non-Win32 helper, green where authored on macOS) + clippy `--all-targets` 0 both states + CI guard layers 1-4 + 5a + 5b-default + 5c (8/8 KAT binaries) PASS [5b-all-features fails only on those 4] + retrieval floors HELD zero decay (kernel 100/100/100 -- 8th consecutive; blended 93.9/96.9/97.7; clippy 89.6/95.8/95.8; perf 94.3/96.2/98.1). Record: Research 093 / Plan 037.

      Batch 80 (2026-08-27, Research 094 -- the WebGPU dispatch-overhead characterization distill, Plan 038; source arXiv:2604.02344v1, Maczan, submitted 2026-02-09, open data, measured across 4 GPU vendors + Dawn/wgpu-native + 3 browsers; the substrate is OURS -- wgpu host orchestration, cross-validated in-tree by riir-ai Bench 761): 3 rules, corpus 384 -> 387 (C 35 -> 36, D 78 -> 79, E 22 -> 23) -- `dispatch-cost-sequential-measurement-not-single-op` (D: naive single-op dispatch benchmarks CONFLATE the submit->completion-drain round trip with enqueue cost, 10-60x overestimate [Dawn 496.8 vs 23.8 us, Chrome 2071-3123 vs 32.8-66.5 us]; the sequential protocol -- warm up, N back-to-back dispatches, ONE tail sync; submit alone is 40% of the true 32.5 us cost, so submit-count reduction is the lever), `fusion-payback-gated-on-per-dispatch-overhead` (C: the SAME RMSNorm 6->1 fusion measures 1.41x wgpu/Vulkan [24-36 us/dispatch] / 0.95x wgpu/Metal + 0.91x Safari [kernel-penalty-dominated] / 0.92x CUDA [7.4 us -- nothing to save]; payback = dispatches_saved x per-dispatch overhead vs fused-kernel penalty; B* batch crossover marks the overhead-bound regime [batch=1 LLM matmuls at B* >= 7]; the overhead input MUST come from the sequential protocol), `grid-collapsing-mega-kernel-occupancy-trap` (E: WebGPU has NO cross-workgroup sync so the mega-kernel is ONE workgroup -- 17k sequential MADs/thread at production vs 65k+ threads; the tiled middle ground 7->3 dispatches preserves the grid: 2.01x Metal p<0.001 / 1.17x Vulkan p<0.01; mega-vs-multi honestly INCONCLUSIVE at toy scale p>0.38 -- the regime split IS the rule). Pool drops: frguthmann post (consumed by B72), NVIDIA thread-group swizzling (covered by l2-grouped-tile-schedule), LLVM opt-remarks lane (tooling, no fix-space), Instruction-Wall paper (analysis-level, maps to existing A/E ground). GOAT: kernel_heal 926/0/1i + default 703/0 + all-features 1346/4/13i (the 4 = the known environmental l4_daemon Windows-spawn failures, identical B79 signature; count-consistent 1344+2, the parses gate is cfg(not(kernel_verify))-compiled-out under all-features) + clippy --all-targets 0 both states (2 doc-markdown backtick fixes pre-commit) + CI guard layers 1-4 + 5a + 5b-default + 5c (8/8 KAT binaries) PASS [5b-all-features fails only on those 4] + retrieval floors HELD zero decay (kernel 100/100/100 -- 9th consecutive; blended 93.9/96.9/97.7). Record: Research 094 / Plan 038.
            Batch 81 (2026-08-27, Research 095 -- the lucebox concurrency-stack distill, Plan 039; PRs 642/651/659/668, C++/HIP on vendored ggml, R9700/gfx1201; #642 superseded by #651, never cited separately): 6 rules, corpus 387 -> 393 (A 91 -> 93, D 79 -> 82, G 21 -> 22) -- `virtual-tail-packed-tree-verify-over-paged-kv` (A: tree candidates as a VIRTUAL TAIL of the committed length; block-table prefix rows + scratch-slab candidate rows behind ONE logical index; ancestor-visibility walk gates siblings; phys = -1 invisible rows contribute the reduction identity), `gdn-transition-replay-log-commit-many` (G: the forward journals [gate | key | delta] per token/head, journal VIEW OWNS storage; ONE kernel replays accepted prefixes via fmaf(key, delta, gate * current) -- no second target forward), `cross-lane-column-packed-projections` (D: the AXIS-TRANSPOSED fourth member of the same-input family -- N lanes' DIFFERENT inputs packed over ONE shared weight; lane-local consumers [positions/RoPE/masks/cache-writes/attention] carve back out; re-pack at each shared-weight boundary; single-lane fast path), `identity-keyed-graph-retention-stable-arena` (D: composite owner/backend/topology key with the kv axis BUCKET-quantized to 256; retained 512 MiB uninit metadata arena keeps nodes[0] stable; the address SALT prevents cross-bucket aliasing -- retention + salting are two halves of one contract), `staged-append-rollback-lifo-retry-blocks` (A: rollback routes freed blocks to a per-sequence LIFO so retry reproduces the SAME physical rows in the SAME order -- captured graphs + block-table deltas survive reject+retry; a global free list guarantees neither), `fail-closed-multi-resource-commit-transaction` (D: VALIDATE every destination across ALL resources first [device pointers, duplicate-slot bitmap, ranges], then COMMIT_PREVALIDATED; after the first mutation NO recoverable error may escape -- abort, because fallback from a mixed pre/post-commit state is silent corruption). Plus the P3-2 rider on `shared-activation-quantize-memo-per-eval` (lucebox #668: the REQUEST-boundary RELEASE half -- memos popped LIFO BEFORE pool trim; synchronize first; legacy non-VMM pools only, VMM exempt; 2.2-2.4 GiB is a PR-body number). ZERO dedup kills -- the ~50% falsification load was pre-absorbed by the C/E/I rider rulings at research time (Lane C read every near-neighbor in full). Provenance corrections folded: NO measured tables exist in the diffs (all p50/goodput are PR-body numbers; -22 dispatches not reproducible from the op inventory); the F rule gained TWO halves the verdict missed (kv bucket quantization + the arena salt); the generation counter belongs to the SELECTOR graph cache, not the target graph. GOAT: kernel_heal 929/0/1i (926+3) + default 703/0 + all-features 1348/4/13i (the 4 = known environmental l4_daemon, identical B79/B80 signature; count-consistent 1346+2 -- parses gate cfg-compiled-out) + clippy --all-targets 0 lint warnings both states (1 doc_markdown backtick fix pre-commit; the exFAT hardlink cargo noise excepted) + retrieval floors 14/14 HELD zero decay (kernel 100/100/100 -- 10th consecutive; blended 93.9/96.9/97.7) + CI guard layers 1-4 + 5a + 5b-default + 5c PASS [5b-all-features fails only on those 4]. Record: Research 095 / Plan 039.
                        Batch 87 (2026-08-29, Research 103 -- the TTT-KVB App-I boundary distill, Plan 048 / Issue 054; arXiv:2602.21204 "TTT with KV Binding Is Secretly Linear Attention" NVIDIA ICML 2026, corpus record katgpt-rs R516 `513088cd`): 2 rules + 1 B79 rider, corpus 398 -> 400 (G 23 -> 25) -- the BOUNDARY COMPLEMENT of B79's `first-order-affine-recurrence-associative-scan` (B79 = affine => scan; this = what BREAKS the scan): `state-normalization-between-updates-breaks-associative-scan` (G: Norm(A+B) != Norm(A)+Norm(B) -- state normalization between updates voids associativity while the mechanism still reads as linear attention; the scan returns the RAW affine trajectory, a wrong answer at scan speed never an error; LaCT weight-norm = the live anti-shape; fix-space = normalize q/k instead (phi-side, scan-SAFE -- the in-tree negative control `expand_and_l2_normalize_heads_f32` normalizes Q/K PRE-recurrence) or accept the sequential form), `dynamic-kernel-params-break-prefix-scan` (G: Theta updated INSIDE the recurrence makes phi history-dependent through nested-nonlinearity chains (silu' nesting); the scan port silently computes a different operator; fix-space = freeze Theta at layer load, adapt BETWEEN forwards; the paper's V1/V2 reduction rungs (15.93 / 16.31 ppl, best-or-near AND the first parallelizable shape) are the design table), + the B79 rider (SCAN-ELIGIBILITY BOUNDARY sentences in B79's gpu_concept: qk/head-side normalization is part of phi and does NOT break the scan). Theory-distilled grade B (throughput AND quality numbers are LaCT end-to-end, never cite isolated); ZERO in-tree firing population (paper-distilled precedent = B79 itself). Authoring-time falsification: subagent drift caught -- "the ONLY shape that admits the chunk-parallel form" corrected to FIRST/minimal (R516 Table 2 shows V3-V6 parallel tok/s; weight-norm removal is what unlocks it). GOAT: batch87_pseudocode_parses (cfg(not(kernel_verify)) per B70) + NEW batch87_state_norm_breaks_associative_scan (falsifiable breaker gate: scan-computed raw trajectory vs sequential normalized truth diverge materially >1e-3*scale, non-vacuous) + kernel_heal 984/0/1i + default 752/0 + clippy 0 both states (3 self-inflicted doc_markdown backticks: LaCT/gpu_concept -- the B37 lesson) + kernel_verify compile + retrieval floors 14/14 HELD. CI guard NOT run this cycle (shared checkout carried the sibling's in-flight B85/docker WIP -- a full run would verdict their mid-flight code; targeted states validated instead). Record: Research 103 / Plan 048.
                        Batch 92 (2026-08-31, Research 107 -- the ktransformers Marlin distill, Plan 053; kvcache-ai/ktransformers @ 21095b9, `archive/csrc/custom_marlin/`, the modified Marlin W4A16 GEMM -- Neural Magic / IST-DASLab / vLLM lineage with ktransformers' own device-M extension; verdict kernel_opt A-; 11 rules, corpus 400 -> 411, M+1 N+2 E+1 B+1 A+5 L+1): `device-m-runtime-mblock-graph-stable-gemm` (M; the HEADLINE -- M read from a DEVICE pointer inside `Marlin_wrapper` (cu:1415-1426), capped 1024, m-blocks derived at runtime, {1,2,3,4} template dispatch via if-chain, grid = sms exactly (cu:1723) -- ONE captured graph serves every batch size; `test_cuda_graph.py` proves the capture contract; distinct from cuda-graph-scalar-via-device-pointer (B22 scalar plumbing vs shape-freedom) + capture-break-count-not-perf-target (B82 pads; this keeps M dynamic) + enum-dispatch (host-side choice vs device-data choice)), `cpasync-multistage-tile-pipeline` (A; the 4-stage `cp.async.cg` ring with the `stages - 2` wait contract + fence-even-on-wind-down + the MID-TILE issue at `k == b_sh_wr_iters - 2` (cu:1236-1258); the `gptq_marlin_repack.cu` rider re-declares the identical ring at 8 stages for pure movement -- caller-agnostic SHAPE), `register-double-buffer-operand-pipeline` (A; `frag_a[2]` etc + fetch(k+1) before matmul(k) + the EVEN-length-pipelines invariant making every access static), `ldmatrix-mma-layout-loads` (L; `ldmatrix.sync.aligned.m8n8.x4` produces A already in fragment layout -- the load instruction owns the lane permutation; B rides RAW through smem, dequant in registers), `xor-swizzle-smem-static-index-tables` (A; the XOR swizzle precomputed into BOTH static tables (`a_sh_wr_trans`/`a_sh_rd_trans` -- same transform, different access patterns, cu:638-665); the concrete complement B79's no-bank-padding rule already cited as `SwizzleMode`), `multi-pointer-dependency-break` (A; `B_ptr[b_sh_wr_iters]` -- an array of pre-offset independently-advancing cursors breaking the serial address chain on RUNTIME-stride global loads (cu:667-674, the source's own "tiny optimization" with a verbatim `maintining` typo); static tables impossible on this side -- runtime strides), `quant-group-scale-fetch-gating` (A; fetch scales ONLY at group-boundary tiles + the read side QUANTIZES its stage index so the whole group consumes ONE slot (cu:780-797, 862-866); alignment precondition = stripes rounded to group multiples), `lop3-int4-to-half2-dequant` (N; ONE `(a & b) | c` LOP3 per nibble pair with f16 exponent embedding `| 0x6400` + zero point fused into the packed-op constants -- the compiler does not auto-recognize it, source note), `prmt-int8-to-half2-dequant` (N; two prmt byte-perms whose SECOND source writes the exponent byte + one `__hsub2` vs magic 0x64806480 consuming the int8 sign extension; the recipe instantiated on B20's permute-immediate mechanism), `stripe-sms-l2-semaphore-reduce` (E; grid = sms + ragged shapes striped + cross-block partials reduced serially IN THE L2-RESIDENT fp16 output buffer behind a per-column counting semaphore (`ld.global.acquire` spin / `fence.acq_rel` + `red.relaxed` count / reset on last, cu:315-345 + 1074-1084); deterministic FP order in stripe index, no separate accumulation buffer), `mma-fragment-layout-m16n8k16` (B; the reference-contract rule -- FragA=4u32/FragB=2u32/FragC=4f32/FragS=1u32 (dtypes.cuh:20-26) + the mma.sync.m16n8k16.row.col.f32.f16.f16.f32 PTX; honest scope note in-rule: layouts are ISA-documented, the pinned content is the register contract + scale-before-mma keeping the accumulator f32). All quotes coordinator-verified byte-accurate (20 regions re-read); zero measured numbers cited (the source ships none -- Research 107 recorded that honestly). GOAT: batch92_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal lib 1234/0 + default 1001/0 + triple (kernel_heal,latent_retrieval,clippy_verify) 1370/0/8i + clippy 0 self-crate in kernel_heal / kernel+latent / all-features states + retrieval floors 14/14 HELD (no re-pins). Count pins walked: mod.rs header + 7 count comments (incl. the pre-existing Fusion 35->36 drift fix) + tests.rs 69/5362/5364/5375 + router.rs 9/461. Full CI guard backgrounded (log /tmp/b92_guard.log). Record: Research 107 / Plan 053.
                                                Batch 93 (2026-08-31, Research 107 residue -- the ktransformers DISPOSITION batch, Plan 054; zero corpus delta, the B50 zero-yield precedent): the snapshot's "REMAINING QUEUED (candidates 13-14)" line drove this batch and the mine-time verification COLLAPSED it -- candidate 14 was ALREADY INSIDE Batch 92 (`quant-group-scale-fetch-gating` carries the cu:465-471 stripe-alignment precondition verbatim: "Ensure that the number of tiles in each stripe is a multiple of the groupsize"; the queue line was stale the day it was written) and candidate 13 (`k-block-half-of-l2`) has NO source anchor (grep-verified: the 1/2-L2 formula is absent from the repo; the only L2 content already landed as `stripe-sms-l2-semaphore-reduce`). The 8 rust_perf/host candidates all dispositioned with grounded reasons (full table in `.plans/054`): `grow-only-arena-pointer-rebind` = the B32a split-scratch/arena family (verified site `shared_mem_buffer.h` MemoryRequest/append_pointer); `intensity-gated-mat-vec-twins` = B77 `batch-width-routed-dual-tile-twins` covered (external validation); `fuse-independent-gemms-one-task-space` = the B58/B63/B77 same-input family; `allow-n-pending-sync` KILLED ON THE BAR with its real anchor recorded (`task_queue.h:29` + `cpp:53-61` -- parameter-not-rewrite, workload-dependent win = the metric-mismatch class, zero evidence in source; every landed concurrency rule carried a measured verdict); `host-callback-on-gpu-stream` ROUTED to riir-ai Plan 548 intel; `dual-seed-page-hash` = dead scaffolding in source (`hash_check`/`check_hash_seed` declared, never consumed -- and our prefix caches are BLAKE3-committed, 64-bit collision checks moot at 256-bit); `batched-evict-count` = commented-out config check + the B30 count family; `recompute-ratio-guard` = no anchor; `decode-prefill-kernel-split` superseded by B92 device-M (mode-FREE kernel, the better half) + B77 twins. Lane CLOSED; standing lesson: a "remaining queued" line that outlives its batch breeds stale re-mining -- disposition or delete at landing time. Also en route: `.plans/.highwater` repaired (said 52 while plan 053 existed -- the missed-bump collision class). Record: Research 107 (status updated) / Plan 054.
Batch 94 (2026-09-01, Research 110 verdict + Research 112 landing -- the oMLX PR #3260 SSD Expert Streaming + Soft-REAP distill, Plan 058; jundot/omlx @ 3343e44, head UNCHANGED since research time so the verdict-time coordinator verification carried over; two mine-time verification lanes re-read all 10 module files + docs/soft_reap.md, 3,503 LOC, ~35 anchors re-verified byte-accurate with 4 line-range corrections + 2 paraphrased phrases replaced with source wording; 8 kernel_opt D rules, corpus 414 -> 422 (D 85 -> 93) + 1 rust_perf io rule (96 -> 97) + 3 riders): `cross-run-profiles-key-on-content-not-stat-triplets` (D -- warm-start profiles keyed sha256(path+NUL+size+mtime_ns); an mtime-preserving copy of a DIFFERENT artifact matches the stale profile (foreign warm start), a touch of the SAME artifact drops a still-valid one (silent learning loss); key on content (BLAKE3) and guard the derivation like its save/load siblings -- the one unguarded line in an otherwise fully-guarded module, latent in the production call order, reachable via TOCTOU/direct callers) + `fill-gate-speculation-until-working-set-resident` (D -- admission control for the speculative regime: latch readiness on all pools at capacity, count refused passes; measured BOTH sides: pre-gate 166.965 s speculative vs 14.709 s checked, honest ceiling 0.579 s vs 0.556 s at 100% hit) + `decayed-frequency-eviction-recency-tiebreak-route-protection` (D -- halve hotness every 16 routed tokens (saturating); min-hotness victim + recency TIE-BREAK; note routes BEFORE hit/miss classification (misses are demand signal); current route protected; fail loud when all-protected; warm-start hazard folded in as decision content: lifetime counts seeded raw into the DECAYED field = ~64,000x eviction-immunity skew + split-brain vs the persisted undecayed hotlist ranking) + `defer-promoted-write-to-first-consumer-dispatch` (D -- bind (slot metadata) now, materialize (bank bytes) at the first consumer dispatch; the next layer's router evaluation IS the fence; an explicit eval after every dynamic update only materializes the same writes twice; bind vs materialize kept as separate stats; eager for warm-start/pinned-preload only) + `rollback-snapshot-covers-all-mutable-state` (D -- the coverage-contract DEFENSE rule: ndarray-like leaves captured by-reference are safe ONLY under the append-only write contract (in-place mutation of visible bytes leaks straight through); __dict__-less objects silently skipped; scope pinned to the conventionally-named cache argument; first-miss-only trust + rejected passes never returned + retry x1 pinned) + `scratch-bank-isolates-one-shot-operands-from-hot-cache` (D -- one-shot cold groups execute in a reserved scratch region (bank = pool_size + scratch_slots) that never mutates slot maps or residency masks, so hot rows are STRUCTURALLY untouchable; measured: evictions 42 -> 0, miss-heavy prefill-to-decode fixture 2.12-2.19x) + `sort-consumer-routes-by-physical-slot` (D -- remap-scrambled routes defeat the vendor sorted-gather fast path; threshold-gated (64 routed rows) argsort + inverse scatter restores it, bit-identical, +26.6% prefill at 1024x256; physical order is an API CONTRACT with the gather kernel) + `row-addressable-operand-index-not-full-tensor-map` (D -- header-only row-address index (stacked + fragmented layouts), addressed rows only, no payload mmap; THE HAZARD TWIN in-rule: the legacy path guards uneven division (total % rows) while the schema path floor-divides UNGUARDED -- every addressed read after row 0 drifts into the previous row's tail: silent weight corruption; the fragmented path's exact-byte check is the third instance of the same contract, the merkle_root lesson class). Riders: B63 FOURTH MEMBER (bank-level gate/up fusion -- one preallocated quantized bank, weight shape doubled at upload, one gather-QMM + split; decode QMM launches 3 -> 2 per layer, +8.6% single-token, no bank-memory increase), B71 PRODUCER COMPLEMENT (prefetch workers decode to HOST bytes only; device array construction on the owner inference thread -- the staging half of the affinity contract), B84 ROUTING-SHAPED MEMBER (resident substitution: max-relative-loss threshold, ORIGINAL gate weights retained for scoring + renormalized after selection, Qwen-only, 0% exact / 100% resident-set-only, shipped disabled pending quality eval). rust_perf crossover: `coalesce-nearby-reads-bounded-gap-cacheability-gated` (io; source-reported, honestly NotBenchable -- merge addressed rows within a bounded byte gap (64 KiB) into one pread; the DECISION CONTENT is the cacheability coupling: gap budget collapses to 0 on direct/F_NOCACHE descriptors because over-read is only free when the page cache absorbs it; riders: Darwin fcntl hints once per persistent descriptor advisory-swallowed + fd cache keyed (path, cache-mode); the reader's fd race recorded in the annotation, not folded in). Bug yield: the 4 corpus-grade hazards DEFENSE-recorded per Research 110 s7 (stat-triplet -> rule 1; warm-start scale confusion -> rule 3; placeholder poison + snapshot coverage -> rule 5; fd race -> the R9 annotation). GOAT: batch94_pseudocode_parses + batch94_rules_present + batch94_concepts_covered + kernel_heal lib 1238/0 (+3 = the new gates) + default 1001/0 + all-features 1728/0 + clippy 0 --all-targets both states (4 doc_markdown backtick fixes in the new file -- the B37 lesson, 8th strike: MoE-Infinity et al. need backticks in //! prose) + retrieval floors 14/14 HELD (no re-pins) + score-bench #25 FLAT (96.23/1.89/100/100). Count pins walked: tests.rs corpus-count + expected-array D 85->93 + sums x2 (422) + router.rs 461 + mod.rs header/D-comment + rust_perf corpus.rs (97 entries / 40 annotated / 13 NotBenchable / io 5 / fn name / 84-benchable header). Landing notes: entries emitted by a Python generator per the B77 discipline; TWO generator bugs caught by the compile gate -- open-for-write without encoding="utf-8" TRUNCATED mod.rs mid-write on the cp874 codec crash (recovered via git checkout + re-applying manual edits; the two-failure-mode lesson) and a doubled .to_string(),, from composing block_lit + explicit comma; a 423-vs-422 arithmetic transposition in the plan caught by the category-sum gate before commit. Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Research 112 / Plan 058.
                                                   Batch 98 (2026-09-02, Research 118 -- the antirez/ds4 dense.metal/moe.metal distill, Plan 065; antirez/ds4 @ 110afdd8886586f18fc9b28bc5533152dd10e728 -- Batch 97's pin, quotes re-verified byte-accurate at mine time; clone ../vendor/ds4; consumes the NEXT BATCH LEADS Batch 97 queued in the snapshot + the 2 P8-intake candidates + 3 additional candidates; 10 rules, corpus 435 -> 445, A 99 -> 100, B 55 -> 57, C 38 -> 39, D 94 -> 95, E 28 -> 30, H 15 -> 16, L 12 -> 13, M 12 -> 13): `grid-range-split-verbatim-impl-composition` (C; K same-impl-family sibling dispatches fuse into ONE kernel: grid split at the FIRST range's own boundary tgs0 = ceil(width0/NR0), re-based group ids, a BY-VALUE patched args copy [never mutate the shared constant args -- the mine-time correction: 'args aliasing' is a by-value copy, not pointer-casting], the identical impl verbatim per range so the composition is bit-identical BY CONSTRUCTION, and a DEVICE-scope barrier + volatile reloads before any epilogue that re-reads the composed outputs [same-threadgroup store->reload visibility; the epilogue half is B97 bit-exact-fusion's mechanism at same-threadgroup scope]; the 5-way sibling embeds the whole 4-way body verbatim; distinct from B58 launcher-level segments, B63 weight-concat [impossible: separate route-time buffers], B97 shared-staged-tile [no tile; impl reuse via grid partition], B26 host pass merging [one launch, strictly stronger], B97 virtual-folding [no reduction topology touched]), `share-activation-stream-keep-projection-reductions` (A; the NON-MERGE boundary of the same-input family: same-activation matvecs share the activation WALK but keep per-stream accumulators/scales/reduction shapes verbatim -- 'the point is not to fuse two independent weight streams into one matmul'; forced when a same-row elementwise epilogue [SwiGLU silu(gate)*up] needs both projections in ONE thread -- weight-concat would scatter row r and row r+N into different threadgroups; the BOUNDARY COMPLEMENT of B63 [concat for per-row independent consumers, share-the-walk for same-row epilogues -- they compose into a decision]; distinct from B97 shared-staged-tile [the GEMM-side member, barrier axis; this is the GEMV-side member, registers only]), `tiny-rows-scalar-thread-per-row-downshift` (E; K < warp width: the simd-split GEMV pays a two-barrier two-stage tree to combine partials of a <32-FMA dot -- the right arm is a SIMPLER KERNEL, one thread per output row, serial K, zero smem, zero barriers, host gate in_dim < 32 with nsg=1 smem=0; parallelism DOWN-shift, the threshold derives from the warp width not a tuned constant; batch is a DIFFERENT axis [routes to R4's bridge]; bench-note: no in-file perf numbers -- structural argument, bench-pin before promotion; distinct from B23 [specializes INSIDE the parallel family; this downgrades the thread organization], B77 [K-depth axis vs batch-width axis], B39 [host cache cost model]), `small-batch-matvec-gemm-bridge-rung` (M; the GEMV<->GEMM middle rung for small prompt batches: 2D in-simdgroup lanes [tx = lane%nxpsg works K, ty = lane/nxpsg takes MORE ROWS], each thread dequantizes its K-chunk stripe ONCE and dots it against ALL tokens held in registers, partial-lane conditional shuffle reduction [no smem], tx==0 stores; a three-way ladder n_tok==1 GEMV / n_tok<=cap [F16 gate 8 + alignment precondition; Q8_0 env-tunable to 16 with r1ptg=4 batching above] -> bridge / larger -> GEMM; HONEST CAVEATS in-rule: the source's comments say 2..8/2..5 but the live gate is env-tunable -- never hardcode the comment's range as the regime; and the middle rung is NOT universally the winner [measured Q4_K counter-example: ext ~220 GB/s vs classic 530-650 GB/s on M5, host intercepts back -- re-measure per format+silicon]; distinct from B77 batch-width twins [two tile shapes of the SAME GEMM vs different kernel ORGANIZATIONS with a dequant-amortization middle], B23 [adjacent only]), `moe-routed-down-sum-single-accumulate-pass` (H; routed top-k fan-out whose k outputs are SUMMED into one shared row-space: k dispatches + the combine kernel -> ONE kernel -- per-simdgroup row block, sequential slot walk accumulating into the SAME register accumulator [no cross-lane traffic between slots], ONE final cross-lane reduction per row; preconditions: consumer SUMS the k outputs, k small [6] + decode-shaped [n_tokens==1; prefill keeps the per-expert parallel path], NOT BIT-IDENTICAL [summation reordered: per-expert trees + slot combine -> slot-order accumulation + one tree -- tolerance-gated, explicit contrast to the boundary-emulation family]; OUTPUT-SUM composition -- slots have DIFFERENT inputs AND weights, not a same-input fusion; TP variant folds a per-row addend [shared-expert partial] into the final store; distinct from the B15/B58/B63/B97 same-input family, B97 expert-major work-map [map builder vs id consumer], B92 stripe-semaphore [no cross-block reduction here]), `fused-vector-site-transcendental-form-nonportable` (B; the P8-intake M5 sigmoid candidate, measured INVERTED at mine time: the algebraically-bounded tanh-identity sigmoid [0.5*tanh(0.5z)+0.5] produced NON-FINITE logits at the FUSED float4 decode sites on M5 Max while the direct form 1/(1+exp(-z)) stayed finite and is the throughput baseline [direct is IEEE-finite for all finite z: exp->inf gives 1/(1+inf)=0; non-finites need reassociation to exp(z)/(exp(z)+1)]; the discipline: transcendental form choice is a per-call-site, per-vector-width, per-silicon CONTRACT -- re-validate at the exact site, inline the validated form AT fused sites so a compile-time gate flip cannot reach them, gate alternatives behind a named flag for same-binary A/B, and encode the OBSERVED behavior not the assumed mechanism [the source's own overflow prose is motivation, the measured tanh-at-fused-sites failure is the contract]; tanh form grep-verified UNIQUE to dsv4_hc.metal; distinct from B32a [sign-branch = algebraic argument bounding -- the rules compose], B29 fast-math flag [global gate vs per-site form], B35 [derivative recovery]), `monomorphic-hot-caller-last-hit-memo` (D; the P8-intake pipeline-memo candidate, corrected at mine time: the memo does NOT memoize pipeline CONSTRUCTION [the generic g_pipeline_cache already did] -- it memoizes the LOOKUP [NSString key formatting + dictionary probe] behind a single-slot last-hit check [two scalar compares], miss falls through to the generic path which back-fills -- the memo is in front of, never instead of, the generic cache; preconditions load-bearing: the caller is MONOMORPHIC across the hot loop [alternating callers degrade silently to the generic path], the VALUE is an expensive object while the avoided cost is the LOOKUP [the complementary end of B39's cost model: rebuild-when-payload-tiny-and-lookup-cheap vs memoize-lookup-when-key-materialization-dominates-and-payload-expensive], pointer key fields need NULL guard + strcmp; env kill-switch for same-binary A/B [present at 3 of 4 sites]; distinct from B26 prebuilt bind groups [eager init-time prebuild vs lazy first-hit memo], B26 host-cache weights [data vs handle lookups], B29 allocation-identity keys [pointer keys vs scalar-arg last-hit]), `per-row-scale-folded-into-rhs-staging-tile` (B; a per-row RUNTIME activation scale folds into the EXISTING F32->narrow RHS staging boundary -- one fused multiply before the narrow store kills the separate scale-materialization pass; placement contract: mirror a donor kernel byte-for-byte so the ONLY arithmetic change is WHERE the F32-to-F16 rounding lands [the source's own comment]; NOT bit-identical to scale-then-matmul -- a different product gets rounded -- the lossy-surface rule, behavior-gated; placement taxonomy vs B29 delay-rms-scale [AFTER the GEMV epilogue, output side -- this is BEFORE the matmul, input side: opposite sides of the matvec] and B36 constant-scale-into-upload [constant weight hyperparameter vs runtime per-row activation]), `dequant-byte-pairs-as-aligned-u16-loads` (L; byte-pair dequant loads as aligned u16 -- 8 loads replace 16 per Q8_0 block -- sanctioned ONLY with (a) the alignment invariant DERIVED from struct layout and stated [a half scale field precedes the byte array => 2-byte aligned], never assumed from a runtime pointer value, and (b) a bit-identical load-width TWIN contract [same per-element narrow + scale multiply, only the load width changed -- never a reindexing]; distinct from B51 container-word unpack [REINDEXES to kill reload/div/mod; this keeps per-element indexing], B20 lane-pinned sub-block coalescing [cross-lane vs within-thread]), `cross-simd-bitonic-alternating-bank-exchange` (E; workgroup-wide register-resident bitonic selection: intra-simd steps ride shuffle-xor [never TG memory], ONLY cross-simd steps use TG banks -- DOUBLE-BUFFERED into two banks selected by cross-stage PARITY, ONE publish barrier per cross stage, NO read-back barrier; the safety argument is stage-distance-based [a bank is rewritten only two cross stages later, the intervening publish barriers proved readers done] and is invalidated by ANY barrier-count or stage-schedule change; one-workgroup regime, serial top-k extraction fine for small k; the complement of B57 smem-slot-role-reversal [B57 ADDS barriers at role reversal; this REMOVES read barriers via parity double-buffering -- both coexist, they govern different access patterns]; distinct from B92 stripe-semaphore [cross-BLOCK device-memory reduce vs intra-workgroup exchange]). Riders (4): B97 bit-exact-dispatch-fusion += the SCOPE-EXCLUSION half [denominator/order-sensitive stage feeding L>=~10 stacked consumers stays OUT of mechanically-free fusion unless the rounding boundary is pinned -- the 43-layer MoE router: dsv4_misc.metal L4874-4881 exclusion + L5065-5069/L5160-5163 pinned rungs] + 2 more external instances; B35 mask-sentinel += the POINTER-STATE complement [ragged paired kernels point out-of-range lanes at the matrix BASE so one loop body serves both, reductions gated by active flags incl. skipping the dead arm's reduction -- dense.metal L233-239 + L288-289]; B32a sign-branch-stable-sigmoid += the LINEAR-TAIL member [softplus select(log(1+exp(x)), x, x>20) removes the overflow domain branchlessly -- accuracy+domain joint threshold, dsv4_misc.metal L5178-5183]; B97 proven-shape-static-sibling += EXTERNAL CONFIRMATION [the sum6 static sibling: host-proven literal shape, byte-identical walk, identical-results fallback -- moe.metal L6498-6503]. Dropped with reasons: virtual-simdgroup cohort packing [dense.metal L559-561 -- B97 virtual-thread-folding instance]; host shape-keyed nr0=4 override [ds4_metal.m L19590 -- B23 instance]; sentinel-pointer standalone [rider-scale -> B35 rider]; softplus select standalone [rider-scale -> B32a rider]. GOAT: batch98_rules_present + batch98_concepts_covered + batch98_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal lib 1255/0/1i [1252 + 3 new gates; ALL count pins green] + default lib 1008/0 unchanged + clippy -D warnings clean BOTH states + retrieval floors 14/14 HELD [~307 s, no re-pins] + score-bench #29 96.2/1.9/100/100 FLAT. Count pins walked: tests.rs corpus-count 435->445 + expected-array A/B/C/D/E/H/L/M + sums x2 + grouped x2 + router.rs expert total + mod.rs 435-rule doc line + 8 category headers + mod/extend wiring. Gate-catch: one case-mismatched needle [B51 lesson, 11th strike]. Full CI guard not run this cycle [the B95 honest-skip shape]. Record: Research 118 / Plan 065.

                                                      Batch 108 (2026-09-05, Plan 078 -- the riir-ai M3 campaign measured-GOAT distill, prefill GEMM + decode attention; M3 box, CPU-only; the FIRST in-source batch since Batch 63 -- the entire M3 prefill/decode campaign landed un-mined while the program pivoted external B64-B107; source riir-ai @ 59a172623bb3748ed49061e2202b787b4e4e3f08, committed state only): 4 rules, corpus 490 -> 494 (A +1, E +2, G +1; entries_gpu_21.rs). Files mined: gemm_ternary_simdgroup_smem_cubecl.rs (Issue 771 T1 + 843 T3), gemm_ternary_simdgroup_cubecl.rs (Issue 768), cudarc_kernels/attention.rs (Issue 779), qwen_attention_cubecl.rs + tests/probe_831_attn_pair_time.rs (Issue 831 / Bench 831). Every rule carries a measured verdict from OUR OWN silicon -- the numbers belong to Benches 774/790/804/831 and are cited as records, never re-measured (the plan's non-goal). Two verification lanes read the pin byte-accurately pre-plan; coordinator spot-check at landing: A-D all VERIFIED, two honest offsets recorded (the Bench-756 -3.4% cross-substrate line sits at qwen_attention_cubecl.rs:1319 not in the toggle block; the ~1.8x/valid_n/geometry-scan NUMBERS live in qwen_attention_cubecl.rs:940/:1017/:1090-1092, the probe file carries the test + the TIE record at :16). THE RULES: `smem-envelope-gate-kernel-variant` (G; the Issue-779 derived capacity predicate 32*(head_dim+4) <= head_dim*33 admitting the k4 smem-layout variant at ALL eight rows-family launchers via ONE verify_kf4_for helper -- below the threshold the staging overflows into p_smem and phase B races row-0 score writes barrier-free (hd-64 fixtures corrupt nondeterministically; production hd-256 safe only by slack 8320 <= 8448); fallback arm bit-identical (load width only); predicate deterministic in head_dim -> capture-safe; companion dead-lane V-read guard), `multi-simdgroup-shared-staged-tile` (A; the Issue-771-T1 smem 128x64 GEMM -- ONE 256-thread workgroup = 8 simdgroups computes ONE output tile, W+X staged ONCE cooperatively per 32-K window and consumed via per-sg slice bands, per-sg PRIVATE 8x64 result regions ('one shared tile would race across the 8 sgs' stores'), TWO barriers per K window, 26.6 KB of the 32 KB Apple budget, regime m>=128 && p>=64, bit-identical by construction incl. the signed-zero tail-skip exactness), `independent-accumulator-chains-hide-load-latency-at-capped-occupancy` (E; the Issue-831 4-chain unroll -- 4 independent K-dot partials = 4 loads in flight per thread, the ONE lever that moved multi-wave lengths (~1.8x @4096, 466->263 us/call; FLAT @2048 single-wave -- both regimes are the content); valid_n loop bound replaces the per-iteration guard; tree finish (d0+d1)+(d2+d3) reassociates -> tolerance-band-gated, never bit-identity; the REGIME BOUNDARY: B44's lane-split redesign measured a TIE at production occupancy (median 1.01-1.06) -- axis splitting buys nothing at a workgroup-supply cap), `byte-cut-loses-when-plateau-is-issue-bound` (E, refutation-type; the Issue-831 packed-f16-KV arm -- built, G0-exhaustive-65536-bit-exact, G1 band pass, and measured 1.44-1.60x SLOWER at every length: the plateau sat at ~114 GB/s vs ~400 GB/s peak with supply capped, so halving bytes buys nothing while the in-kernel f16->f32 decode ALU adds unhideable issue pressure; Bench-756 4090 -3.4% cross-substrate corroboration; the regime test BEFORE any byte-cut: measure GB/s-vs-peak + occupancy supply; re-arm on a mechanism change only). RIDERS (5): B01 row-tile-amortize-input-load += the CMMA STAGED-GEMM IN-STACK INSTANCE (Issue 768 32x32: input tile consumed across 4 M-subtiles, 42-63% measured input-load share, regime m>=64 with the 0.773x-at-m=48 counter-example, bit-identity argument S1:938-940 + the untried-axis note S1:563-565); B06 row-dot-split-inner-axis-block-reduce += the CAPPED-OCCUPANCY REGIME COMPLEMENT (the lane-split TIE record, probe :13-17); B57 smem-slot-role-reversal-needs-barrier += the EXPLOITATION HALF (Issue 843 T3 staged epilogue: dead W staging tile reused as the 8 per-sg epilogue regions, tile_w[sg*512..+512), 32 barriers -> 4, overwrite-only-after-half-1-drained, +2.5-2.8% @16K / +4.5% @8K, bit-identical, default ON); B79 first-order-affine-recurrence-associative-scan += the BREADTH CONFIRMATION (arXiv:2608.17641 rl-triton -- GAE/V-Trace/Retrace/TD-lambda/eligibility traces are the SAME first-order recurrence, O(log T) scan, 1.6-5.7x over torch-compile; riir-train GRPO/PPO the potential consumers); B82 wy-transform-tree-verify-state-free += the EXTERNAL CONFIRMATION (arXiv:2608.01651 Bole -- token-level factors = the pseudo-value matrix, reconstruct-only-sampled, 82-99x transient-state + 3.4-7.7x tree-verify SGLang-integrated, reconfirming the memory-pressure conditionality; the queued land-with-next-adjacent-batch riders DISCHARGED here). RECORDED-NOT-LANDED (batch record only): co-resident-wave dispatch-order axis swap REFUTED (-6..-7.5% everywhere; W-slab-sharing order correct; S2:40-42 -- adjacent to B9 swap-large-axis, rider if a second instance appears); the 4-arm dispatch ladder (8x8/8x32/32x32/smem, hand-measured per arm -- in-stack instance of B104 swept-shape-bucket-launch-tables ethos); empty-partial reduction identity (S4:975-988 -- B35 instance at partial-buffer grain). GOAT: batch108 gates 3/3 FIRST-SHOT + kernel_heal lib 1349/0/1i (baseline 1346 + 3) + default lib 1078/0 unchanged + kernel_heal+latent 1457/0/2i (floors HELD zero re-pins) + clippy 0 in three states + all-targets kernel_heal (7 doc_markdown backtick fixes in the new //! header -- the B37 lesson, 12th strike). Count pins walked: tests.rs 490->494 total + message + batch-list + current-494 comment + expected-array A 108->109 / E 35->37 / G 26->27 + sums x2 + grouped + the stale 464 cover-all comment repaired + router.rs 490->494 + mod.rs doc line + mod/extend wiring + 3 category comments + 3 member-list tails. Gate-catch: the B18 full-concept pin on row-tile-amortize-input-load tripped on the rider -- moved WITH the rider per the B22 precedent (the gate doing its job). Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Plan 078 / Research 131 (in-source -- no .raw clone to remove; the sources live in the sibling repo at the pin).
                                                      Batch 110 (2026-09-05, Research 133 / Plan 081 -- the vLLM libtorch_stable distill, the item-11 online-source search executed via the LOCAL-CLONE route while the MCP search quota stayed dead until 09-15; 4090 box, CPU-only): 11 rules, corpus 494 -> 505 (E +4, D +2, B +2, C +1, K +1, N +1; entries_gpu_22.rs). Source vllm-project/vllm csrc/libtorch_stable @ 8277c42e4c74c5dd604f29b6d938a3456658f247 (Apache-2.0, never mined -- prior contact is B100 reshape_and_cache citation + league timing only). Two inspection lanes (A: decode-fusion/MLA/GDN/KDA 8,366 lines; B: topk/sampler/MoE/cache 14 full + 9 partial) + coordinator byte-verification of 13 lead quotes; the batch lands the VERIFIED A-/B+ tier of a 25-candidate pool (the B-/C+ remainder = Batch 111). THE RULES: spin-barrier-needs-co-residency-capped-launch (E, the GOAT lead -- occupancy-query the EXACT instantiation, cap at sms*occ, headroom = sms when occ>1 else 1, never below one group, fall back to the non-barrier kernel; the B58 correctness MIRROR -- uncapped stays legal when the barrier is unreachable; gate batch110_spin_barrier_launch_contract pins the arithmetic both directions), captured-graph-init-unconditional-and-stream-ordered (D -- node set frozen at capture: a host-guarded init omitted at capture is a replay-time bug; cross-CTA init needs the stream-ordered edge an intra-CTA barrier cannot provide; both manifested in-source, one fix), ballot-matrix-tie-rank (E -- lane i holds tie i, warp j evaluates tie j, popc(ballot) IS the rank under the (score desc, idx asc) total order; 32x32 = 1024 comparisons in one instruction per warp, O(1), no sort no atomics; regime manufactured by coarse-bin arithmetic), m-group-shared-weight-scan-register-relief (E -- skinny-GEMM register relief: kTGroups thread groups in ONE block walk the SAME weight K-slice sequence, group 0 misses the rest hit L1, weight traffic 1x, accumulators shrink kTGroups-fold; measured 1.26-1.66x with honest non-uniform rows + the in-source revision history; OUR router-GEMM shape), softmax-denominator-cancels-through-scale-invariant-norm (C -- (acc/s)*rsqrt(mean((acc/s)^2)+eps) = acc*rsqrt(mean(acc^2)+eps*s^2), zero divides, one rsqrt, tolerance-gated never bit-identity), batch-invariance-locks-reduction-geometry (D -- reduction geometry independent of batch composition under a bit-exact contract, disabled by the SAME expression; vectorization precedence explicit; Issue-845/850 kin), rope-pair-partner-via-cross-lane-shuffle (B -- align per-lane stride to the pairing structure, one shfl_xor fetches the partner, sign fold replaces branching; half % elems_per_lane == 0; two independent in-source instances; NOT WGSL/Vulkan), row-parallelism-ladder-split-vs-pack (E -- warps-per-row is a function of row count: split 3-wide at decode <= kDecodeRowSplitMaxTokens=64, one-per-row at parity, pack 2/warp + grid cap + grid-stride at overflow), persistent-kernel-length-bucketed-mechanism-ladder (K -- per-ROW mechanism dispatch with DERIVED thresholds (8192/2048 = 4 elems/bin), heavy state in dynamic smem, rare paths noinline; vs B104 host-scope swept tables), router-math-stays-fp32 (N -- selection-critical math fp32 end-to-end, affordable because the GEMM is skinny M<=32; per-ROLE precision placement; H3 cross-tier divergence hazard recorded), degenerate-selection-falls-back-to-identity-uniform (B -- neutralize NaN/Inf BEFORE selection to compose with the tie-break: 0 + lower-index-wins -> identity [0..k), + uniform 1/k weights; 3 independent in-source arrivals with documented crash modes). RIDERS: DEFERRED to the Batch 111 rider sweep (the ~30-instance inventory preserved verbatim in Research 133 -- highest-value: the hi/lo packed-u64 redux decomposition, the inverted-index pack, the phase-counted barrier, B57 union-across-phase; avoided 16 fragile edits on 2,000-char single-line strings late in the session). UPSTREAM HAZARDS (Research 133, not ours to fix): H1 silent tie-buffer overflow drops (4 sites, __builtin_expect(pos < CAP, 1)), H2 VLLM_STABLE_DISPATCH_GROUP_SIZE falls through with no default arm (the silent-zero class), H3 cross-tier router numeric divergence ((sigma+b)-b != sigma in fp32 -- OUR router code relevant), H4 KDA PDL trigger fires before its output stores (contradicts the sibling MLA kernel own contract), H5-H8 minor. GOAT: batch110 gates 4/4 FIRST-SHOT (rules_present + concepts_covered + pseudocode_parses + the spin-barrier contract gate) + kernel_heal lib 1357/0/1i + default lib 1082/0 + kernel_heal+latent retrieval floors HELD zero re-pins (kernel self-variants 100/100/100, blended 94.7/96.9/97.7) + clippy 0 real findings both states (1 doc_markdown backtick fix in the new //! header -- the B37 lesson, 13th strike; 1 too_many_lines allow on the concepts gate per the batch98 precedent; the exFAT hard-link noise excepted). Count pins walked: tests.rs 494->505 total + message + batch-list + current comment + expected-array B 57->59 / C 43->44 / D 112->114 / E 37->41 / K 7->8 / N 29->30 + sums x2 + grouped + cover-all comment + router.rs 494->505 + mod.rs doc line + mod/extend wiring + 6 category comments + 6 member-list tails. Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Cleanup: .raw/vllm removed at landing (the .raw discipline). Record: Plan 081 / Research 133.

                                                                                                            Batch 107 (2026-09-05, Research 130 / Plan 077 -- the uzu + lily defers batch, the B105 deferred queue consumed; 4090 box, CPU-only): 8 rules, corpus 482 -> 490 (A +2, C +1, D +1, E +2, H +1, N +1; entries_gpu_20.rs). Sources re-cloned at the verdict pins: trymirai/uzu @ 115609b (Release 0.5.23, MIT) + perplexityai/pplx-garden lily @ 1ed972ed (Apache-2.0). Two subagent lanes re-verified all 12 defers; coordinator spot-verified 6 quote clusters (all byte-accurate). THE RULES: `sparse-demand-paged-kv-cache` (A; B+ -- placement-sparse KV buffers, full virtual extent / physical pages mapped as the gap-diff per prepare, address never moves, one shared heap pool, the MTLHeapType::Sparse-is-for-textures trap), `gumbel-argmax-filtered-sampling-descent` (C; B+ -- Gumbel-max + in-kernel top-k/p/min-p filters + bounded descent on filter failure, clamped uniform, fixed Philox field = fixed Plackett-Luce order), `packed-total-order-multi-field-argmax` (E; B- -- (score,index) argmax as ONE reduction via a u64 packed key; the router_topk two-reduction + 0xFFFFFFFF sentinel is the in-source anti-instance), `integer-mma-span-epilogue-dequant` (N; B- -- int fragments per SPAN = min(left, right group), dequant at the span boundary, activation zero-point from precomputed group sums, host-decided parity-banked scale residency; most prefill-relevant of the batch), `single-threadgroup-fused-count-scan` (H; C+ -- MoE prologue count + raking scan + total + tile partials in ONE kernel ONE threadgroup, honest decode-side regime, upstream E>128 defects recorded in-rule), `radix-select-ordered-key-topk` (E; B- -- vocab-scale top-k as MSB-first radix SELECT over packed u64 keys, arrival-counter histogram merge self-cleaning, pass count from index bits; the verdict-time size-router claim CORRECTED to coexistence-as-design-evidence), `prefill-chunk-command-buffer-capacity-coupled` (D; C+ -- lily PREFILL_CHUNK 4096 = scratch capacity, boundary = only sync AND the upload window, no re-zero via write-before-read, drop-before-allocate growth), `tensor-op-strided-device-operand-binding` (A; B- -- Metal-4 tensor ops bind by residence: device-resident -> strided views with NO k-loop, produced -> smem, freed budget funds softmax state; the B35 staging tension cited deliberately; + the nax_version compile-probe rider on B105). RIDERS (corpus text): B63 deferred-residual += lily decode-graph instance + the measured bf16-ULP compiler-dependent contract; B95 whole-sequence += gdn_prefill_regscan second implementation + the simdgroup-uniform-guards complement. DROPS: flat-topology-fast-path (B23-family batch-trie specialization, no bounded fix-space), RIF-sentinel (born dead -- sole instantiation RIF=1/SERIAL=0, sentinel unreachable, zero-filled cache), weight-hygiene cluster (B63/B31 external instances + one out-of-domain loader gate). UPSTREAM HAZARDS: uzu counts_offsets_fused E>128 CONFIRMED + EXTENDED (the :118 chunk-read reads counts_shared[lid] not [base + lid] -- offsets wrong for E > 128; parity suite max E = 128 while wiring admits 512 -- the fixture-population class), weaver_top_children unclamped Gumbel (u32 -> f32 can round to exactly 1.0 -> -inf -> silent always-lose bias; the clamp exists at the OTHER Gumbel site), unified_sampling bounded-fallback semantics note, lily descending_blocks shipped-but-unarmed knob. GOAT: batch107 gates 3/3 (one concepts needle caught pre-landing -- case mismatch, the B51 lesson) + kernel_heal lib 1346/0/1i + default 1078/0 + kernel_heal+latent 1454/0/2i with retrieval floors HELD zero re-pins + clippy -D warnings 0 both states (one doc_markdown backtick fix: counts_offsets_fused in the new //! header -- the B37 lesson). Count pins walked: tests.rs 482->490 total + message + expected-array + sums + cover-all comment; router.rs expert total; mod.rs doc line + 6 category comments (the N comment was stale at 27 pre-batch, actual 28 -- repaired to 29). Terminal hazard re-confirmed: this box drops command output non-deterministically even for single redirects -- log to /tmp, cp into the gitignored target/, read via file tools; redirect order matters (> f 2>&1, not 2>&1 > f). Record: Research 130 / Plan 077 (.raw/uzu + .raw/pplx-garden removed at landing).
                                                                                                             Batch 106 (2026-09-05, Research 129 / Plan 076 -- the MTPLX upstream residue distill, the queued NEXT-UP row consumed after B105; 4090 box, CPU-only): 5 rules, corpus 477 -> 482 (D 106 -> 111; new slice entries_gpu_19.rs). SOURCE youssofal/MTPLX upstream main @ `dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d` (tag v2.11.1, Apache-2.0) -- the three surfaces Batch 103 never read: the upstream `mistakes/` post-mortem corpus (16 files; the filename IS the rule -- highest confidence-per-token axis in the program), docs/plans+specs+concurrency (2,233 lines), and the targeted kernel diff since the mined base `8bc4d88`. EN-ROUTE: fresh FULL clone required (the queue's depth-1 note) + `git config core.longpaths true` (mistake filenames exceed Windows MAX_PATH -- clone checkout FAILED twice before the fix); the mod.rs "464-rule" doc line found 2 batches stale (post-B104/B105) -- repaired to 482. THE RULES: `prefix-reuse-key-must-mirror-the-restore-predicate` (D, the B+ lead CONFIRMED with BOTH instances in-source -- the over-keying half: the policy_fingerprint carried tool_choice + per-request contract flags while those bytes rode a transient trailing sentinel, so every transition round re-prefilled 41k cold (41 s + 44 s), fix contract verbatim "the fingerprint only has to carry what changes the KV or the MTP history WITHOUT changing the tokens"; the under-keying half: the 2.10.2 admission shed asked longest_prefix EXACT containment while restore serves by BLOCK prefix -- commit dd4031a8 itself -- 41,901 tokens re-prefilled cold / 54 s with a 41,391-token block restore available, fix "ask the bank the question the restore asks"; the union-of-serving-modes reuse predicate; in-stack analog Issue-742 GDN+KV prefix cache 441x TTFT; sibling of B103 shape-class one layer up), `request-arrival-work-asks-reuse-state-first` (D -- the PLE first-gather-early lane gathered 650k n-gram rows at arrival predicted from the prompt alone and blocked the owner thread at scope exit; warm turns start prefill 40k later -- "the gather was pure waste and its wait was pure dead time" (5-7.5 s/turn, 21 tok/s measured on 200-token turns); fix = ask the cheapest reuse predicate (shares_ram_prefix, one tuple-slice compare per entry, duck-typed no-probe-no-preempt default) + NEVER AWAIT UNCONSUMED WORK; commit 6fd0c67), `serving-raises-only-behind-strict-flag` (D -- the engagement verdict raised on the live serving path and a user got finish_reason "error" after a 30 s cold prefill of 31k tokens because one low-entropy chunk was declined; the verdict-delivery policy: measurement arms export STRICT and get the raise, serving records counter + ONE warning per verdict class per process ("a serve that hits this on every request must not turn its log into the receipt it failed to be") + the pre-arming raise-grep + the per-dtype fail-closed lane state (FAILED terminal, traffic re-routes, error re-raised never swallowed); COMPOSES with B103 request-shape-decline-mirror-completeness's armed-lane-must-engage raise -- one policy keyed by context, cross-referenced both directions), `derived-key-single-constructor-parity-test` (D -- the 2.8.0 wizard rebuilt the tune-record settings dict by hand, its lookup hash never matched the save side, every start re-offered tuning (issue #280 "machine heats up with zero requests"); ONE constructor every reader+writer calls + a drift-failing parity test + provenance markers for non-CLI values ("a config value that happens to EQUAL the parser default is still the user's standing pin"; the wizard lost the picked model -- picked 4B, tuned 27B); cousins B29/B39/B94 fix key CONTENT, this fixes the DERIVATION TOPOLOGY), `vendor-fused-op-instantiation-check` (D, axis-A find -- before adopting/benchmarking a vendor fused op, read the shipped binary's ACTUAL instantiated specializations: MLX sdpa's fused kernels are instantiated only for head dims 64/96/128/256 (+192 in 0.32), the MLA latent is 512 wide, so the "fast path" measured the vendor's own unfused fallback while the team believed it was measuring fusion, and the hand-fused arm wins as "that same fallback minus two copies"; consumer-side complement of B23 + the vendor-shaped cousin of B103 capability-keying; in-stack analog Issue-731's fast-path coverage hole). Riders (no corpus rules): p99-inter-emit-gap census ("felt smoothness is the p99 inter-emit gap" + "gate ceilings must sit just above the measured-good distribution" -- league-doc citations, external confirmation of the Issue-853 percentile doctrine), stale-binary provenance (resolved-binary-path + probe-for-a-marker-only-the-new-code-has -- Bench-749/Issue-866 confirmation), git-stash-shared-by-every-worktree (refs/stash is ONE stack for all worktrees; git-show swap + fsck --unreachable recovery -- our AGENTS.md stash rule + hash-object fix confirmed externally), delivered-cadence-vs-server-TPS (the uncapped-client-path rule + read1()-not-read(n) probe honesty), census-vs-E2E 4th instance ("the census establishes dispatch structure only and is not presented as E2E timing"; 91% attribution). 11 mistake files dispositioned out-of-scope in Research 129 (AppKit/QA-hygiene classes); `docs/perf/` subdirs + `docs/research/` recorded as the named-unread remainder. GOAT: batch106_rules_present + batch106_concepts_covered + batch106_pseudocode_parses (cfg(not(kernel_verify)) per B70) 3/3 FIRST-SHOT + kernel_heal lib 1343/0/1i + default lib 1078/0 + kernel_heal+latent 1451/0/2i (floors HELD zero re-pins) + clippy 0 --lib and --all-targets both states (the exFAT hard-link cargo noise diagnosed via cargo clean -p rebuild -- zero real warnings). Count pins walked: tests.rs total 477->482 + message + batch-list + current-482 comment + expected-array D 106->111 + sums x2 + grouped + router.rs 477->482 + mod.rs D header/list + the 464->482 doc-line rot repair. Record: Research 129 / Plan 076 (.raw/mtplx-upstream removed at landing).
                                                      Batch 105 (2026-09-05, Research 127 + 128 COMBINED / Plan 075 -- the uzu + lily Metal-engine batch, the mine-time cross-source mandate executed as COMBINE per the B103 precedent; M3 box, CPU-only; two read-only inspection lanes (subagents) re-verified every quote at both pins before any corpus edit): 11 rules, corpus 466 -> 477 (A +1, C +1, D +3, E +1, F +1, H +3, M +1). SOURCES trymirai/uzu @ 115609b (Release 0.5.23, Apache-2.0; 10 verdict-time candidates + 8 riders ALL confirmed -- 2 with corrections) and perplexityai/pplx-garden lily/ @ 1ed972ed (Apache-2.0, `lily/` only; 4 candidates confirmed). MINE-TIME FALSIFICATIONS (3, all carried honestly): uzu "RELAXED everywhere" REFUTED by the strict-layout override arm (gdn/tree_verify/out.metal -- relaxed MPP diverges from FP32 across layers; the override became rule 6's precision-keyed decision content); the "Hadamard of a constant column" comment DOES NOT EXIST in the uzu tree (the bias-demotion CODE is the evidence; the quote killed, never landed); lily's tile policy is CHUNK-SIZE-routed (T32Sg4/T64Sg4 at chunk_m >= 3072, BOTH tiles 4 simdgroups) NOT expert-load-routed -- the R128 B77 rider framing corrected, blog +13.2%/fixed-16 numbers external-only; plus the family-gate refusal text names the requirement ("family 10, M5 and later"), not the refused family. THE RULES: `concurrent-dispatch-dependency-level-barriers` (D, the lily GOAT lead -- record the decode DAG as dependency LEVELS, one MTLDispatchType::Concurrent pass, full-scope barrier ONLY at true inter-level edges, the _written tensor list deliberately ignored documentation, independent chains encoded interleaved so they share levels, serial encoders ignore the barrier, ONE code path both modes; 795 kernels -> 555 stages; wgpu has no concurrent-dispatch equivalent -- the raw-Metal lane is the sanctioned escape; the M3 A/B fixture DEFERRED Plan 075 T7, GPU lane claimed by 851 Phase A at landing), `gpu-resident-moe-routing-no-host-sync` (H, lily -- the whole routing pipeline GPU-resident in ONE command buffer, zero-fill reset rides the pass (+2 kernels, the only literal match for the blog's claim), sentinel-padded capacity bound ceil(S/T) + min(E,S) replaces the readback, consumer-side desync guard (a policy mismatch drops rows SILENTLY), no-rezero write-before-read reuse; prefill counting-sort scoped, decode union-map GEMV equally readback-free), `indirect-device-dispatch-tile-map-moe` (H, uzu C1 -- device-computed launch geometry via indirect dispatch args, only the RAGGED axis indirect (x = num_tiles_n stays host-static), over-provisioned TGs early-return so empty experts cost one dead TG row; cross-ref'd against the capacity-sentinel posture as the two-regime pair), `pipelined-token-slot-ping-pong-submit-ahead` (D, lily -- U32[2] ping-pong slots, argmax writes the next step's INPUT, submit-before-wait across two passes, the 4-byte stop-check read is out-of-band and gates nothing, pos advances at encode time, overshoot drains as a no-op), `split-k-two-pass-attention-decode-long-context` (F, lily -- fixed-block two-pass at the EXACT 32768 = 128x256 crossover (the cap is arithmetic, not tuned), per-block (m,l,acc) partials + combine re-deriving global max/sum with no KV re-read, exact_model_shape gate, MLX sdpa_vector_2pass_1 attribution), `cooperative-tensor-mma-capability-route` (M, uzu C2 -- fail-closed supportsMXU probe DEFAULT FALSE, shape-identical fallback tier, precision-keyed strict-layout override, multiply-vs-multiply_accumulate per k-step, alignment-gated packed W4 tensor operands (the C3 packed-nibble lead folded as decision content: alignment decides, not dtype), the gram.metal did-not-improve-e2e refutation carried as the per-call-site principle; B103-NAX posture), `rht-epilogue-fusion-contract` (C, uzu C4 corrected -- RHT fuses into the GEMM epilogue only when split_k == 1, transforms stripped from split-K partials, bias DEMOTED to post-transform (ordering, not arithmetic, is the correctness content), input-side RHT folds into the A8 quantize kernel; the quantized-weights claim reframed as wrapper convention), `lane-sequenced-deterministic-bucket-scatter` (H, uzu C7 -- atomic-free deterministic permute via 32-step lockstep write phase, determinism from INPUT ORDER not sorting, O(sg) barriers amortized over the following GEMM, capacity clamp off by default; lily's atomic-cursor scatter carried as the contrast posture), `a8-activation-route-stability-guard` (D, uzu C6 -- adopt the operand-class downgrade ONLY if the downgraded shape re-routes to the same optimized tier (re-run the full dispatch), centralized legality incl. min(weight_group, 128) activation group sums for zero-point formats), `target-tile-split-k-deterministic-two-dispatch` (E, uzu C5 -- TWO kernels not atomics: transforms-stripped partials + fixed-ascending-order reduce folding scale->acc->bias, quant-class target-tile table + quant-group alignment ladder + decrement loop guaranteeing k % (split_k * align) == 0; the deterministic twin of B92's semaphore), `hazard-tracked-aliasing-scratch-allocator` (A, uzu NEW-B3 -- barriers COMPUTED from a RAW/WAW/WAR range tracker (clear-on-hazard: the barrier consumes the history), scratch aliasing legality DERIVED from the same tracker state (GPU-timeline-only scratch may alias, CPU-read never), named prefill/decode pools keep epochs mode-scoped; re-adjudicated from the R127 drop row which had declined it unread). Deferred to the queue (8): uzu sparse-demand-paged-kv (B+), gumbel-argmax-unified-sampling-descent (B+), multi-field-argmax-tuple-reduction (B-), flat-topology-fast-path, integer-mma-epilogue (B-), fused-count-scan (C+), radix-select (B-); lily chunk-command-buffer-prefill. Riders recorded: B49 x2 + the Metal-4.0 pragma-unroll fact, B97 frontier-clamp, B103/B100 GQA third implementation, B104 occupancy/qmv tables, B98 GEMM-over-GEMV carve-outs, LinearAttn state-resident-chunked, Morton counter-regime, B63 load-time-concat external instance, B62 skinny-router with the bit-identity uniformity reason, B16 dequant-in-tensor-op. EN-ROUTE find: uzu counts_offsets_fused.metal counts_shared[128] threadgroup OOB writes for 128 < E <= 512 (CPU ref unbounded -- flagged for upstream, not a rule). EN-ROUTE count-pin catch (the pins doing their job): the first landing filed 4 rules into the wrong KERNEL_CATEGORIES lists (two sed views had mislabeled the Compute and L list tails as A/H tails) -- Compute 58 / L 16 vs pins, caught by kernel_categories_counts_match_plan_003_table, fixed before commit; lesson recorded: verify list IDENTITY (the KernelCategory variant above the list), never tail position from a prior read. GOAT: batch105_rules_present + batch105_concepts_covered + batch105_pseudocode_parses (cfg(not(kernel_verify))) 3/3 FIRST RUN + kernel_heal lib 1338/0/1i + default lib 1076/0 + kernel_heal+latent retrieval_precision_regression_gate PASS (floors HELD) + router 23/23. Count pins walked: tests.rs total 466->477 + message + batch-list + current-477 comment + expected-array 7 categories + sums x2 + grouped x2; router.rs 477; mod.rs mod/extend + 7 member lists + 7 category comments. Clippy both states DISCHARGED post-close 2026-09-04 by the resume session (default + kernel_heal `--all-targets -D warnings` green; `batch105_concepts_covered` needed the house `#[allow(clippy::too_many_lines)]` -- the batch98 precedent -- one-line fix, landed with the byte_char_slices discharge commit). Record: Research 127 + 128 (status lines updated) / Plan 075.
                                                      Batch 104 (2026-09-05, Research 126 / Plan 074 -- the ggml-org/llama.cpp fa-vec offline swept launch-table distill, the 09-02-queued league opponent-watch lead consumed; M3 box, CPU-only, read-only source lane): 2 rules, corpus 464 -> 466 (K 5 -> 7). SOURCE ggml-org/llama.cpp (MIT) -- infra `efeda76b9` (08-24: ggml-metal-tuning.{h,cpp} the runtime table + tools/tuning/{main,fa-vec,bench}.cpp the offline sweep harness), table pin `8963a9bdc` (08-28, #27863: the M3 Max rows -- 91 rows F16+Q8_0, the league-relevant SKU; our pinned opponent fork e311ed38f does NOT carry fa-vec, upstream-only), harness `d230ddd76` (b10793; tools/tuning/ BYTE-IDENTICAL pin->HEAD verified by empty diff; post-pin movement is table ROWS only: 1,293 -> 3,420 rows across 20 SKUs in two weeks, the community sweeping through discussion #27668). `swept-shape-bucket-launch-tables` (K, the A- GOAT lead -- when a kernel's optimal launch config is non-monotonic across device x dtype x shape x axis buckets, sweep OFFLINE and ship a constexpr data table keyed by an 8-byte packed struct ("static_assert(sizeof == 8, \"tightly packed for memcmp\")") with SIX decision contents: the fallback ladder (exact bucket -> ne11-collapsed domain default -> family-representative SKU -> template baseline), the short-axis skip ("short KV: attention is a small slice of the step, left to baseline"), consumer-side device-limit validation ("fall back to baseline (Q=1) if the tuned config exceeds threadgroup memory"), the shared legality constraint (fa_vec_legal_ne "single source shared by the offline tuner and test-backend-ops" + fa_vec_baseline_ne "hand-maintained mirror; keep in sync with those instantiations"), exception-row sparsity (defaults + exact rows only where pointwise regret > tau), and the swept-vs-derived axis split (Q/NE swept, nsg stays the runtime while-loop formula); grid derived from runtime specialization boundaries ("Bucket 0 is not sampled at all: the runtime leaves short KV at baseline, so no measurement there can reach the table"); harness contract "prints pasteable table rows ... never a pass/fail test" / "stdout carries nothing but table rows"; in-stack contrast: our tiled-flash m16/m32 single-axis single-SKU length gate), `no-harm-bucket-representative` (K, B -- a bucket representative is a WORST-CASE-REGRET contract, never best-average: "A config may represent a bucket only if it is no slower than the baseline config at every point that bucket covers" + the padding witness ("The padded-row waste ceil(n/Q)*Q/n is largest at the smallest ne01 of each residue class mod Q, so one of a bucket's first Q values carries the worst padding it can ever see, and that value has to be sampled. Otherwise the bucket bounds nothing: a config picked on the aligned ne01=8,16 says nothing about ne01=9 ... a floor on the evidence, not a proof") + two gates not one (TUNE_THETA 1.05 aggregate tunes-at-all, TUNE_TAU 0.05 pointwise rides-a-default -- "a ratio-of-sums lets a default that wins on aligned ne01 hide a large penalty on a misaligned point") + unmeasured-is-baseline ("Nothing measured, nothing proven: a bucket with no surviving sample admits baseline only"; empty bucket -> baseline "instead of letting the ne11-collapsed domain default ride in unmeasured") + anchor/cooldown drift gating with dirty-window re-measure + refusal reporting ("a sweep on another machine then shows where the winner loses") + fixture realism is timing-load-bearing (the -INF mask blocks "are what drives the kernel's skip-INF path, so this pattern is load-bearing for the timings, not just for numerics")). Dedup: anchor-shape-bucketing (L) = cache-KEY bucketing, composed not duplicated; K-family runtime-autotune rules = RUNTIME mechanics; B77 twins = compile-time tile instances; B103 shape-class = capability verdicts; zero hits under both vocabularies for swept-table ladders / worst-case-regret / no-harm / padding-witness / exception-rows. Drop ledger: sample-where-consumable folded into rule 1; harness-as-shipped-binary folded as process half; mask-timing note cited in rule 2 (B97 owns the mechanism). GOAT: batch104_rules_present + batch104_concepts_covered + batch104_pseudocode_parses (cfg(not(kernel_verify)) per B70) 3/3 + kernel_heal lib 1294/0/1i + default lib 1035/0 + kernel_heal+latent retrieval_precision_regression_gate PASS (kernel self-variants 30 labeled 100/100/100 -- floors HELD zero re-pins, blended 94.7/96.9/98.5) + clippy 0 --lib in three states (one doc_markdown fix: F16+Q8_0 backticks -- the B37 lesson). Count pins walked: tests.rs corpus total 464->466 + message + batch-list comment; kernel_categories_counts K 5->7 + sums x2 464->466 + grouped; router.rs rules_for_expert_total 464->466 (the 18th-count-pin-site lesson -- the FIRST failed run caught it); mod.rs K header comment + member list + batch-line + header-count comment. EN-ROUTE: the stale .plans/.highwater (read 72, Plan 073 existed -- the 848 class) repaired to 74 at allocation. En-route gate-collision REPAIRED IN-PASS: the batch104 gates' insertion edit fuzzy-clobbered the batch97 oracle test's header (#[allow] + fn line) and batch103's `for` loop header -- both restored byte-exact before any commit, caught by the default-build compile + the kernel_heal gate's E0061-class error. Record: Research 126 / Plan 074 (.raw/llama-upstream removed at landing).
                                                      Batch 103 (2026-09-04, Research 124 / Plan 072 -- the MTPLX PR #391 + MLX NAX combined host-orchestration kernel batch; the Plan-072 NEXT-UP unit unblocked by B102's landing, T7 default COMBINE executed: one corpus cycle, one GOAT run, one count-pin walk; M3 box, CPU-only): 4 rules, corpus 459 -> 463 (D 98 -> 102), + the B100 external-instances rider. SOURCE 1 youssofal/MTPLX PR #391 (fork davidtai/MTPLX-1 branch port/qwen38-flash-next-210-restack, Apache-2.0, OPEN/unmerged) -- T0 RE-PIN: verdict pin ef96313 moved to **a5e38bb** (pushed 09-03 09:10 -0500, '16K decode 57.7 -> 80.9 tok/s'; the branch REWRITE deleted the experimental fable_*/docs scaffolding and rewrote the PR body 09-03 18:24Z); every anchor re-verified at the new head -- TWO verdict-time anchors did NOT survive and are carried honestly in-rule: the W87 W-number attribution (body no longer names it -- recorded as NOT re-confirmable, mechanism independently verified) and the 'hard-coded draft width' body note (re-anchored to the wiring test's 'the ONE call site that could reach the verify width asked about a width the flag never armed ... read a zero row count, returned False'); the q_len x gqa_factor <= 32 dead band (sdpa_2pass_paged_q8.py:193 `if 32 * gqa_factor * int(q_len) > 1024: return None`), the W54 19.2% ceiling ('0.4 % against a 15 % gate; tile ceiling 19.2 %' / 'the ceiling is too low to fund'), and the W68 two-outage history (now first-class in-source at qsa_sparse_decode.py:340-349: 'Two request-shape raises reached production before this list existed. The first was ``k_eff != TOP_K`` (2026-09-02, HTTP 500 on every HumanEval prompt)') all VERIFIED byte-accurate): `shape-class-keyed-capability` (D, the A- GOAT lead -- a capability/eligibility verdict is valid ONLY for the shape tuple the native gate reads: key by that tuple and re-ask per shape class, import the armed-width constants rather than re-spelling them ('imported rather than re-spelled so the routing predicate and the kernel cannot drift apart'), state which axes the process-wide probe certificate covers ('The probe runs on the FIRST one only (the verdict is per process)' + the probe-geometry certificate 'a 4,096-token probe exercises the same grid a 17,408-token cycle does' -- context axis certified, width axis NOT); the owner's post-mortem rule 'a flag either works on every request path the server accepts, or it fails loudly at install'), `wavefront-overlap-pair-ceiling` (D, B -- bound the overlap-pair count BEFORE building any multi-stream/wavefront seam: a linear S-stage pipeline has exactly S-1 overlappable producer/consumer pairs, each bounded by min(producer tail, consumer head); the source declined their 2-stream x 2-stage wavefront prefill on exactly this arithmetic, and the sibling experiment priced the machinery: 'cross-stream fences cost as much as the overlap gains'; the Bench-801/803 bound-first doctrine in kernel-shaped form), `request-shape-decline-mirror-completeness` (D, B -- a routed fast path must mirror EVERY request-dependent decline branch of the native contract ('THE MIRROR. Every branch ... that depends on the REQUEST -- its context length, its row count, its block budget -- paired with the stable key'); everything else is CONFIGURATION and raises ('Anything else raises rather than falling back, so a typo in a sweep cannot quietly measure the default'; 'One key set to ``0`` on its own raises at model install'); decline SITES named not totaled ('a single total would not have shown that, so the sites are named') + reasons counted with min/max extremes so the key space cannot grow + the armed-lane-must-engage raise ('an armed arm that runs the stock chain is worse than an outage: it is a measurement that looks like a result'); the two DIFFERENT context questions (budget vs crossover) both asked ('the two questions are different, and :func:`context_decline` asks both'); mirror pinned against the native source by test). SOURCE 2 apple/mlx @ f99e916 (PR #4416, the 09-03 queued NAX lead): `backing-provenance-causal-shadowed-extent-widen` (D -- widen the K extent to the next tile multiple and read the padded rows WITHOUT copying, only after the slices PROVE the padding rows are backed (stride sanity 'if ((st[0] < 0) || (st[1] <= 0) || (st[2] <= 0) || (st[1] % st[2] != 0))' + row-pitch containment 'if (row0 + rows > pitch)' + buffer end) and causally unreachable ('The causal offset describes the true diagonal even if kL is widened below.' / 'int qL_off = kL - qL;'); the ragged-tail masked-store branch DELETED for the chunked-KV-cache shape; silent-by-design refusal at the right layer). Rider: B100 `register-budget-head-split-gqa-reread-trade` += EXTERNAL INSTANCES (NAX hd-256 dsplit: 'static_assert(WN == 2, ...)' the ONE-peer exchange s_xchg[WM][WN][2*kEPF*32] + Q-half register-residency hoisted out of the KV loop; #4380 GQA 12/16 table 64/g8/hpt8, 128/g8/hpt4, 128/g12/hpt4, 128/g16/hpt2 -- hpt FALLS as GQA rises, eligibility threshold unchanged at KV >= 8192, a different kernel family, not a contradiction). T5 graph-build overlap (W67) evaluated then DROPPED: their lane is default-on at the new head (-1.5% cycle) but our Bench-761/771 caches consumed the same seam (submits 577 -> 1.98/token, M3 decode at the ~31 ms GPU floor) -- no measured exposed host-enqueue window remains; reopen on any future host-side decode decomposition. T6 riders (research-note records): riir-ai Bench 805 mask-fuse external confirmation (+3.5% prefill but divergence 8/8 greedy prompts at median position 7.5, rounding class -- ours bit-identical, the divergence posture is the point); Issue 734 GDN-blocked regime note (theirs -0.35 s on unblocked-eager stock vs ours 0.97-0.98x on already-batched -- do not cite as contradiction); Bench 641/B77 micro-vs-e2e third external instance (micro 1.94x, e2e -3.6%, peak +1.44 GB). Drop ledger held per plan (mincore read-skip, route kernel, K20 pre-scatter, FR-Spec, sparse split-K decode, GDN keep-mask fold [their own +4.2% LOSS], oMLX keepwarm) + en-route: the branch rewrite DELETED several verdict-time lead files -- surviving mechanisms are the body tables + retained kernels. Non-goals held: NO prefill-throughput claim (none of the retained levers transfer); no league action (M4-class silicon + finer-quant MoE pack). GOAT: batch103_rules_present + batch103_concepts_covered + batch103_pseudocode_parses (cfg(not(kernel_verify))) 3/3 (one gate-catch: the wavefront concepts needle 'stages - 1 pairs' re-pointed to 'exactly S-1 overlappable producer/consumer pairs' -- no text change) + kernel_heal lib 1285/0/1i + default lib 1029/0 + kernel_heal+latent 1392/0/2i with retrieval_precision_regression_gate PASS (floors HELD zero re-pins) + clippy 0 --lib in all three states. Count pins walked: tests.rs corpus total 459->463 + message + batch-list comment + expected-array D 98->102 + comment + sums x2 + cover-all comment 459->463 + router.rs expert total + mod.rs 463-rule doc + D header comment 98->102 + 4 list appends. EN-ROUTE count-pin rot FIXED: the 15 category header comments in corpus/mod.rs were stale by up to 2 batches (A said 103 vs true 105, C 39 vs 41, H 16 vs 17, N 27 vs 28) -- none pinned by any test, all 15 now match the pinned table. Generator lesson re-proven (the B77 escape-strip class): the first IN-PLACE edit of the B100 rider landed bare quotes inside the string literal (22 compile errors) and was repaired byte-precisely; the Python-heredoc GENERATOR path was clean first-shot. Record: Research 124 / Plan 072. **ADDENDUM (same day, second session -- the katgpt-rs Issue-665 collision class: both sessions executed the same NEXT-UP row concurrently; push-wins adjudication kept the sibling's landing 98b3b8f wholesale, the duplicate was yielded, and the SALVAGE is the one rule the landed batch silently omitted): `dispatcher-eligibility-predicate-lags-kernel-capability` (D; the MLX #4416 lead -- #3842/714a7ef landed the hd-256 NAX kernel WITH full array-mask support [has_mask function_constant(300), buffers 5/6] in the SAME commit as a dispatcher gate reading do_causal && !has_arr_mask: dead conservatism for the kernel's ENTIRE LIFE -- every masked prefill silently ran the decomposed fallback, correct output, slower, no test fails; #4416/f99e916 deleted the stale conjunct 13 days later touching ONLY the 2-line predicate; fix = align the predicate with the kernel's SPECIALIZATION SURFACE + a POSITIVE ENGAGEMENT test at every supported shape -- our split-decode >512 / tiled-flash / m16-cmma canaries already carry it; NOT in the landed batch's corpus, drop ledger, or Research 124 -- an oversight, not a decision; its fix-space was even CITED by the landed decline-mirror rule; landed as corpus 463 -> 464, D 102 -> 103, all anchors re-verified at 714a7ef/f99e916/b636898 [origin/main moved 90846ad -> b636898, allocator API only]; GOAT: batch103 gates extended 3/3, kernel_heal 1291/0/1i, default 1035/0, floors HELD kernel 100/100/100, clippy 0).

                                                      Batch 102 (2026-09-04, Research 123 / Plan 071 -- the FlashQuant sparse-dense fusion distill, arXiv 2608.15531, the walk-#6 armed paper-distill batch; CPU-only unit on the 4090 box while the 504 T3 NoRA confirm matrix holds the GPU; TAKEN OVER from a dead sibling session that left the plan all-unchecked and untracked -- renumbered 070 -> 071 per the reserved-071 collision flag, highwater stays 72; 6 rules, corpus 453 -> 459, A 103 -> 105, C 39 -> 41, H 16 -> 17, N 27 -> 28): `heterogeneous-unit-paths-shared-tile-fusion` (C; the additive-heterogeneous-paths member of the same-input family -- content-sharing fusion: Tensor-Core dense + CUDA-core sparse paths co-scheduled over ONE tiled execution space, activation staged ONCE, partials accumulated on chip, 'The partial results from the two paths are accumulated on chip before being written back to global memory'; preconditions offline tile correspondence + execution-unit warp specialization; EW rider folded [low-intensity path gets MORE warps, +2-4%]; NOT bit-identical, tolerance-gated; batch 8-32 regime, the walk-time batch-1-2 refutation carried honestly; measured 1.12-1.37x over Sputnik+Marlin on the 4090, tiling alone 6-16%, traffic -45.4%), `lpt-min-heap-disjoint-column-ownership` (H; the work-map family's balance complement -- assignment-time LPT over a descending nnz sort: 'a min-heap maintains the cumulative nonzero count of each bucket, and each column is assigned to the currently least-loaded bucket', one owner per column forever so 'This design preserves conflict-free accumulation'; balance purchased OFFLINE vs B97's runtime arrival-order atomics and B92's runtime conflict ordering), `tile-coo-sparse-tile-aligned-format` (N; a tile-local sparse format designed backward from the consumer's four constraints; gem (a) INDEX WIDTH DERIVED FROM STAGED-TILE EXTENT -- 'Given the maximum shared-memory tile size of 256x256, both row_index and col_index can be represented using uint8_t', assert at encode; gem (b) PADDING-SLOT REPURPOSE -- 'the unused padding slots can be repurposed to store original weight values', vectorization padding as free accuracy storage; uint4-packed decode, storage (Ntile+1)*4 + NB*4 + nnz*4 vs CSR (K+1)*4 + nnz*12), `bucketed-sparse-reader-column-partition` (A; the ACCESS-GRANULARITY-MISMATCH rule -- a staged tile co-owned by two reader classes: the sparse reader's per-element access conflicts when 'more than N_thread = bank_size/4 threads access the same column simultaneously', so partition columns into N_bucket = warp_size/N_thread buckets instead of touching the address map the dense TC reader owns; the consumer-side arm of 'layout is contested' beside B92's address-transform arm), `bank-group-circular-element-reorder` (A; offline DATA-side bank staggering -- reorder elements within each bucket by gid = floor(k/2) % 4 in the bucket-dependent circular order ([0,1,2,3]+bucket_id) % 4, 'reducing the number of conflicts from 4 to 1'; keyed to the consumer's walk schedule, bucket-DEPENDENT start is what staggers, stable reorder, pin the schedule), `stream-k-persistent-cta-mac-segments` (C; the work-DECOMPOSITION rule -- 'distributes the aggregate MAC iteration space across persistent CTAs', contiguous segments that SPAN tiles with a follow-up global reduction; MANDATORY CONTROL ARM = occupancy-grid-cap-refuted-uncapped-default B58 [uniform work measured 1-2% LOSS capped -- irregular per-tile cost is the adopt condition]; composes with B92's semaphore as the reduction mechanism; Lane A proposed rider-on-B92, Lane B's full kill-check found decomposition-vs-reduction distinct -- LAND, coordinator adjudicated). Sub-mechanism dispositions: warp-level Sliced-K folded into the fusion rule; the NAX hd-256 dsplit + #4380 GQA-12/16 table stay QUEUED as B100 rider material (not this batch). Discipline: two parallel distillation lanes (A: fusion + ownership; B: format + bank scheduling) with mandatory dedup verdicts, zero prior FlashQuant/Tile-COO/Stream-K corpus hits grepped across all 15 corpus files; all quotes verified byte-accurate by the coordinator against the arXiv HTML v1 rendering (the jina PDF route 500'd at mine time; the HTML rendering was the fallback -- recorded as the paper-fetch lesson). GOAT: batch102_rules_present + batch102_concepts_covered + batch102_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal lib 1282/0/1i + default lib 1029/0 + kernel_heal+latent 1389/0/2i with retrieval_precision_regression_gate PASS (kernel 30 labeled 100/100/100 -- floors HELD zero re-pins) + clippy 0 both states (one doc_markdown round on the new //! header: ternary/Q4_K needed backticks -- the B37 lesson's continuing strikes). Count pins walked: tests.rs corpus total 453->459 + expected-array A/C/H/N + sums x2 + grouped + cover-all comment 398->459 + router.rs expert total + mod.rs 459-rule doc + 4 category list appends. Record: Research 123 / Plan 071.

                                                      Batch 101 (2026-09-02, Plan 069 -- the unnumbered probe candidate PROMOTED, from Research 119's Parallel-session addendum [the M3 lane yielded then queued]: the aarch64 complement of Batch 99; 1 rule, corpus 98 -> 99, dispatch 4 -> 5): `dispatch-handler-pair-hoist` (dispatch; **MEASURED ParetoDominant 3/0/0 -- 2.89x/1.21x/1.23x at n=1K/100K/1M dispatches on the LCG-random stream, corpus validator the recorded instrument, alloc axis EXACT 0/0, alloc-det stable; the 100K/1M cells land inside the addendum's recorded 1.19-1.23x band, the 1K cell reads 2.89x [the probe's raw 0.837-0.840 = 16-19% fixed-wins band was the small-n regime; the validator's 1K cell is above it -- recorded honestly, both instruments cited]** -- a hot loop reads its handler fn pointer OUT of a runtime table on EVERY dispatch [the classic `HANDLERS[op](..)` idiom]; on aarch64 the indirect call keeps an IN-LOOP indexed load [asm-pinned: `csel x8, xzr, x22, ne; ldr x8, [x19, x8]; blr x8`] and the fix hoists BOTH table entries once before the loop -- register-resident select, the load leaves the critical path. REGIME: unpredictable streams ONLY -- period-16 0.988-1.004 and alternation 0.990-1.003 read Tie [a learnable stream lets the single indirect site's predictor hide the load]. CROSS-ARCH CAVEAT (x86 verdict PENDING, armed defer in Plan 069): x86_64's DestProp-merged form materializes addresses via `lea rip+h_*` -- NO in-loop load -- so the win may be aarch64-shaped; expected Tie there; the 4090 box carried 4 cargo + 3 rustc active sibling builds at batch time [contention], so the re-gate is ARMED not run. **RE-GATE DONE 2026-09-04 (quiet 4090 window, verdict-stable x2): ParetoDominant 3/0/0 -- 2.69-2.73x/1.09-1.10x/1.07-1.08x at 1K/100K/1M -- the expected Tie REFUTED; the lea-rip analysis belonged to B99's known-items fixture, while this runtime-TABLE fixture loads the handler in-loop on BOTH arches; the rule is now cross-arch ParetoDominant, annotation updated (updates-never-deletes), the concepts pin flipped PENDING -> REFUTED; honest residue: the structured-stream Tie cells were not re-measured on x86 [predictor-mechanical, expected arch-independent].** the rule is suggestion-only so no --fix path can fire it anywhere. DEDUP vs `mir-destprop-branch-site-merge` (B99): distinct mechanism AND fixture -- known fn items + tail-call + SPLIT [two direct jmp sites, x86-verified no-re-merge] vs runtime table + value-returning + HOIST; on aarch64 B99's SPLIT re-merges structurally so it does not transfer to this shape, and B99's runtime-opaque NEGATIVE CONTROL covers splitting on unpredictable streams, not hoisting; the two compose: known items -> split, runtime table -> hoist. Suggestion-only. GOAT: batch101_rules_present + batch101_pseudocode_parses + batch101_concepts_covered [incl. the never-delete x86-pending caveat pinned by test] + batch101_hoist_preserves_output_and_both_handlers_run [G1 output identity + non-vacuity: both handlers distinguishable] + the corpus bench verdict above + default lib 1022/0 + bench lib 1040/0 + retrieval floors 13/13 HELD + clippy 0 both states. Count pins walked: corpus header [STALE at 97/dispatch-3 from B99 -- the 18th-count-pin-site lesson live AGAIN, fixed to 99/5] + fn doc 98->99 + entries.len() + benchable eighty_five->eighty_six 85->86 + annotated forty_one->forty_two 41->42 [x4 sites incl. the 99-42 message + the contains-list entry] + AGENTS.md + .docs + .highwater 68->69 + queue snapshot. Record: Research 119 addendum (disposition note added) / Plan 069.
                                                      Batch 111 (2026-09-05, Research 133 / Plan 082 -- the vLLM libtorch_stable REMAINDER tier + the rider sweep, the B110-deferred queue consumed; M3 box, CPU-only; source re-cloned + RE-PINNED at the verdict sha 8277c42e at mine time -- origin had moved to 16328c7a minutes before the clone, the fetch-specific-sha dance): 11 rules, corpus 505 -> 516 (A +3, D +2, H +2, K +1, M +1, N +2; entries_gpu_23.rs) + 20 riders across 13 files (19 kernel_opt + weight-tile-consumption-order in rust_perf corpus.rs -- the B85 layout twin, name: field) + 2 KILL dispositions + 1 RIDER demotion. THE RULES: grid-dependency-wait-scoped-to-dependent-operands (D -- PDL wait scoping in three postures: conservative top / load-scoped / once-flag; the trigger placement is a VISIBILITY contract, kernel end, 'must not fire before this kernel's stores are issued' -- the H4 hazard IS the failure; no-op pre-sm90 and when PDL disabled), ue8m0-pow2-scale-as-biased-exponent-byte (N -- floor absmax at 1e-4 so log2 never sees 0, CEIL the log2 so the payload provably fits the fp8 range, clamp(exponent+127) into a u8; dequant = exponent-field bump = EXACT PoT, Issue-845 kin; 7 real + 1 pad byte/token vs 28), register-rotation-prefetch-loop (A -- the register-only rung of the prefetch ladder: prime curr, prefetch next guarded by has_next, rotate at the loop tail; no smem no cp.async, ragged-safe, portable; loses on dependent iterations/short bodies/register spill/bandwidth-bound), shared-operand-groups-per-warp (A -- the operand-affinity axis: K heads of ONE token share the cos/sin row; two cp.async groups ORDERED BY CONSUMER NEED, wait_group<1> unblocks group-0 compute, the full wait_group<0> defers to the first head's RoPE), launch-heuristic-pinned-to-calibration-arch (K -- the no-sweep-infrastructure guardrail: conservative baseline default, tuned ladder gated to sm_version == 90, 'until profiled'; B104's ref_shape IS this naive; B104 swept table = the upgrade path), warp-aggregated-atomic-scatter (H -- ballot + ONE leader atomicAdd + shfl base + popc(mask & lanemask_lt) rank: contention O(hits) -> O(warps); the per-element anti-instance ships in the same file), refine-by-rescan-prefix-filter-vs-buffered-ties (D -- the H1 fix contract AS A RULE: measure the bin size against the tie-buffer capacity FIRST, buffer iff it fits, else rescan under the accumulated bit-prefix; clamp to what was WRITTEN (min(le_full, kMaxTies)) never what was counted; the silent-drop class (__builtin_expect(pos < CAP, 1)) is the hazard; three in-source instances of the one decision), coarse-precision-first-exact-refine (N -- first histogram pass on a LOSSY coarse key (f32->f16->sign-flip order-preserving->top 8/12 bits) to isolate the threshold bin; exact refinement recomputes from the ORIGINAL f32, never the lossy key; per-dtype traits mandatory), whole-row-register-resident-under-length-budget (A -- 16 floats = 4xfloat4 per thread -> rows <= 16384 at 1024 threads: load ONCE, all phases consume registers, ties stored as (idx, score) pairs; beyond the budget buffer indices only and pay the re-read), lane-owned-topk-output (M -- result k parked on lane k during the reduction, unpacked only on lanes < K; gated by LaneOwnedResourcesFit (merge_per_lane <= 64) AND the expert range 512..=1024 / top 9..=16), routing-by-precomputed-lookup (H -- deterministic-identity routing -> offline tid2eid table -> 6 gather loads + live weight recompute; out-of-table tokens take the E-expert fallback behind an explicit sentinel check, never a silent miss). MINE-TIME DISPOSITIONS: #21 (noinline-rare-path) KILLED as COVERAGE -- B110's persistent-kernel-length-bucketed-mechanism-ladder protection (b) carries noinline + #[inline(never)] (entries_gpu_22.rs:100/:102); #23 (clamp-item-metadata-before-collective-branch) KILLED as COVERAGE -- B110's spin-barrier companion contract (entries_gpu_22.rs:28) already cites persistent_topk.cuh:909-928; #24 (capture-stable-grid-shape) demoted to a RIDER on device-m-runtime-mblock-graph-stable-gemm (B92 owns the capture-stable core; the per-row device-resident-length variant is rider-sized). THE RIDER SWEEP (20, all located with quotes): hi/lo packed-u64 redux decomposition -> B107 radix-select; inverted-index pack external confirmation -> B107 packed-total-order; phase-counted barrier (wait_ge(counter, (phase+1)*ctas), no reset) -> B92 stripe-sms; B57 union-across-phase-boundary + filler-thread barrier-count matching; stable-softplus third form (sqrtf(fmaxf(x,0)+__logf(1+__expf(-fabsf(x))))) -> B98 fused-vector-site; loud-refusal third instance (cfg-gated no-op device body + host STD_TORCH_CHECK) -> B103 shape-class; PDL capability gating (numAttrs = (sm_version >= 90) ? 1 : 0, fail-soft complement) -> B105 cooperative-tensor-mma; measurement-revision honesty (M=14 0.91x flip re-swept, B300+B200 named) -> B104 swept-shape; family-gated table (kGfx950*, 'for gfx950, FP32, and k=1024') -> B104 no-harm; KDA cp.async ring second family -> B92 cpasync; num_experts switch second family -> B23 hardcoded-fast-path; verbatim cache store on format match (kAuto) -> B37 verbatim-upload; convert-before-quant boundary emulation -> B97 bit-exact-dispatch-fusion; zeros-for-identity re-init (WEAK MAPPING recorded) -> B35 mask-sentinel; pointer-array multi-layer insert merge -> B26 merge-dispatches; RoPE-fused cache-write twin (3 kernel families) -> B22 fused-activation-quantize; layout-at-write twin (caveat recorded) -> rust_perf weight-tile-consumption-order; GDN MTP accepted-chain replay -> B81 gdn-transition-replay; per-row capture-stable variant (#24) -> B92 device-m; runtime pointer-alignment gate + scalar fallback -> B98 dequant-byte-pairs. GOAT: batch111 gates 4/4 (rules_present + concepts_covered + pseudocode_parses + batch111_ue8m0_byte_contract -- the gate pins the floor/ceiling/tightness/PoT-exactness/monotonicity/non-vacuity arithmetic) + kernel_heal lib 1361/0 + combined 1469/0/2i (retrieval_precision_regression_gate PASS, floors HELD zero re-pins) + default lib 1082/0 + clippy 0 in default/kernel_heal/all-targets states (5 pedantic casts in the ue8m0 gate allowed with the algorithm-is-the-cast justification, the B59 precedent). Count pins walked: tests.rs 505->516 total + batch-list + current comment + expected-array A 109->112 / D 114->116 / H 21->23 / K 8->9 / M 14->15 / N 30->32 + sums x2 + grouped + cover-all comment + router.rs 505->516 + mod.rs doc line + mod/extend wiring + 6 category comments + 6 member-list tails + the 3 FULL-STRING pins of ridden rules regenerated (batch22 fused-activation + batch23 hardcoded-fast-path + batch26 merge-dispatches -- the older batches pin the WHOLE concept string, so the rider text must be spliced into the pin, the B103/B108 precedent). GENERATOR DISCIPLINE HELD: entries + riders via Python generators (target/batch111/), zero hand edits on long strings; the generator's needle self-assert caught 2 needle drifts pre-landing. GOAT GATE SELF-YIELDS RECORDED: the ue8m0 gate's vacuity guard caught the sweep's own fixture bug TWICE (mantissa-only bit sweep collapses to 1 byte; 59<64 under an arbitrary floor) before the author fixed the sweep to vary the EXPONENT FIELD -- the gate doing its job, the Bench-802 T5 shape. Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Cleanup: .raw/vllm removed at landing. Record: Plan 082 / Research 133.
                                                      Batch 112 (2026-09-05, Research 134 / Plan 083 -- the nnethercote July-2026 byval-move memcpy-threshold distill, rust_perf layout; M3 box, CPU-only; the R134 shortlist consumed the same day): 1 rule, corpus 100 -> 101 (layout 9 -> 10) + 2 external-confirmation riders in the name field (usize-index-to-u32 += rustc #158720 cache-line-boundary -- AST nodes 72->64 B, >10% wall + cache-miss -29%, shrink past 64 B is a step function; sort-key-cache-precompute-out-of-lock += rustc #157179 -- rustdoc sorted by long generated HTML strings, a short text repr as sort key won >6% icount). THE RULE: hot-byval-move-over-memcpy-threshold -- LLVM lowers by-value moves of a >128-byte type to a memcpy CALL (rustc #160005: a hot 136 B type paid ~half its compile time in memcpy until shrunk to 104 B, 40% wall on the two worst crates; DHAT copy-profiling was the finding instrument). THE ARCH FINDING (the batch's measured yield): the threshold is an LLVM BACKEND CONSTANT and it is arch-shaped -- the corpus gate's asm reading (cargo rustc --emit asm on the bench shapes) shows a 136-byte by-value move INLINING to ldp/stp pairs (4 pairs + an 8 B tail) on aarch64 + current rustc -- NO memcpy call -- while a 320-byte move on the same box emits bl _memcpy, so the aarch64 inline ceiling sits in (136, 320], ABOVE x86-64's 128 B. M3 bench verdict: Mixed 1/0/1 (1.29x/0.99x at 1K/1M, verdict-stable x2, allocs 0/0, alloc-det stable) -- the 1M tie is the honest mechanism cell (both arms inline at 136 B); the n=1K cell is warm-up-dominated (per-iter cost halves from 1K to 1M in BOTH arms), never cite it. Fix doctrine arch-independent + never-negative (shrink under the target's ceiling / pass by ref / avoid the move); suggestion-only. X86 RE-GATED SAME-DAY (4090, quiet, rustc 1.93): the asm shows the step DIRECTLY -- the 48-byte copy inlines while the 136-byte loop's hot loop carries `movl $136; callq memcpy` per iteration; wall clock moves the same direction (136 B loop = 1.32-1.35x the 48 B loop per iter on x86 vs 1.00-1.13x on the M3). GOAT: batch112 gates 4/4 FIRST-SHOT (rules_present + pseudocode_parses + concepts_covered + batch112_byval_folds_agree_bit_identical -- the fairness gate pins bit-identical folds across the two arm shapes AND the 136/104 size_of content) + default lib 1086/0 (+4 gates) + bench lib 1104/0 + latent 1193/0/1i with floors HELD zero re-pins + clippy 0 default-all-targets/bench/latent states + score-bench run #39 FLAT (96.2/1.9/100/100, miss map unchanged). Count pins walked: corpus.rs doc header 100->101 + layout 9->10 + entries.len() 100->101 + benchable 86->87 + annotated 43->44 (membership added) + unannotated pinned at 57 (the new rule joined the ANNOTATED side -- first-pass arithmetic said 56, the runtime count refused, corrected) + serde 43->44 + propagates 44/57 + the stale per-category test prose (layout 7 -> 10). No new plan-number misses: .plans/.highwater found at 81 while plan 082 existed -- repaired to 83 at Plan 083 authoring (the missed-bump class, B94/B106 precedent). Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Plan 083 / Research 134.
                                                      Batch 114 (2026-09-05, Research 138 / Plan 087 -- the oMLX ANE-prefill + GDN blocked-sequential distill, kernel_opt G + D; M3 box, CPU-only; the shelf row-5 named-unmined lead (ane_heterogeneous_sources.md) + the PR3298/2853 follow-up consumed): 2 rules + 6 riders, corpus 516 -> 518 (G 27 -> 28, D 116 -> 117; entries_gpu_24.rs). Source jundot/omlx @ e467261 (Apache-2.0) re-cloned at the pin; a quote-verification lane re-read all ten surfaces with exact line numbers (range-drift table in the lane report). THE RULES: recurrence-prefill-exact-restructure-before-reformulation (G, the A- GOAT lead -- restructure the EXACT sequential recurrence's memory layout BEFORE an algebraic reformulation: the stock per-slice kernel re-reads k/q once per value-slice threadgroup = 32x redundant traffic (~13 GB/16k-token layer); the restructure = TB-block smem staging + Dv/16 value blocks (8x fewer threadgroups per k/q row) + register-resident state; ~2x @16K (14.9 vs 29.7 ms/layer) on M3 ULTRA -- source-reported, never cite as ours; the WY/FLA chunked reformulation measured a LOSER three independent times (oMLX + Rapid-MLX + MTPLX ports) + our own Bench 734/769 GDN-chunked negatives 0.97x/0.98x -- restructure-first is an empirically separated ORDERING) + ane-bank-split-ladder (D -- near-half-first split ladder when a monolithic accelerator bank fails to map at PROGRAM-CREATE (0x20004; our 0x50004 cascade at ~105 resident programs the same class); THE MEASUREMENT-HONESTY HALF: the driver's release is ASYNCHRONOUS -> drop partial banks + gc + settle BEFORE the next headroom re-measure; THE GATE HALF: 70% phys-footprint fraction stops the ladder BEFORE jetsam (4 bounded attempts still hit 48.3 GB on 48 GB -- source-measured); bank-count = a DETERMINISM axis (bit-stable at 2 banks, many small banks diverged at greedy ties); fail-open snapshot; env-forced initial cap; feeds riir-ai Issue 876 T-probes + Issue 769 T2/R1/R4a-R7 -- recorded, the issue edits belong to the M3-queue lane). THE RIDERS (6): B84 token-local += the recurrent-safe fraction cap (cap-at-z + alignment floor + the loud requested-fraction cap log; qwen35_ane_prefill.py:1023-1041/:1239-1295); B84 pad-tail += the profitability predicate (0 < threshold <= rows < tile) + the SINGLE-PROMPT-ONLY guard ('Flattening a real batch would lose the sequence boundaries required by the GDN recurrence') + tail routing below qmm min-tokens (:260-315); B22 warp-shuffle += the SUB-SIMDGROUP cooperation radius (8-lane segments inside a 32-lane simdgroup, simd_shuffle_down(4,2,1) 3 steps + one leader broadcast simd_shuffle((tid%32)/8*8), zero hot-loop barriers -- the FULL-STRING pin regenerated); B95 whole-sequence += the posture at prefill grain (gdn.py:411-533); B107 prefill-chunk += the program-SHAPE vs delivered-chunk-WIDTH coupling (compiled-larger-than-delivered COMPILES BUT NEVER EXECUTES -- delivered cap = max(step, floor) min block-cache size, warn with the fix; :318-356); B79 vectorized-smem += the SCALAR-STAGING arm of the pad boundary (scalar per-element staging keeps the conservative pad; the refutation scope is VECTORIZED line-sized stores; gdn.py:444-476). Lane-intel (not corpus): the tri-engine CPU-fraction split + warm-first-dispatch (:408-449/:2407-2459) -- 769's f-split + R4a warm rows track it. HONEST NOTES: the 'carried-state-reduction-stays-scalar' reassociation comment is Batch 113's llama.cpp content (ssm.metal:454), NOT this source -- the oMLX scalar+shuffle MECHANISM stands on its own at gdn.py:498-514; Kernel S has NO channel-tile loop (that pattern is Kernel B's, :283-292) -- the queue line's phrasing corrected in Research 138. GOAT: batch114 gates 3/3 FIRST-SHOT + kernel_heal lib 1368/0/1i + default lib 1086/0 + combined 1483/0/2i with retrieval floors HELD zero re-pins (kernel 30 labeled 100/100/100, blended 94.7/96.9/97.7) + clippy 0 in default / kernel_heal / kernel_heal+latent --lib states + kernel_heal --all-targets clean. Count pins walked: tests.rs corpus count + message + batch-list comment + exhaustive/sum/grouped + expected-array D/G; router.rs expert total; mod.rs doc line + D/G category headers + list appends + mod/extend; the B22 FULL-STRING pin spliced. En-route repairs: .plans/.highwater 87 -> 88 (Plan 088 existed) + .research/.highwater 138 -> 139 (Research 139 existed) -- both missed-bump stale from the sibling's filings, the B94/B112 class. Generator lessons: the pin-splice anchor must carry the string TERMINATOR (the first splice dropped the closing quote -> 172 cascading E0762s -- caught by the first compile, fixed by restoring the quote); rider appends must keep the host sentence's period (a rstrip('.') silently ate it on 4 sites -- caught by reading back the diff). Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Plan 087 / Research 138.
                                                      Batch 113 (2026-09-05, Research 137 / Plan 086 -- the llama.cpp v0.4.0 delta re-mine, kernel_opt G + D + N + I; M3 box, CPU-only; B104's fa-vec pins sit INSIDE this release window so fa-vec is NOT re-mined; executed AHEAD of its soft 'after arxiv walk #8' slot since the walk is time-blocked and this batch was not): 4 rules + 18 riders, corpus 518 -> 522 (G +1, D +1, N +1, I +1; entries_gpu_25.rs -- the plan's T3 named entries_gpu_24, which Batch 114 took; amended at execution). Source ggml-org/llama.cpp @ 5266f24da (v0.4.0, MIT) re-cloned at the pin; a quote-verification lane re-read all five surfaces with exact line numbers and corrected THREE attributions (the XOR-swizzle #25635 and MoE-fusion #25952/#27621 commits are CUDA-SIDE, zero 'moe'/'swizzle' in ggml-metal; --lazy-mode is APP-LAYER -- common/arg.cpp:2737 + llama.h:217 + llama-model-loader.cpp:1082-1088 -- not ggml/src; the Metal-4 gate is has_tensor/MTLGPUFamilyMetal4_GGML, not 'tensor_map', and also gates on M6/A20 with measured reasons in-source). THE RULES: chunk-transition-consumed-by-mma (G -- the GOAT lead half 1: the per-chunk Gram C*B^T 'computed in each chunk one time and reused across the head_dim channel tiles' (ssm.metal:300-302), the causal exp(acs_i-acs_j) fold applied to the STORED Gram once (:406-413), both output halves on the matrix unit (:417-435); the five-clause use_mma gate (ops.cpp:1773-1782) + the mma_tokens/snap_reserve kernel-FAMILY tail dispatch (:1775-1776/:1802-1819); tolerance-gated -- chunk OUTPUTS reassociate) + carried-state-reduction-stays-scalar (D -- the sharper half: the source's own contract 'Keep the carried-state reduction in token order. Reassociating this particular product with MMA compounds rounding differences at every chunk boundary; CB, y_diag, and C*S remain on the matrix unit.' (ssm.metal:454-456) -- a per-product PLACEMENT decision: non-compounding consumers (per-chunk outputs) may ride the matrix unit, compounding consumers (the carried state) stay scalar in token order; generalizes to every chunked-recurrence MMA implementation) + shader-fastdiv-mulhi (N -- the GPU fast-division cluster: pow2 fastmod mask + umulExtended-mulhi ROUND-UP fastdiv ((msbs + n) >> L, the +n IS the round-up; magic pair precomputed host-side 'see init_fastdiv_values') + fastdiv_L byte-spaced 8-slot/6-bit-mask packing + fastdiv_small quotient-zero branch-skip; utils.glsl:4-26; no in-source commit comment -- provenance is the delta, c9ca51c #27526 per the release notes) + sparse-kv-index-list-sentinel-padded (I, graded B-, landed as a rule after the T1 fa.metal read confirmed the contracts: the producer compresses each mask row into an ascending KV-index list, -1 SENTINEL-padded to n_kv_max_padded (a NCPSG tile multiple), the consumer gathers by the list; the mask remains the SINGLE SOURCE OF TRUTH; TRUST-BUT-CLAMP ('if the hint is violated, keep only the first n_kv_max entries', fa.metal:1152-1156); dense-overflow re-read arm (:1158-1172); decode-side Q == 1 (fa.metal:1076-1179 + :1318-1324)). THE RIDER SWEEP (18: 16 kernel_opt + 2 rust_perf name-fields): B92 xor-swizzle += #25635 (CUDA-side, fattn-swizzle.cuh:6); B98 moe-routed-down-sum += #25952 + the #27621 regime extension ('restricted to 1 token' -- the decode-only precondition was ARTIFICIAL, later lifted to specdec); B46 size-gate += the five-clause use_mma gate (each clause comments its cost); B57 smem-slot-role-reversal += the s_buff device-scope barriers BOTH sides (ssm.metal:451-452/:470-471); B110 row-parallelism-ladder += the kernel-FAMILY tail fallback (ops.cpp:1802-1819); B107 prefill-chunk += the Metal top-k radix FIXED-COST shape gate (~50-60us single-workgroup-per-row, use_radix = ncols > 2048 && (k > 64 || (nrows > 4 && ncols >= 8192)), #28073); B105 tensor-op += #27468 oneDNN f16 KV in-place bind; B105 cooperative-tensor-mma += the Metal-4 has_tensor probe with the MEASURED-REASON default-off ('M2 Ultra: ~5% slower / M4, M4 Max: no significant difference', env re-enable); coarse-precision-first-exact-refine += the Metal radix-select (4 radix-8 passes, ops.cpp:5360-5362); B96 fwht += the Kronecker FWHT REVERT as regime evidence (1f3d318 #28016 reverted by c845263 #28184 in-tree; the restore #28254 is post-v0.4.0 -- never a rule); B22 fused-activation += DFlash-encoder-into-KV-injection + the non-fused GDN/LID paths DISABLED (#27310/#27877); fill-gate B94 += the sparse-hint clamp (fa.metal:1152-1156); B104 no-harm += the MMVQ-to-MMQ crossover moving to SM87+ (#28285); hardcoded-fast-path B23 += mm_ids_helper generated for ANY n_expert_used (#27978); B38 env-flag += KleidiAI init off the dispatch path (#27891); B71 rotated-ring-undo += snap_reserve as the rollback-consumer ownership constraint (ops.cpp:1775-1776 + #28123); rust_perf eager-pointer-lazy += the --lazy-mode feature (app-layer citations); rust_perf channel-send-batching += the quantizer row-slab stream (#27830). LEAGUE FLAG recorded in riir-ai perf_rematch_league.md: upstream v0.4.0 completes the fa-vec M3 Max table (#28373) -- the fork pin lacks fa-vec; any re-pin to >= v0.4.0 moves the opponent's M3 prefill cells. GOAT: batch113 gates 3/3 FIRST-SHOT + kernel_heal lib 1371/0/1i (incl. the 2 full-string pins regenerated for the B22/B23 riders) + default lib 1086/0 + combined 1486/0/2i floors HELD zero re-pins (kernel 30 labeled 100/100/100, blended 94.7/96.9/97.7) + clippy 0 in default / kernel_heal / kernel_heal+latent / rust_perf states + kernel_heal --all-targets clean. Count pins walked (corpus 522, D 118, G 29, I 5, N 33, router 522). Generator lessons: the rust_perf NAME-FIELD splice appended AFTER the string's .to_string() (the anchor included it) -- 2 E0732-class breaks caught by the first compile, fixed by moving the rider inside the string; the idempotency guard's shared 44-char prefix false-positived across riders of the same file -- fixed to a unique body slice [61:105]; line-count greps under-count multi-rider lines (count occurrences with -o). Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Plan 086 / Research 137.
                                                      Batch 109 (2026-09-05, Plan 080 -- the post-mining P10 measurement-honesty distill, from the katgpt-rs Issue 723 Class A2 evidence; the intake's last open class consumed; 1 rule, corpus 99 -> 100 (complexity 18 -> 19; annotated 42 -> 43; NotBenchable 13 -> 14): `dead-result-timing-loop-folded-under-lto` (complexity, suggestion-only, bench_spec None -- the FIRST rule on the measurement-honesty axis: a timed loop whose work result is discarded (`let _ = f()`) measures ZERO under rustc 1.98.1 + fat LTO -- the fold eliminates inlined-callee work EVEN THROUGH `black_box` INSIDE the callee; the fix pattern is CHAINED state (xorshift-PRNG shape, each iteration consumes the previous accumulator, unprunable by construction) + consumer-side `black_box` + the loud nonzero-baseline assert (NaN compares false against everything: a `>=` PASS bar fails loudly on a zero baseline while a `<=` budget bar passes silently forever -- assert BEFORE the ratio; ratio gates want equal non-pruner work + interleaved median-of-ratios). THE GATE SHAPED THE RULE -- two draft fix-patterns were REFUTED by the batch's own G1 fixture and both anti-shapes are pinned as assertions: (1) an XOR sink over a GF(2)-linear work fn cancels EXACTLY over 0..2^k (XOR of work(0..4096) = work(0) = 0 -- the first draft's fix was itself foldable); (2) a wrapping_add sink over a balanced power-of-two batch collapses to the seed-independent constant 2^64 - 2048 = 18446744073709549568 (every balanced output bit is 1 in exactly half the terms regardless of seed -- measured, both arms identical). Differentiation: debug-stat-full-scan-out-of-hot-path is the unconsumed PRODUCTION statistic (this is the HARNESS side); encoded-layout-borrowed-view's bench note is the same physics from the bench-consumption side. GOAT: default lib 1082/0 + rust_perf_bench 1100/0 + latent 1189/0/1i (floors HELD, retrieval_precision_regression_gate PASS zero re-pins) + clippy 0 real findings both states (exFAT hardlink noise only) + the 4 batch gates (present/parses/concepts/sink-dataflow). Count pins walked: corpus.rs header (100/complexity 19/NotBenchable 14) + build-fn doc 100 + entries pin 100 + annotated 42->43 x4 sites + forty_two->forty_three rename + unannotated message 100-43 + thirteen->fourteen rename + 14-pin + membership assert. NOTBENCHABLE -> NotBenchable case lesson (the annotation-vocabulary contract fired on the draft, the B51 class live). Record: Plan 080 / Research 132.
                                                      Batch 99 (2026-09-02, Research 119 / Plan 067 -- the wasmi v2.0 blog distill, the FIRST rust_perf batch since Batch 94; source wasmi-labs.github.io 'Wasmi 2.0 - Engineering of the Fastest Wasm Interpreters', Robin Freyler, 2026-09-01, first-party + measured, the DestProp finding cross-verified on Stitch; blog-only source so fixtures are ORIGINAL per license hygiene; 1 rule + the C+ secondary honestly measured-and-queued, corpus 97 -> 98, dispatch 3 -> 4): `mir-destprop-branch-site-merge` (dispatch; **MEASURED ParetoDominant 2/0/1 -- 1.04x/1.24x/1.25x at n=1K/100K/1M dispatches, 50/50 mixed stream, corpus validator the recorded instrument, alloc axis EXACT 0/0, alloc-det stable x3** -- a hot conditional dispatch whose two arms load a KNOWN fn item and converge into ONE call site lowers [asm-inspected on this toolchain, not inferred] to `lea rax,[rip+h_a]; lea r9,[rip+h_b]; test cl,cl; cmovne r9,rax; jmp r9` -- a conditional move of the two code addresses + a SINGLE indirect tail-jump site mixing branch-target history forever; the split form emits TWO DIRECT `jmp h_a`/`jmp h_b` tail-jumps -- NO re-merge by LLVM. Both mechanisms are the rule's content: devirtualization enablement (the phi of two known fn items can never fold to a direct call) + per-site predictor history. REGIME map measured across three skew arms (98/02 1.18/1.30/1.28 most stable; 50/50 1.11/1.21/1.19; 75/25 1.31/1.05/1.03 borderline). NEGATIVE CONTROL measured -- the boundary: when the two fn pointers are RUNTIME-OPAQUE (fn args / table loads -- genuinely dynamic dispatch) BOTH forms keep indirect calls and the split buys NOTHING (0.95-1.00x across all nine cells x2 runs) -- the rule applies ONLY to compile-time-known code pointers; for genuinely dynamic dispatch dyn-to-enum is the right rule instead. TOOLCHAIN REGIME: rustc >=1.92 MIR DestinationPropagation (the source measured the 1.91->1.92 boundary, ~2800 -> >4200 CoreMark ~+50% from the single fix on wasmi's tail-call dispatch; re-gate on toolchain bumps, the debug-assert-length-hoist precedent). Suggest-only -- a parse-only proposer cannot see codegen shapes. Source: wasmi v2.0 blog 'Accidental Rust Deoptimization', Research 119 / Plan 067.) The SECONDARY (`hot-dispatch-gpr-arg-budget`, C+) was probed and honestly NOT landed: 7-int-arg vs float-domain-packed hot call loop measured **Tie on this box (1.009x/0.999x, two runs)** -- windows-msvc x64's 4-GPR budget differs from the blog's sysv64 evidence and OoO hides the 3-slot stack spill at call granularity; recorded as queued-secondary with the kill-free note (re-open on a sysv64 measurement window; ABI-specific, interpreter/VM-loop-shaped pattern). GOAT: batch99_rules_present + batch99_pseudocode_parses + batch99_concepts_covered (the kernel-batch concepts convention applied to rust_perf) + batch99_split_sites_preserve_output_and_dispatch_sequence (G1: identical handler SEQUENCE via per-handler counters + identical final state, non-vacuous) + the corpus bench verdict above + default lib 1012/0 [1008 + 4] + bench lib 1030/0 + retrieval floors 13/13 HELD + score-bench #30 FLAT (96.2/1.9/100/100) + clippy 0 real findings both states (exFAT hardlink noise excepted). Count pins walked -- SIX sites, THREE initially missed and caught by the full-suite run (the 18th-count-pin-site lesson re-proven): the seed-corpus total pin 97->98 + its fn rename + the dispatch-3->4 comment; eighty_four->eighty_five benchable 84->85 + fn rename; annotated 40->41 + fn rename (thirty_nine -> forty_one) + list entry + the Batch-99 doc line; unannotated message 97-40 -> 98-41; serde round-trip annotated 40->41; drafter-propagates annotated 40->41 + the stale '34' comment fixed to 41. Generator lessons struck AGAIN: the line-initial `+ ` doc-markdown class (B37, now the 11th+ strike -- caught by clippy in the count-pin doc walk) + `///` inside a fn body (the B85 E0658 class, caught on read-back) + an edit_file old_text that duplicated a 'Batch 91 added 2' line (caught by clippy's doc-list lint). Full CI guard not run this cycle (the B95 honest-skip shape: default + bench + latent states + floors + score-bench all individually validated). Record: Research 119 (status -> RECORD) / Plan 067.
                                                                                                            Batch 100 (2026-09-02, Research 120 / Plan 068 -- the Layr-Labs/qwen-3.8-mtp-challenge distill, the M5-Max Swift/Metal inference arena, same team as antirez/ds4 [Batches 97/98]; source pin 0863b06ac16e26e48fc06e97444095b00feb66d4 verified BYTE-IDENTICAL to the verdict pin at mine time -- tree static since 2026-08-23, the leaderboard 'perf bumps' are auth-gated submission scores not tree changes; two parallel verification lanes: Lane A read pagedattention.metal [500 lines] IN FULL + kernel-research.md sections 1-6, Lane B read the 5 files the verdict left unread incl. the never-before-read LagunaDFlashBlockSession.swift; 8 rules, corpus 445 -> 453, A 100 -> 103, D 95 -> 98, E 30 -> 32, + 1 B22 rider): `functional-cache-versioning-loses-donation-under-kernel-read` (A; the GOAT LEAD -- a large shared device slab must NEVER be versioned through functional slice updates when any custom-kernel dispatch reads it in-flight: the MLX runtime pins every op's INPUT Data handles until command-buffer completion, so a kernel read pins the slab and the next slice_update fails is_donatable() -> full-slab copy fallback, measured 23 layers x {K,V} x 8 GiB = ~370 GiB copies/token [ONE numeric correction from the verdict's '24x' -- 24 layers total, first read pins, subsequent 23 copy], TWO-REGIME evidence [write-only chains donate linear even 24 deep; interleaved kernel reads degrade to O(slab)x layers], SUPERLINEAR rider [the transient copies trip the memory-limit throttle: B=1 25,470 ms vs the 1.9 s copy arithmetic, ~13x], detection corollary [needs >= 2 layers sharing a slab group -- parity tests stay green by construction], fix = stable buffer written IN PLACE by a kernel [vLLM reshape_and_cache; 'any fix keeping per-layer slice_update on multi-GiB shared slabs is dead on arrival'], post-fix 25,470 -> 13.6 ms/step; category corrected from the verdict's tentative D -- its siblings append-cache-rewind + kv-window-ring-append live in A), `fused-kv-write-single-writer-reads-newest-from-input` (A; race-free fused KV write WITHOUT cross-threadgroup primitives iff exactly ONE threadgroup per byte writes [hgroup==0 of the row's LAST partition, guard hgroup == 0 && part == (attend_len - 1) / PTOK] and NO threadgroup reads the racing slot from the slab -- all hgroups of the last partition load the newest position from the knew/vnew INPUTS [bit-identical bytes]; ring-alias impossibility [the overwritten slot held a position >= ring*S older, outside every attendable window]; the DECODE arm of the in-place contract), `in-place-write-through-input-emits-fence-consumer` (D; under concurrent encoders with hazard tracking at DECLARED-buffer granularity, an in-place write through an INPUT declaration is invisible to the tracker: the write kernel emits a chained fence [fence = prev + 1, one thread] and every reader of the SAME bytes -- directly or transitively -- consumes it [zero-cost index add] for order + barrier; decode arms ride graph edges [the host kernel literally takes wfence as an input operand]; + the MLX <8-element CONSTANT-address-space ABI rider [pad small device arrays to >= 8]); `residency-set-wired-once-post-warm-persistent-footprint` (D; Metal residency-set capacity is ZERO until a wired limit is applied -- unwired, the driver re-establishes residency for the whole weight tower per command buffer; wire ONCE after the input-independent warm at the live post-warm footprint + page allowance ONLY [later scratch fails the fit and stays unwired by design]; never end or shrink the ticket [shrinking evicts resident weights]; anchor re-verified L202-212), `register-budget-head-split-gqa-reread-trade` (E; per-thread acc floats HPT*D/32 over the ISA register budget -> split a kv head's GQA query heads across GQA/HPT threadgroups [d512/GQA8 -> HPT=2, d256 -> 4, d64/d128 unchanged; largest-divisor-under-cap selection]; per-HEAD split so no head's summation order changes; the trade = KV re-read GQA/HPT times [vLLM RFC #15351 ~30% at g=8 -- eligibility, not optimality]; the host models the smem formula byte-for-byte + pins it with a no-GPU test because an over-cap dispatch is an UNCATCHABLE process fatal [the Gemma-4 d512 history: the old head-dim-only gate passed it and KILLED the process]; test file confirmed: CBv2PagedEligibilityTests), `batch-composition-invariant-grid-early-return` (E; NOVEL from Lane A -- each threadgroup owns one (row, head, partition) triple and every row's extent derives from ITS OWN metadata: per-row arithmetic INCLUDING float summation order is bit-identical regardless of batchmates; over-launched groups for longer batchmates RETURN EARLY [a branch, not compute]; their slots are never read because the merge RECOMPUTES the partition count from the row's own attended length -- a merge trusting the launched count would read unwritten slots), `never-gqa-multiply-full-d-merge-smem` (A; NOVEL from Lane A -- the source doc's own extracted rule: a full-D merge record per (simdgroup, head) is 'the anti-pattern no production kernel uses' [32,832 B at d512/GQA8/NSG1]; the 3-scheme menu: 32x32 transpose tile [constant ~4 KB regardless of D and GQA, (D/32)*GQA rounds x 2 barriers] / no intra-tg merge [one simdgroup per query head, smem = 0] / warp-count-bounded regions [D x warps <= 8, never x GQA]; evidence: both GQA-fusing production kernels have GQA-INDEPENDENT smem [0 / 4.35 KB]; the smem-budget complement of the head-split rule -- they compose), `container-rebind-over-retrace-for-churn-survival` (D; NOVEL from Lane B -- a compiled graph serving a CHANGING row population routes operands through REGISTERED CONTAINERS re-read every call, so membership churn becomes a lane REBIND [different array refs + two int scalars: no copy, no retrace] and eager readers always see current KV; four stabilizers [identity + host-mirror no-op fast path; partial-bind failure clears the lane WHOLESALE; host mirrors advance per replay; eager fallback re-establishes -- never a permanent invalidation]; the FOURTH mechanism of the captured-graph mutator family beside B58 drop-or-survive / B81 retain+rebuild / B92 device-M / B97 static-sibling; CUDA analog cudaGraphExecUpdate). Rider: B22 `warp-shuffle-intrinsics-for-sub-warp-reduction` += the Metal measured-cost note (simd_sum ~14.5 cycles throughput / ~28 latency vs ~2 for a shuffle; quad reductions ~2x cheaper; a shuffle tree can beat the builtin -- kernel-research.md section 1.1 L57-60; the batch22 full-concept pin MOVED WITH the rider, the gate catching the deliberate change exactly as designed). Deferred with reasons (ledger in Research 120): indivisible-span chunk snapping + one-shot starvation guard [multimodal-instance, no in-tree consumer], hidden fast-path footprint charged to the admission ledger [batch full -- reopen on the next adjacent batch], chained-decode lazy-token overlap [medium confidence], prime-host-syncs-off-the-traced-region [B58 rider candidate], scratch-lane recycle [pad-family rider], warmup-past-the-wrap-seam [measurement methodology, the warmed-arm half already encoded in riir-ai Bench-749/831], R6 command-buffer byte budget [SKIPPED per plan: config-knob shaped, anchors NOT in the metal/kernel-research files -- Lane A grep-confirmed]. Drop-rows held (Qwen35GatedDelta = B95 instance; DFlash spec-decode = B71/B81/B84; MTP micro-patterns = B30/B25/B26/B27/B36; Lane B additionally confirmed B42-family + B38 positives + a B25 ANTI-instance [full-logits download is challenge-contractual]). GOAT: batch100_rules_present + batch100_concepts_covered + batch100_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal lib 1262/0/1i + default lib 1012/0 count-identical + clippy exit 0 both states --no-deps (one doc-markdown round on the new //! header, 3 hits -- the B37 lesson's continuing strikes: slice_updates/entries_gpu_01.rs/simd_sum needed backticks) + retrieval floors 14/14 HELD (bands identical: clippy@1 89.6 / perf@1 96.2 / blend@1 94.7). Count pins walked: tests.rs corpus total 445->453 + expected-array A/D/E + sums x2 + grouped + router.rs expert total + mod.rs 453-rule doc + 3 category count comments + 3 list appends + the batch22 concept pin. Generator discipline: entries emitted by the Python heredoc generator with needle pre-verification (all 27 needles exact-substring-checked BEFORE emit -- zero case-mismatch gate failures, a first). Tooling hazards recorded (Lane A's side-finding): the search tools cannot index the .raw clone [gitignored -- silent no-matches] + multi-command terminal invocations drop all but the last command's output on this box. Record: Research 120 (status -> RECORD + mine-time appendix) / Plan 068.

Batch 95 (2026-09-01, Research 111 -- the mlx-lm gated_delta.py GDN prefill distill, Plan 055; ml-explore/mlx-lm @ main `mlx_lm/models/gated_delta.py`, Apache-2.0, read in full; the Bench-831 follow-up queue lead; 3 rules, corpus 411 -> 414, D+1 E+1 G+1; TWO parallel subagent lanes INDEPENDENTLY CONVERGED on the same three decision contents -- the strongest dedup signal observed in the program, coordinator merged both lanes' best halves and re-verified every quoted line byte-accurate): `simdgroup-row-packing-reduction-amortization` (E; the packed kernel packs 8 value rows per SIMD-group -- lanes_per_row=4, values_per_lane=32, in-lane 3-level pairwise tree + 2 row-group shuffles, grid y Dv -> Dv//8; the BIT-IDENTITY CONTRACT is the load-bearing half: power-of-two lanes_per_row + contiguous per-lane slices + in-lane tree replicating the LOW butterfly levels' pairing shape + per-partial sequential order preserved [IEEE + commutative not associative]; distinct from B28 plane-WG [ONE reduction per plane vs MANY rows per plane] + B22 substrate choice + B28 row-owned per-step rows + B81 column packing + B25 accuracy-driven trees), `whole-sequence-zero-barrier-single-dispatch-scan` (G; the FOURTH recurrence dispatch strategy beside scan [parallel-t, needs associativity] and chunked family [cross-workgroup handoff]: the ENTIRE T loop inside ONE kernel, state register-resident per-SEQUENCE [one entry load, one exit store], all reductions simd-local, cooperation radius pinned to one simdgroup so a barrier is STRUCTURALLY never needed -- exact by construction, no rel-error gate; eligibility = register capacity + simdgroup shape, NOT a sequence-length crossover; distinct from B79 scan + B12 chunk family + B28 row-owned per-step residency + B57 role-reversal [the zero-barrier elimination side] + B33 staging), `reduction-restructure-needs-source-owned-comparator` (D; pin a restructured reduction bit-identical against a comparator that WRITES the butterfly explicitly in source -- a library intrinsic's lowering is toolchain-owned, and packed==generic when BOTH call the intrinsic pins only a TOOLCHAIN COINCIDENCE [a driver update moves both kernels' bits together while the assert stays green]; sequencing load-bearing: comparator FIRST [device-scoped agreement recorded, never the contract], variant built against the contract SECOND, level mapping proven at the rewrite site; trajectory propagation = pinning ONE reduction pins every y + the final state; eligibility + kill-switch + comparator-as-A/B-arm part of the discipline; distinct from B57 [GATE-fixture half vs ORACLE half -- they compose] + B35 fwd/bwd parity + B79 [rel-error when no oracle exists vs bit-exact-by-construction when one does] + B51 across-builds vs across-restructurings + B23 which-kernel vs which-numbers). Riders (positive instances): MLX_GDN_PACKED module-level env read = B38; packed_eligible predicate = B23 hardcoded-fast-path; GQA hv_idx/(Hv/Hk) = B23 gqa; B79 confirmation -- its "mlx-lm at HEAD still runs the sequential per-token loop" note extends to GDN. 8 candidates rejected as dups/thin (ledger in Research 111). GOAT: batch95_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal lib 1235/0 (+1 = the new gate) + default 1001/0 + clippy 0 --all-targets both states + retrieval floors 14/14 HELD (blended band unchanged, no re-pins). Count pins walked: tests.rs 69-70/5362/5364/5375 + expected-array D85/E26/G26 + the stale "current 400" comment (2 batches stale -> 414) + router.rs 461 + mod.rs D/E/G comments + arrays. Full CI guard not run this cycle (the 3 affected states + floors + pins all validated; recorded honestly). Record: Research 111 / Plan 055.
                                                                          Batch 96 (2026-09-02, Research 114 -- the PrismML fork Metal quant-MV distill, Plan 060; PrismML/llama.cpp @ e311ed38f, the league-repinned opponent tip, clone ../vendor/prismml-llama.cpp, Apache-2.0; armed by the league opponent-watch intake a749024 / riir-ai Issue 843 T5 -- the 1.4x prefill jump root-caused REAL: ZERO PQ2_0 mentions in ggml-metal at the old pin 9ca265a, the type-142 Bonsai GGUF ran the generic dequant fallback there, while e311ed38f ships native kernel_mul_mv_pq2_0_f32 [73f0fed04, 08-25] + the Q2_0 twin + the TQ2_0 ternary codec kernel + wide-row set_rows + the FWHT weight-fold runtime; 4 rules, corpus 422 -> 426, N 24 -> 26, D 93 -> 94, B 54 -> 55): `telescoped-digit-coefficients-byte-fma-dot` (N; the HEADLINE -- the TQ2_0 mul_mv kernel scores packed 2-bit bytes as exact integer-valued floats: digit oracles floor(v * 4^-k) are BIT-EXACT [power-of-2 scaling, v <= 255 < 2^24], four telescoped per-byte coefficients c_k = y_k - 4*y_(k-1) absorb the positional decomposition so one byte costs 4 FMAs with zero shifts/masks/unpacking, the codec -1 offset rides as the -sumy accumulator SEED, coefficients hoisted across the weight-row loop; HONEST NUMERIC CONTRACT in-rule: extraction exact, products/sums reassociated => tolerance-gated, NEVER bit-identity; upstream lineage cited in-source: ggml-org/llama.cpp PR 26980; distinct from B92 prmt/lop3 + B44 unpack recipes [they EXTRACT -- this eliminates extraction] + O strength-reduce-div-pow2 [the oracle, not the content] + B17 sign-planes), `bit-plane-plane-sums-input-side-offset-dot` (N; Q2_0/PQ2_0 dot = positional bit-plane sums acc_lo + 2*acc_hi with the 2^k digit weight folded into ONE end multiply, and the (w-1) offset paid from the INPUT side: sumy = the activation-slice sum computed once per slice and SHARED across every weight row -- the offset plane costs zero per-row work and the format needs no per-block zero-point; branchless select-adds over raw bytes; the offset-codec GENERALIZATION of B17's deferred subtraction [B17's pos/neg sign planes cannot express (q - z) codecs]; distinct from B22 bit-plane layout + B83 filtering transpose + norm-decoupled scoring [composes]), `wide-row-copy-tile-rows-across-workgroups` (D; kernel_set_rows_f32_wide -- the in-source comment IS the rule: the generic one-threadgroup-per-row set_rows 'cannot saturate the memory bandwidth for rows of 100k+ elements' [recurrent-state snapshots, the GDN state write-back lane]; tile the row ACROSS workgroups -- 2D grid intra-row-tile x row, dims folded into grid.z, float4 slabs -- with a caller-side width threshold, 'a dispatch fact, not a device query'; the CONCURRENCY axis: saturation is machine-level, what must scale is workgroups in flight not lanes in one; distinct from B44 row-dot lane-splits + B33 output-owned reductions [no reduction here]), `fwht-shuffle-xor-butterfly-register-resident` (B; kernel_fwht -- the Walsh-Hadamard TRANSFORM fully in registers: N/LANES strided elements per lane, cross-lane stages via simd_shuffle_xor where the xor mask IS the butterfly partner and the sign rides (lane & stage) == 0, in-register stages above the lane count, ZERO smem + ZERO barriers, 1/sqrt(N) folded into the load; N 64..2048 at 32 lanes; the TRANSFORM side of the Hadamard family -- recursive-kronecker-cat is construction, B95's butterflies are REDUCTIONS [many-to-one] while this is a full EXCHANGE network [every lane emerges with distinct outputs]; consumer context: Hadamard incoherence folds for outlier spreading). Riders (no corpus edit): B20/B92 byte-perm family gains EXTERNAL MEASURED confirmation on a THIRD substrate (fork a7cc29645: HIP v_perm_b32 symbol expansion, MI300X Q1_0 112->322 tok/s @4B / 81->269 @8B / 31.5->97 @27B, PQ2_0 113->259 @8B / 48.6->91.7 @27B, bit-exact gates per the commit) + the CDNA-vs-RDNA MMVQ geometry regime note [keep split-K on CDNA where small_k runs several times slower]. Dropped with reasons (ledger in Research 114): GDN ring decode [batch-95 lane's external confirmation], im2col dispatch-mismatch fix [correctness, not decision content], Hadamard fold RUNTIME plumbing [codec/training-side; the fwht KERNEL yielded R4], MMQ id instantiations [plumbing], rs-rollback examples [host tooling]. GOAT: batch96_rules_present + batch96_concepts_covered + batch96_pseudocode_parses (cfg(not(kernel_verify)) per B70) + kernel_heal lib 1248/0 (+3) + default lib 1008/0 + latent_retrieval lib 1109/0 with retrieval_precision_regression_gate PASS (floors HELD, no re-pins) + clippy lib kernel_heal + default + all-targets kernel_heal all 0 (one doc_markdown round on the //! header -- the B37 lesson, 9th strike) + score-bench #26 96.2/1.9/100/100 FLAT. Count pins walked: tests.rs corpus-count 422->426 (+ the stale '423' message text fixed) + expected-array B/D/N + sums x2 + router.rs expert total 414->426 (+ stale '(400)' comment) + mod.rs doc + B/D/N category headers. Gate-catch: concepts_covered FAILED once on a case-mismatched needle (POSITIONAL vs positional -- the B51 case-sensitivity lesson re-proven live, the gate doing its job). Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Research 114 / Plan 060.
                                                                                                                                                    Batch 97 (2026-09-02, Research 117 -- the antirez/ds4 DwarfStar Metal distill, Plan 063; antirez/ds4 @ 110afdd8886586f18fc9b28bc5533152dd10e728, the DeepSeek-V4-Flash/GLM-5.2 native inference engine behind the league's 250.1 t/s @11709-token prefill M3 Max row, clone ../vendor/ds4; beta-quality fast-moving repo, every quote re-verified at mine time; provenance verified against upstream ggml-org/llama.cpp @ 9ca265a -- the falsification check CAUGHT one candidate: the flash-attn mask-block classification is llama.cpp-lineage, NOT ds4-original, re-provenanced and landed with corrected origin; 9 rules, corpus 426 -> 435, N 26 -> 27, E 26 -> 28, C 36 -> 38, H 14 -> 15, M 11 -> 12, A 98 -> 99, F 10 -> 11): `half-domain-exact-band-dequant-with-raw-bit-oracle` (N; the GOAT LEAD -- the resident MXFP4 prefill pair tile dequantizes through a 32-byte half code table instead of the 64-byte f32 table + per-element f32->f16 narrowing: E8M0 is a pure power of two and every E2M1 magnitude is exact in binary16, so for scale bytes in the closed band [103, 142] the half product is the once-rounded value of the float product; OUTSIDE the band keep f32 [half(scale) underflows to 0 below 103, saturates to inf above 142 -- e.g. scale 2^-25 x code 4 -> f32 path 2^-23, half path 0], and the 0xff NaN row stays arithmetic so GPU NaN payload canonicalization is unchanged; two consumption arms: per-block band check + 16-entry table, or the full 256x16 LUT generated offline by an exact RNE generator pinned by SHA-256 with a --check staleness mode; PROMOTION gated by an exhaustive raw-bit oracle over ALL 4096 pairs incl. both band boundaries -- our CPU twin `batch97_mxfp4_half_band_exhaustive_oracle` pins the band proof 640/640 + non-vacuity [out-of-band mismatches exist: e=102 -> 0 vs 2^-23, e=143 -> inf vs 32768] + boundary rows + the 0xff policy; en-route T6 bug CAUGHT BY THE GATE: the twin's normal path initially stored the IMPLICIT mantissa bit, overlapping the exponent field [0.5 -> 0x3C00 = 1.0] -- the subnormal path includes it, the normal path must not; format lineage MXFP4 = GGML, mechanism = DS4-original), `tail-simdgroup-cull-barriers-preserved` (E; routed-MoE GEMM tiles whose last work item has empty row halves: the cull predicate gates ONLY the mma/store bodies while staging and EVERY threadgroup barrier stay unconditional -- in-source 'Keep all threads in staging and at every threadgroup barrier, but let the second row-half skip MMA on short final expert tiles'; two ownership forms 16*(sgitg/2) < nr1 pair / 16*sgitg < nr1 compact-32x32; skipping a barrier in any simdgroup diverges the barrier count = hang/UB, and skipping staging breaks the strided output copy; the complement of the B57 barrier-adding rules), `sibling-projections-one-kernel-shared-staged-tile` (C; the TILED member of the same-input family: routed gate+up GEMMs share ONE staged activation tile per k-step so the k-loop pays two threadgroup barriers TOTAL instead of two per matrix, bit-identity to the unfused compose as the promotion gate; distinct from B15 fuse-gate-up-gemv-activation [the GEMV-level register-sharing twin -- this is the staged-tile + barrier-axis sibling], B63 weight-concat [impossible: gate/up are per-expert device buffers chosen at route time], B58 segments, B77 memo), `expert-major-work-map-token-scatter-clamped` (H; the routed MoE map built by ONE token-centric pass of per-expert atomic counter claims instead of the O(E x T) expert-by-expert token scan; expert-local order DELIBERATELY unspecified [self-addressing destinations], write clamp row < capacity + published count min(count, capacity), and the load-bearing trailer invariant 'Zero work must overwrite any stale prior value' -- consumers launch padded capacity + cull, so a stale count executes stale tiles; a counting-sort by atomic, distinct from atomic-scatter-reduction [values] + B94 sort-consumer-routes [external order contract] + B94 scratch-bank expert_major [bank LIFETIME, different concern]), `proven-shape-static-sibling-identical-results-fallback` (M; the static sibling differs from the dynamic kernel ONLY by baked constants and is documented byte-identical in accumulation order [compile-time trip count 128/16 = 8 steps + literal stride]; EVERY static entry keeps a 3-compare runtime shape check whose dynamic fallback is 'unreachable in production' but kept so 'a mis-selected pipeline produce[s] identical results instead of garbage'; NEVER delete the unreachable defensive branch as dead code; distinct from B23 [which-kernel axis vs derivation+fallback contract] and composes with B92 device-M [static where proven, dynamic-M where capture stability demands]), `barrier-after-async-mma-before-staged-tile-reuse` (A; mechanism-doc class, Metal4 TensorOps-only [not runnable on M3 Max, the B70/B92 precedent]: the async cooperative matmul reads staged tiles in flight, so the barrier AFTER mm.run + double-buffered staging is the lifetime proof -- alternation gives every tile a full iteration of slack, one barrier per k-step instead of two; upstream baseline pinned: ggml-metal's tensor mul_mm pays TWO barriers per k-step with a single staged tile, so the rule is the ds4 delta; direct-RHS + full-tile-dispatch-guarantee ride as adjacent disciplines; contract generalizes to wgmma/TMA-class async consumers), `bit-exact-dispatch-fusion-boundary-emulation` (C; fusing a DEPENDENT dispatch chain whose outputs are pinned bit-exact: every former dispatch boundary was also a compiler-optimization fence, so the fusion re-materializes each intermediate through device memory with a device-scope barrier + volatile-qualified reload [else the compiler contracts the stages' roundings] and reproduces legacy reduction ACTIVE-LANE topology [an if (tid < 8) guard recreates an 8-lane TG8 simd_sum inside a 32-thread group]; barrier-scope mem_device_and_threadgroup for cross-lane device writes; verified against full-vocabulary logits before promotion; distinct from B26/B19 merge family [no bit constraint, same-input] and B95 reduction-restructure [this REFUSES to restructure]), `virtual-thread-folding-preserves-reduction-tree` (E; fuse a wide fixed-shape producer reduction into fewer consumer threadgroups by folding the producer's legacy thread mapping onto the consumers as VIRTUAL slices -- every partial and pairing of the legacy tree survives bit-exactly, only how many times the tree runs changes [redundant recompute buys the dispatch + round-trip elimination]; the intermediate may be recomputed on the fly only when elementwise-exactly reproducible [y = x*scale]; shape gate load-bearing [masked tail iteration changes lane occupancy = bits]; cluster2 packs K legacy groups as independent simdgroup clusters hoisting the shared recompute [6 vs 12]; the time-slice twin: 32 legacy split-K workgroups through 8 physical simdgroups, same 32-lane reduce topology; lane-identity rule: keep operands on their legacy lanes so fast-math codegen does not move; distinct from B47 [the OPPOSITE trade: materialize once] and B95 row-packing [packs rows, this packs whole legacy groups]), `mask-block-classification-skip-placement` (F; PROVENANCE FALSIFIED -- llama.cpp-lineage, NOT ds4: upstream @ 9ca265a already ships kernel_flash_attn_ext_blk ['0 - masked (i.e. full of -INF, skip) / 1 - not masked / 2 - all zero', the (C > NW || Q > 1) amortization gate, the consumer classes incl. class-2 skip-staging, and the inline vec-kernel 'skip -INF blocks' simd_max check]; ds4 adopts verbatim; landed because the mechanism is ABSENT from our corpus, origin cited accordingly [the popcorn precedent]; 3-class mask-tile classification + the amortization-driven placement choice [pre-pass byte-map when Q > 1, inline chunk-max when one query row]; partial final blocks default to compute; stale map after mask edit = the failure mode). Dropped with reasons: candidate 7 row_blocks hoisted index [LICM/CSE territory + thinner B44 instance -- no bounded decision]; dense.metal yielded leads not rules this batch [quad-composition args-aliasing, sum6 route-fused down projection, shared-read-not-merging pair boundary, tiny-matvec parallelism down-shift, 2..8-token matvec-ext bridge -- queued in the snapshot]; host ds4_metal.m single-entry pipeline memo [P8-intake candidate]; M5 stable-sigmoid-vs-exp annotation [P8-intake candidate]. GOAT: batch97_rules_present + batch97_concepts_covered + batch97_pseudocode_parses (cfg(not(kernel_verify)) per B70) + batch97_mxfp4_half_band_exhaustive_oracle (the T6 mandatory gate) + kernel_heal lib 1252/0 + default lib 1008/0 + retrieval floors 14/14 HELD (~304 s, no re-pins) + clippy lib kernel_heal + default + all-targets all 0 (one doc_markdown round on the //! header -- the B37 lesson, 10th strike) + score-bench 96.2/1.9/100/100 FLAT. Count pins walked: tests.rs corpus-count 426->435 + expected-array A/C/E/F/H/M/N (the kernel_categories_counts_match_plan_003_table pin CAUGHT the category deltas -- the 18th-count-pin-site lesson re-proven) + sums x2 + grouped x2 + router.rs expert total 426->435 + mod.rs doc 426-rule + A/C/E/F/H/M/N category headers. Gate-catches: two case-mismatched needles (the B51 lesson twice) + the T6 implicit-bit bug above. Full CI guard not run this cycle (each affected state validated individually; the B95 honest-skip shape). Record: Research 117 / Plan 063.
                        Batch 84 (2026-08-28, Research 099 -- the oMLX v0.5.4rc1->v0.6.3 release delta, Plan 045; jundot/omlx PRs #2966/#3133/#3110, subagent-verified against PR pages + diffs + release notes at mine time): 2 rules + 1 rider, corpus 396 -> 398 (D 83 -> 84, N 21 -> 22) -- `pad-tail-to-fixed-tile` (D: residual tails >= the calibrated crossover floor(S*G/H)+1 = 1,357 run the RESIDENT fixed-shape tile instead of a separate small dispatch; ONLY intermediate activations zero-padded, synthetic rows stripped BEFORE any stateful consumer, prompt/KV/positions/recurrent state never extended; +29.0% e2e at a 2,047-token tail, -5.9% at half-tile = the regime IS the rule), `token-local-only-approximate-accelerator` (N: the delegation boundary for approximate compute is the consumer's STATE HORIZON -- token-local rows tolerate bounded approximation, recurrent-feeding rows stay checkpoint-precision, the fraction is a logged CEILING; +26.3%/+22.7% are PREFILL-THROUGHPUT, never quality [the research record's PP gloss corrected at mine time]; over-ceiling split diverged 3/5 probes at 32K while byte-deterministic), + the B56 rider `assigned-columns-only-staging-dequant` (slice to the device boundary BEFORE dequant; bit-identical; live-fields-only at column granularity). The 4th queued candidate (latch-failed-accelerator-shed-to-fallback) DEFERRED per the queue verdict -- fold into a measured-GOAT from our own status=0x9 work. GOAT: batch84 parse gate PASS + kernel_heal 966/0/1i + default 736/0 + all-features 1471/0/14i + clippy 0 in-repo --all-features -D warnings + CI guard ALL LAYERS PASS (after repairing 5 PRE-EXISTING latent breaks found by the guard, separate commit: the sibling kat-vote and_then E0631 + 3 more clippy classes) + retrieval floors HELD zero decay (kernel 100/100/100 -- 12th consecutive; blended 93.9/96.9/97.7). Research highwater collision class 5th occurrence: .research/098 was consumed mid-batch by a sibling's cflow verdict -- renumbered 099, highwater bumped 100. Record: Research 099 / Plan 045.

                        Batch 82 (2026-08-28, Research 096 -- the TreeWY + Launch-Bound arXiv distill, Plan 043; 2608.20961 B200/vLLM + 2608.26612 A100/torch, both fully read from the idle-loop queue-11 pool walk after the MCP search quota exhausted): 3 rules, corpus 393 -> 396 (D 82 -> 83, E 23 -> 24, G 22 -> 23) -- `wy-transform-tree-verify-state-free` (G: DFS PRE-ORDER layout makes the GDN transition system STRICTLY LOWER-TRIANGULAR; ONE forward substitution solves every pseudo-value for the whole draft, tree or chain; reconstruct ONLY the accepted state on commit; O(N*d_v) pseudo-values vs N+1 state blocks -- 128x smaller per head at d=128; win CONDITIONAL on memory pressure, 0.97-0.99x parity below the knee; trees affordable but NOT a throughput win -- EXTERNALLY CONFIRMING our Issue 721 structural negative on a different stack), `work-scaling-probe-launch-bound` (D: 32x the work at 1.55x the time = launch-bound -- 5.6-9.0x isolated kernels returned 0.999x end-to-end vs a MEASURED 1.072x Amdahl ceiling; f from wrapped record_function module ranges [81 modules], never kernel-name matching which undercounts twice; the kernels are integration-bound, not Amdahl-bound -- cross-validates our B76/B80 + Bench 761), `capture-break-count-not-perf-target` (E: eliminating all 23 graph breaks via dynamic-shape capture made the model 3x SLOWER + first-forward 1 s -> ~40 min -- unbacked symbolic-shape guards cost more than the eager dispatch they replace; restructure the op shape-static or accept the break; corroborated by TreeWY's piecewise-capture eviction exceeding the mask cost -- B58's corollary). The substitutability finding (2.7% route-replay attribution vs a 98%-collinear surrogate) kept research-record-only -- no bounded kernel fix-space. GOAT: kernel_heal 944/0/1i (941+3) + default 715/0 + all-features lib 1426/0/13i (main tree, pre-sibling-edit; the known l4_daemon env failures did NOT reproduce) + clippy 0 in batch files both states (5 doc_markdown backtick fixes) + retrieval floors 14/14 HELD (kernel 100/100/100 -- 11th consecutive) + CI guard Layer 1 26/28 in an ISOLATED same-parent worktree at 3d567f9 + exactly the 4 batch files -- the 2 fails (all-features, ternary_inference) are the riir-ai sibling's live riir-gpu write_many refactor, proven not-mine. Record: Research 096 / Plan 043.

   Batch 64: 2 rules from Research 072 (Plan 010 — the 2026-08-18 ARM Neon / bounds-check online-search pool, both candidates pre-verified absent; the pool's DROPS stayed dropped). `loop-order-innermost-contiguous` (simd 6 -> 7): i-j-k nested accumulation whose innermost loop strides a 2D access's OUTER axis (b[k][j] at stride-n) -> i-k-j reorder so the innermost varies the LAST index (contiguous row walk + hoisted row anchor) - **ParetoDominant 7.14x / 11.32x / 13.80x** at n=64/128/256 (BETTER than the source's 9x headline; the win GROWS with n as the column-stride cache damage compounds). Source: Arun Nagaraj matmul-simd (memory-hierarchy, not ISA). Distinct from B62's transposed-weight-register-gemv (that repacks the DATA for GEMV; this reorders the LOOPS over an unchanged layout — the no-repack general case). Float sum-order changes -> rel-error gate, not bit-identity. `debug-assert-length-hoist` (complexity 14 -> 15, annotated): debug_assert! mentioning .len() is ERASED in release, dropping exactly the constraint bounds-check elision needed -> assert! - **Mixed regime-split 1.13x@1K / STABLE 0.67-0.71x@100K on aarch64** (3-run verified) + the scope finding: asserts over DATA-DEPENDENT index arrays can never enable elision (no propagation through runtime index values — measured 0.74-0.80x dominated on the first harness shape); the rule's applicability is parameter/range-pinned indices only. Source: Shnatsel bounds-check-cookbook (the canonical article; its own aarch64 number was 1.03x). GOAT: corpus 15/15 bench-state + 607 default + 659 latent + retrieval floors HELD (blended 101: 92.1/96.0/97.0) + clippy 0 all-targets both states. Drive-by: the header's stale '65 entries' + 'simd has 5' (B62 had missed both) fixed. Generator lessons: 2 anchor aborts on B62-drifted text; the harness-in-file is the ESCAPED string form — raw-text anchors never match it (replace via index-of + json.dumps). Corpus 67 -> 69 (simd 7, complexity 15; benchable 59; annotated 14). Record: Research 072.

   Batch 66: 2 rules from Research 073 (Plan 011 — the polars kernel-idiom distill, the SECOND external-crate source via the `distill` skill; 3-lane verdict session + coordinator adjudication). `sparse-set-bit-iteration-tz-blsr` (arithmetic 5 -> 6): `for i in 0..64 { if (mask>>i)&1 != 0 }` -> trailing_zeros + `m &= m.wrapping_sub(1)` (BLSR) loop, popcount iterations not 64 - **ParetoDominant 4.08x/3.90x/1.54x/1.34x** at 1K/10K/100K/1M masks (~4/64 density; the win is LARGEST cache-resident + shrinks toward memory-streaming at 1M — the pool's 4-20x was the small-N estimate). Bit-identical (same indices). `sortedness-check-branchless-accumulate` (iterators 7 -> 8, ANNOTATED regime-split): early-exit `if v < prev { return false }` validation -> branchless `sorted &= prev <= v` full scan - **ParetoDominant 2.98x/2.72x** @100K/1M on usually-sorted data (the vectorize-by-removing-early-exit mechanism, the MIRRORED regime of filter-count-to-any); annotated because early-violation data flips it (the branchless form is ParetoDominated there — suggest only for usually-sorted validation). **The coordinator kill that shaped the batch:** the lane's GOAT `float-sum-multi-accumulator` was KILLED as a positive instance of `simd-chunk` (corpus.rs:283-284's fixture IS the shape) — salvaged as a simd-chunk spec-doc enhancement rider (the polars float_sum mechanism: strict-FP forbids reassociation so the win is N independent accumulator chains, STRIPE=16, + the tree-reduce finisher — NOT auto-vectorization of the serial form). **Standing dedup lesson: grep `buggy_code`/`fixed_code` BODIES, not just rule concepts — a rule whose fixture IS the candidate's shape is coverage.** GOAT: 612 default + 630 bench-feature + 664 latent lib tests (retrieval floors HELD) + clippy 0 both states + G-det alloc-stable + 0 allocs both rules. Count-pin sites found: 21 total for rust_perf (2 NEW — the unannotated/annotated counts in `drafter_propagates_context_notes` + `unannotated_rules_have_no_context_notes`; plus the 14->15 annotated family x3). Generator lessons: the B32c `json.dumps` `\u2014` escape bug re-hit (ensure_ascii=False STILL non-optional) + Rust has NO adjacent-string-literal concatenation (multi-line `context_notes` must be ONE literal with `\`-continuations or a single long line — the file's fixture lines set the long-line precedent). Corpus 69 -> 71 (arithmetic 6, iterators 8; benchable 61; annotated 15). Load-recorded: single-threaded benches under load ~4-6 (p335 active) — magnitudes robust. Record: Research 073.

   Batch 67: 1 rule from Research 074 (Plan 012 - the regex-automata distill, the THIRD external-crate source; source `rust-lang/regex` - NOT BurntSushi/regex which is a stale fork, regex-automata at ROOT). `byte-find-position-to-memchr` (iterators 8 -> 9): `xs.iter().position(|&b| b == lit)` on `&[u8]` -> `memchr::memchr(lit, xs)` - **ParetoDominant 5.00x/19.25x/25.31x/25.37x** at 32B/256B/4K/64K (needle at n-16, both arms ~full scan). **The predicted small-regime wash did NOT materialize** - memchr's tiny-input fast path wins even at 32B (the Teddy-bound wash concern was overly conservative; recorded honestly). The mechanism core (verified in rust-src 1.97.1): generic `iter().position(predicate)` has an opaque predicate + early-exit -> cannot dispatch to a byte primitive + cannot vectorize, while `contains` IS already specialized via `SliceContains for u8` -> core's SWAR memchr (core/src/slice/cmp.rs) - **the position/contains asymmetry IS the rule**; contains EXCLUDED from the trigger. Dep-gated (fxhashmap manifest-scan precedent: fire only when the target declares memchr). Arity ladder memchr/memchr2/memchr3 for `||` chains (the engine's exact ladder; >=4 needles skip). **Rider deferred with reason:** our own proposer helpers carry 3 live positive instances (collect_append_to_extend.rs:173, hashmap_with_capacity.rs:232, vec_macro_zero_fill.rs:183) but memchr is TRANSITIVE-only in riir-clippy - adding a direct dep for non-hot healer-internal scans fails cost/benefit; sites documented for a future touch. **Generator lesson: the `// [dep: crate]` marker needs the FULL spec** (`[dep: memchr = "2"]`) - a bare crate name generates invalid TOML (caught by the bench's own FAILED verdict, fixed in one pass). GOAT: 612 default + 630 bench-feature + 664 latent lib tests (retrieval floors HELD) + clippy 0 + G-det stable + 0 allocs. Corpus 71 -> 72 (iterators 9; benchable 62; annotated 15 unchanged). Record: Research 074.

   Batch 68: 1 rule + 1 rider from Research 075 (Plan 013 - the tantivy utility-layer distill, the FOURTH external-crate source; source `quickwit-oss/tantivy` `common/src/vint.rs` + `common/src/bitset.rs`, whole-repo MARGINAL B- but utility-layer YES B). `varint-compose-word-single-store` (io 1 -> 2): per-byte `write_all` varint loop -> 5-threshold ladder compose + single `write_all` - **MEASURED ParetoDominated 0/3 sizes x 3 writer regimes** (Vec-push / inline-Cursor / Box<dyn Write>, load ~25-33, alloc axis equal + G-det stable, outputs byte-identical by the exhaustive gate). **The honest-loser annotation is the product** (the `precompute-table` class - the rule exists to STOP the tantivy folk port): the folk div-chain mechanism dies twice - (1) 128 is a compile-time constant so LLVM strength-reduces `x % 128 / x / 128` to `&127 >> 7` (the B59 two-regime note in reverse: NO hardware-div chain exists to eliminate), (2) even N dyn-dispatch writer calls are hidden by OoO execution behind the ladder's 5-deep branch chain. **Standing distill lesson: a source shipping ladder-beside-loop as a 2-regime design is evidence the AUTHOR considered both - NOT evidence the ladder is faster on your box; x86 re-gate before citing.** The exhaustive gate (full u16 + 128^i boundaries + LCG u32, BOTH conventions - stop-bit-on-last [tantivy] + continuation-bit [protobuf], the B59 ported-numeric MANDATE) PASSES bit-identical; the fixture+harness canary pins the three ladder copies together. **Rider landed:** the tz-blsr `x ^= 1 << tz` ANTI-SHAPE + dependency-chain mechanism note (ARM64 rbit->clz->lsl->eor vs sub->and; x86 tzcnt->btc vs blsr; godbolt.org/z/fnfrP1T5f from `TinySet::pop_lowest`) - comment-only, zero count impact. Optional B- `unaligned-word-load-from-bytes` DEFERRED with reason (the same OoO-hides-call-chains verdict would likely reproduce on this box). Docs debt fixed in-passing: `.docs/11_domains/rust_perf.md` was 3 batches stale (69-rule counts + the B61-B67 table rows missing) - now 73/63/16 + all rows. GOAT: G1 exhaustive+canary PASS, G2 honest FAIL->annotation (never promote a perf gain on a wrong result), G3 clippy 0 both states + 620/638 lib tests + retrieval floors HELD (13 retrieval tests green), G-det stable. Corpus 72 -> 73 (io 2; benchable 63; annotated 16). **x86 RE-GATE 2026-08-19 same day (4090 box, quiet, load 10 — the annotation's standing condition discharged): dominated 0/3 there TOO, HARDER — 0.40x/0.43x/0.50x @ 1K/100K/1M, the ladder 2.0-2.5x slower; the honest-loser verdict is CROSS-ARCH.** Bonus same-window re-gates: `newton-refinement-vs-hardware-transcendental` x86 = ParetoDominant 1.64x/1.66x (never-loses cross-arch; M3 near-tie is aarch64-fdiv-latency-specific — the rule is STRONGER than annotated) + `debug-assert-length-hoist` x86 = Mixed 1.00x/0.92x (regime-split holds cross-arch; the article's x86 headline does NOT reproduce). All 3 annotations + Research 070/072/075 updated. Record: Research 075.

   Batch 69: 1 rule from Research 075 §Secondary (Plan 084 — the Plan 013 T6 defer MEASURED-REFUTED, same day): `unaligned-word-load-from-bytes` (io 2 -> 3) — shift/or byte-assembly word read (8 loads + 7 shifts + 7 ors) -> ONE unaligned `from_le_bytes` slice load. **The refutation's mechanism lesson: the Batch 68 varint honest-loser was CALL-count (N dyn-dispatch writer calls, OoO-hideable); this is LOAD-count (8 L1 loads vs 1 fused load) — OoO cannot hide 8 loads behind 1, and LLVM does not fuse the shift/or chain into a byte copy.** Cross-arch probe (M3 aarch64 load ~68-118 interleaved + 4090-box x86 load 7-31, median-of-5): u64 2.17-2.23x / 1.39-1.47x, u32 1.27-1.38x / 1.20-1.29x — ParetoDominant BOTH arches BOTH widths, G1 bit-identical at EVERY byte alignment. Validator GOAT: **ParetoDominant 3.76x/3.71x/3.67x @ 100K/1M/10M, 0 allocs both arms, G-det stable, retrieval floors HELD** — a WINNER rule, no annotation. u16 excluded (unmeasured, diminishing); u128-funnel stays out (unmeasured); proposer deferred (suggestion-first, B59/B68 precedent). Tantivy's current `BitUnpacker::get` already ships the fixed form — the rule targets the byte-assembly idiom found elsewhere. Corpus 73 -> 74 (io 3; benchable 64; annotated 16 unchanged). Record: Research 075 + Plan 084.

      Batch 65: 2 rules from Research 081 (Plan 021 — the hotpath.rs profiler contention distill; the 2026-08-19 pool landed 2026-08-22 once the quiet-box gate cleared): `align-64-false-sharing` (concurrency 4 -> 6) — adjacent 24 B per-thread atomic-counter slots -> `#[repr(align)]` to **ARCH** line granularity, ParetoDominant 4.7-9.4x at 2-12t (1t tie, honest) **with the beyond-source finding: align(64) is INSUFFICIENT on 128B-line Apple Silicon (recovers only 1.16-2.44x of the 3.3-10.8x win — pad to align(128) there or CachePadded)**; + `channel-send-batching-unpark` — per-item MPSC sends to a parked crossbeam-select! receiver -> per-64 `mem::replace` batch flush, ParetoDominant 1.16-1.29x + allocs 3232->1624 @100K (the margin is pipeline-overlap-honest: the source's 6.25x needs the park pair ON the critical path; take-vs-replace: take-regrow pays ~7 allocs/batch and measured WORSE than the buggy arm on the alloc axis). Both regime-annotated (the first measured-winner caveat class — the drafter token check now accepts ParetoDominant). GOAT: rust_perf 176/176 bench-state + default lib 659/0 + all-features 1117/0/13i via CI guard ALL LAYERS + clippy --all-targets 0 both states + alloc-det stable + retrieval floors HELD zero re-pins (kernel 100/100/100, blend@1 94.7). Corpus 74 -> 76 (concurrency 6; benchable 66; annotated 18). Proposers deferred (suggestion-first, B59/B67/B68/B69 precedent). Record: Research 081 + Plan 021.




The pipeline (shared across all domains):
1. **L0 — Rule distillation**: each rule → named entry + bounded fix-space
2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec, zero-alloc (Issue 001, DONE)
3. **L2 — Validator**: `syn`-based syntax parse check (always-on, modelless — Issue 002, DONE).
   Domain-specific verification is opt-in:
   - clippy: `cargo clippy` re-run (`clippy_verify`, Issue 003, DONE)
   - rust_perf: `cargo test` re-run (`rust_perf_verify`, Plan 001 Phase 2, DONE)
   - kernel_opt: CubeCL compile (`kernel_verify`, Plan 001 Phase 3, STUB —
     full numeric compare deferred per Proposal 012 §Phase 0)
4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
5. **L4 — LoRA fallback** (deferred): adaptive learning, last resort

**Plan 005 T3.5 (2026-08-16) — the G1 recovery contingency (Bench 027).**
Root-caused the clippy crowding to the BM25 tokenizer: the substrate default
`CodeTokenizer` discards ALL punctuation, but the clippy corpus is 40 minimal
generic functions whose entire discriminative signal IS punctuation
(`identity_op` = `* 1`, `unused_unit` = `-> ()`, `deref_addrof` = `&*`) — so
every rule collapsed to the same `fn f x` bag and the most generic one
(`identity_op`) topped 7 of 22 misses. Perf/kernel never had the problem
because they are identifier-rich. Fix: `src/draft/op_tokenizer.rs`
(`RustOpTokenizer` / `RustOpBigramTokenizer`) — identifier tokens
byte-identical to `CodeTokenizer` plus one token per maximal punctuation run
plus operator-adjacency bigrams, injected via the substrate's own
`with_bm25_tokenizer` hook; BM25's IDF decides what discriminates, no tuned
weights. **clippy top-1 54.2% → 70.8%, perf 87.5% → 97.9%, blended
70.8% → 84.4%; bit-identical across 3 runs.** Floors rebased (blended
80/89/90 of 96, clippy 33/42 of 48, perf top-3 46/48). Two REFUTATIONS
recorded: the latent fusion weight is inert (identical top-1 at 1.0/0.5/
0.25/0.0 — BM25 dominates), and centroid re-fitting cannot move G1 at all
(retrieval is fan-out, the router is annotation-only). **The ≥80% G1 bar is
still 9.2pp away — Phase 3 stays open on T3.5.4** (a top-k re-ranker: 9 of
14 top-1 misses have the right rule at rank 2-3, and top-3 is 89.6%).
Sweep: `cargo test --features latent_retrieval --lib retrieval_sweep_report
-- --nocapture`.

**Plan 005 T3.5.4 (2026-08-16) — instantiation re-ranker: built, benchmarked,
NOT promoted (Bench 027).** `src/draft/instantiation_rerank.rs` re-ranks the
retrieved top-8 by running each candidate rule's `propose_fix` visitor over the
span (one parse, K visitor walks) — rule *instantiation* instead of text
similarity, the distinction Bench 018's threshold refutation named as the
ceiling. On paper it clears the G1 bar: clippy top-1 70.8% → 87.5%
(`Structural`) → 95.8% (`All`), blended 84.4% → 96.9%. **Those deltas are
CONTAMINATED and must not be quoted as a gain**:
`clippy_fixture_labels_verify_via_proposers` asserts every clippy fixture fires
its OWN proposer and zero of the other 40 cross-fire — exactly the property the
re-ranker exploits. `rerank_circularity_diagnostic` proves it to the digit
(`All` top-1 = pool recall @8 = 95.8% = 46/48), so the table measures "was the
right rule retrieved", not "did re-ranking pick it". Per the GOAT rule the gate
did NOT pass → **`RerankMode::default()` stays `Off`, asserted by a test**.
Established non-circularly: safety (`clean_spans_fire_no_lint` — structural
proposers don't fire on 12 real production excerpts), coherence (`--fix`
already gates on the same visitor), zero cost when `Off`. **Unblocking it needs
an oracle independent of our proposers** (real `cargo clippy` labels — the
Bench 024 G7 harness already exists) plus spans where several lints genuinely
co-apply. Tracked as Plan 005 T3.5.5.

**Plan 005 T3.5.5 (2026-08-16) — uncontaminated oracle; re-ranker PROMOTED;
PHASE 3 CLOSED (Bench 027).** `src/draft/clippy_oracle.rs` asks real
`cargo clippy` (all three lint groups, JSON, compile-failure detection) what
fires on a span — an oracle with no connection to our corpus or proposers.
**It overturned half of the T3.5.4 diagnosis:** the labels were never wrong
(real clippy confirms 43/43 of the compiling fixtures); the defect is SAMPLE
SELECTION — 0/43 fixtures trip more than one corpus lint, so exactly one rule
can be right and a ranker has nothing to decide. Re-running G1 against real
labels reproduced the degeneracy exactly. Fixed by authoring the missing
sample: `src/draft/eval_fixtures_composite.rs`, 28 multi-anti-pattern spans
**labeled by the oracle, not by hand** (27 compile, 25 genuinely multi-lint;
a second batch avoids the first's `needless_return` pairing bias).
**Uncontaminated G1: off 84.0% → Structural 96.0% (+12.0pp)**, with the
degeneracy check now failing to fire (`all` 96.0% != pool recall 100.0%).
GOAT: G1 PASS, G2 +52-64us/span (+5.6-7.2%), G3 415/461/481/471 lib tests
across four feature states + clippy 0, G4 one `syn::File` per span, modelless
throughout → **`RerankMode::Structural` promoted to default**, pinned by
assertion. Committed eval after promotion: clippy 87.5/93.8/93.8, perf
97.9/97.9/97.9, blended 92.7/95.8/95.8; floors rebased (blended 88/91/91 of
96, clippy 41/44 of 48). **Phase 3's >=80% G1 bar is MET on both the committed
eval (87.5%) and the honest composite set (96.0%) — Phase 3 CLOSES.** Two
caveats carried forward: the committed clippy rows are a regression floor, not
an effect size; and 5 of the 48 original fixtures do not compile, so no
independent oracle can validate them. Run:
`cargo test --release --features latent_retrieval,clippy_verify --test uncontaminated_g1 -- --nocapture --test-threads=1`.

**Issue 024 (2026-08-16, RESOLVED + removed per noise-reduction rule; commit `e34ade9`) —
cross-domain switch-cost measurement + `RERANK_POOL` 8→12.** katgpt-rs Research 484's
switch-cost question asked of our healer: 12 directed clippy×perf pairs × 2
orders, paired fresh-baseline (`src/draft/switch_cost.rs` +
`eval_fixtures_cross_domain.rs`; real-clippy truth via
`tests/cross_domain_oracle_truth.rs` — 24/24 merged spans compile + carry
exactly the one labeled lint). At pool 8: perf→clippy **+8.3pp** (exactly one
pair, `needless_return`+`vec-with-capacity`, BOTH orders), clippy→perf 0.0pp.
Mechanism probed = **retrieval pool crowding, NOT fix interference**: the
correct rule crowded to raw similarity rank 9 by loop-shaped rules scoring 3×
its solo winner — one position past the re-rank pool, evicted before the
instantiation re-ranker ever saw it. Mitigation: `RERANK_POOL` 8→12 (deeper
pools add visitor walks, never new promotions). Post-mitigation **0.0pp both
directions, zero drops**; the Issue 017 eval IMPROVED (clippy 87.5/93.8 →
89.6/95.8, blended 92.7→93.8 — other latent near-misses rescued too; floors
re-pinned to blended 89/92/92, clippy 42/45/45); T3.5.5 composite G1 unchanged
at 96.0% (+12.0pp); rerank latency +39.6µs/span (+7.1%, same envelope);
`pool_depth_rescue_guard` pins the rescue. **Verdict: dense fan-out is
switching-cost-free at the corrected pool depth; the fix-ordering
SwitchCostTable consumer (the issue's anticipated gap branch) is REFUTED by
mechanism — ordering fixes targets edit interference, the gap was pool
crowding.** T3 (fix-axis) honestly skipped — the measured gap lives entirely
on the retrieval axis and the cross-fire label test already pins disjoint
Record: [Bench 032](.benchmarks/032_cross_domain_switch_cost.md)
+ the Research 484 healer-consumer row (katgpt-rs) resolves CLOSED.

**Issue 025 (2026-08-16, RESOLVED + removed per noise-reduction rule; commits
`589b0c9` + the `wgsl_mine` required-features fix in `bedf795`) — non-syn
surface scanner (`.wgsl` + `#[cube]` bodies).** T1–T6 all shipped: the
WGSL/cube scanner (`wgsl_scan.rs` — regex-pattern surface scanning with
precision guards discovered by live adjudication), the T5 control gate
(`tests/wgsl_t5_control.rs`), and the T6 mining integration
(`examples/wgsl_mine.rs` — git-history differential scan over both surfaces,
finding-level pairing, MOVED auto-refutation, Removed/Hunk diff association).
Two new patterns mined from the fix pairs: `scan_single_slot_staging` (DEFAULT,
fronts B35 — 2 live findings / 0 FPs across 33 `var<workgroup>` files; the
computed-write role_transport, tree reductions, transpose compounds, and
domino multi-phase are all guarded silent) + `scan_serial_row_dot` (P7
experimental, mining-only — 11 live serial-K GEMV instances textually
unadjudicable). Pair inventory 1 → **3 validated** (B47×riir-ai Issue 701 H-C,
B35×riir-ai Issue 686 H7, B44-body×Issue 698 A-H1); Phase 5 reopen trigger honestly
open at 3/5. Mechanized survey finding: ZERO `.wgsl` commits ever removed a
default-pattern shape (Bench 033's caveat now a proven fact); the genuine fix
pairs live on `#[cube]` bodies + other shapes. Live default-set 30 → 32
(gram.wgsl dead-file carriers, adjudicated real). Gates: 419 default / 602
kernel_heal / 727 all-features + t5_control 4/4 + mine_pairs 6/6 + Issue 017
retrieval floors PASS (corpus unchanged). Record:
[Bench 033](.benchmarks/033_non_syn_scanner_goat.md) +
[Bench 034](.benchmarks/034_wgsl_mining_integration.md). Remaining: the
Phase-5 reopen trigger (future fix commits enter the harness automatically,
one command per surface); the dead-`.wgsl` cleanup is DONE (riir-ai
`738dc2a66`, 2026-08-16 — 6 files / 211 lines removed, `wgsl_mine_pairs`
re-run 10/10 green); inline-CUDA surface as scope extension.

**Issue 026 (2026-08-17, RESOLVED-NEGATIVE + removed per noise-reduction rule;
commit `bedf795`) — entropic (max-seeking) selection mode (TTT-Discover
arXiv:2601.16175 transferred to heal-loop selection).** Shipped
`src/selection_entropic.rs` (KL-budgeted Boltzmann tilt: `solve_beta`
streaming zero-alloc bisection, `tilted_weights`, `tilt_advantages_into`; 9
unit tests incl. shift/scale invariance + exact KL budget + degenerate pools +
rare-success upweighting + auto-adaptation) + `SelectionMode::Entropic` opt-in
wired at `heal()` Step 4. **T2 wiring-site honesty:** the issue's proposed
sites were file-guesses — `AdaptiveDrafter` is an exact-key KV cache,
`RuliologySearch` is a structural Pareto ranking; neither has an outcome axis;
the ONLY outcome-scored pool is `heal()` Step 4 (loud degradation everywhere
else). **T3/T4 measured NEGATIVE (Bench 035): ALL FOUR reward-source ×
mapping variants (sigmoid(A)/w-max-norm × belief/raw-rate) lose the max
metric to BetaPosterior (0.9240/0.9379 vs best entropic 0.9132/0.9226) AND to
LinearBlend, across both evo-noise regimes.** Mechanism: heal-loop pools are
starved (0–3 observations) — the belief ranking the tilt consumes is mostly
thin-evidence noise; amplifying it drowns the correlated evo axis
(BetaPosterior's moderate swing leaves room for evo to rescue good
candidates). The paper's regime (dense reward batches, 512 rollouts/step)
does not have starved pools — same failure class as the `gemma2_directions`
GOAT (real signal, wrong regime). NOT promoted; variant + harness kept as the
reproducible negative-result artifact. B=10 saturated (~0.99 every mode);
B=3 discriminates. Also fixed in passing: the `wgsl_mine` example's missing
`required-features = ["kernel_heal"]` (latent `--all-targets` break from
`589b0c9`). Gates: 428 default lib / 448 full suite / 663
kernel_heal+latent_retrieval (floors HELD) / clippy 0 `--all-targets` / t93
unchanged. Record: [Bench 035](.benchmarks/035_entropic_selection_negative.md).

**Issue 027 (2026-08-17, RESOLVED same day + removed per noise-reduction
rule) — `--fix --write` false-positive cluster: single_match_else /
collapsible_if proposers fired where real clippy is silent.** Found live
during the queue-3 clippy sweep on katgpt-rs: `cargo heal --fix --write` on
`cluster_build.rs` applied 4 edits — 3 outright false positives + 1
pedantic-legit match_bool with mangled formatting (all reverted pre-commit;
katgpt-rs landed only the manual sweep fix `380aa9fc`). Root causes pinned
by an empirical probe matrix against real clippy 1.93 (minimal probe crate,
default + pedantic): **(1) single_match_else** — real clippy fires ONLY when
the wildcard arm is a statement BLOCK (expression wildcards `_ => 0` are
silent even under pedantic; a literal first arm fires under a DIFFERENT fix
shape — equality `if x == 0`, not `if let 0 = x`); the proposer fired on any
non-unit wildcard + any non-wildcard first pattern, and its own unit test
encoded the false-positive shape. Guards added: block-wildcard arm +
non-literal first arm. **(2) collapsible_if** — real clippy skips when a
comment sits between the outer block's open brace and the inner `if` (the
merge would displace it); the proposer now scans the lead + trail gap
slices textually (syn has no comment nodes). **(3) match_bool formatting**
DEFERRED `[-]` — span-preserving edits keep original text verbatim by
design; mitigation is rustfmt-after-write. The pedantic-lints-in-default-
runs policy is BY DESIGN (corpus includes pedantic lints; G6 truthfulness
runs all groups). NOT bugs: the needless_range_loop miss (indexing inside
`assert!` — syn cannot see into macro token streams; conservative skip is
correct, real clippy only fires on post-expansion HIR) + span coverage
(proposers scan whole files; the displayed spans are retrieval-only).
Memory cleanup: the 4 polluted Heuristic-tier trajectories removed
(memory.json restored to pre-incident state). Gates: 433 default lib / 484
latent_retrieval lib (retrieval floors + composite eval + corpus truthfulness
PASS) / 453 full default suite / clippy 0 both states; end-to-end re-verified
— the healer now proposes exactly 1 edit on the same file (the
pedantic-legit match_bool), zero false positives. The probe matrix extends
the `clean_spans_fire_no_lint` sample with the four rejected shapes.

**Issue 028 (2026-08-17, RESOLVED same day + removed per noise-reduction
rule) — the `collapsible_else_if` comment-guard gap (Issue 027 class in the
sibling proposer).** Found during the katgpt-speculative heal sweep (landed
katgpt-rs `e8cefad44`): 8 proposed `collapsible_else_if` edits where real
clippy (`--all-features`, pedantic) had ZERO findings — every site has a
comment between `else {` and the inner `if`, the exact Issue 027
`collapsible_if` shape the sibling proposer never got the guard for. Fix:
the lead + trail gap-slice comment scan ported verbatim (syn has no comment
nodes — the raw source slices between `{`/`if`/`}` are scanned; a broken
span range refuses rather than displaces). Root-cause note on the issue's
"match-arm guard" spec: the proposer structurally only visits `ExprIf`
else-branches — it can never fire on a match arm DIRECTLY; the observed
`and_or_builder.rs:228` comment deletion happened through composition (a
`match_bool` edit turns `false => { // c \n if let .. }` into an else-block
in the fixpoint pass, where the comment-blind proposer then collapsed
through the comment) — the comment guard blocks exactly that path, pinned
by the `match_bool_composed_arm_comment_blocks` fixture. A would-be label
guard on the else ExprBlock was verified UNREACHABLE (syn's else grammar
accepts only `if` or `{` — labeled blocks cannot be else branches) and
dropped, not shipped dead. Gates: 17 collapsible tests (6 new fixtures)
/ 477 default lib / `clean_spans_fire_no_lint` extended with the real
`find_decomposition_regions` shape / retrieval floors unchanged / clippy 0
in default + latent_retrieval states / the katgpt-speculative dry `--fix`
re-run: **0 edits proposed** (was 44 — 36 landed last session, the 8
comment-guarded sites now correctly silent).

**Issue 029 (2026-08-17, RESOLVED-PARTIAL same day + removed per
noise-reduction rule) — the healer MISS side of the sweep truth-diff
(the inverse of 027/028: real clippy fires, the healer stays silent).**
Two classes FIXED + one live hazard closed, three documented open:
**(T1) `range_plus_one` paren ends** — `0..(depth + 1)` parses the end as
`Expr::Paren` wrapping the Binary, rejected by the Binary-only match;
paren layers are now unwrapped first (weaver.rs:2642 class).
**(T2) `single_match_else` equality form** — int-literal first arms
(non-negative `Pat::Lit` holding `Lit::Int`) + block wildcard now fire as
`if scrut == lit { } else { }` (the Issue 027 probe P1 finding made
implementable); scrutinee parenthesized unless postfix-safe
(`helpers::is_postfix_safe` reused); bool/char/str literal arms stay
skipped (unprobed). **The live hazard: whole-match splices drop interior
comments** — `single_match_else` now refuses any match whose comment sits
outside kept fragments (scrutinee/pattern/arm bodies) via the new
`helpers::drops_interior_comment` (kept-range check; broken span
bookkeeping refuses). caddtree_budget.rs:232 was the guard's first
production test — `// First observation: use directly` survives verbatim
inside the kept arm body while the match still fires. Both fixes
LANDED end-to-end (katgpt-rs `c9c626120`): ground-truth confirmed vs
real clippy 1.97 pedantic `--all-features --all-targets` (single_match_else
33→31, range_plus_one 2→1), 1092/1093 lib tests (the 1 failure —
`peira_pruner::hot_path_overhead_vs_baseline` — PRE-EXISTING at HEAD
`e8cefad44`, verified via detached worktree; timing-sensitive debug-mode
test, unrelated). **Deliberate divergence documented:** decomp_reviewer:62
fires real clippy but stays healer-silent (interior comment between arms
would be displaced — our tool is SAFER than clippy there). **T3 — DONE
same day** (riir-clippy `ea76b3c`/`3983c15` + katgpt-rs `2215cac06`): the
`None`/`Err(_)` second-arm class, closed by a 20-probe matrix vs real
clippy 1.97 pedantic. **Probe verdict:** on std Option/Result scrutinees
clippy treats a NO-BINDING constructor arm as the wildcard — fires on
`None`, `Option::None`, `Err(_)`, `Err(..)`, `Some(_)`, `Ok(_)` as the
second arm (incl. reversed pairs `None`+`Some(_)`, `Err(_)`+`Ok(_)`);
SILENT on binding arm2s (`Err(e)`/`Some(v)`), user-enum arms
(`R2::Ok(_)`, `E3::None`, glob-imported `None` — clippy is
TYPE-INFORMED), expression arm2 bodies, and `None`-first + binding
second. All firing shapes suggest MachineApplicable `if let P1 = scrut
{..} else {..}`. **Parse-only approximation:** the extended arms fire
only when the FIRST arm is a std-shaped Some/Ok/Err/None pattern (bare
or `Option::`/`Result::`-qualified) pinning the scrutinee type — user
enums stay silent exactly like clippy; the emitted if-let form needs no
trait bounds (a compiling 2-arm exhaustive match proves P2 ≡ ¬P1, so
the rewrite is sound for every shape that compiles). 13 new fixtures;
clean_spans extended with the user-enum + binding-second silent shapes.
**Healed end-to-end:** 18 sites (katgpt-speculative 8 incl. the original
belief_cache/dd_tree misses, src/pruners/bomber 4, benches 4,
katgpt-pruners 2) — ground truth 118 → 96 repo-wide, EXACTLY −22
diagnostics for 18 edits (the 4 bomber sites each counted twice across
lib+test targets); zero remaining findings in healed files; no new
findings introduced. Healer-silences that AGREE with real clippy:
fft/sr2am_player HarnessUpdate (expression arm2 body) + comment-guarded
matches. Empirical syn facts: bare `None` parses as `Pat::Ident` in
syn (the `is_none_pat` three-arm lesson), `Err(..)` elems hold
`Pat::Rest`. Discovered in passing: the `cargo test -p katgpt-speculative
-p katgpt-pruners --lib` dual-crate invocation has a PRE-EXISTING
feature-unification break (katgpt-speculative's dev-dep
`echo_env_predictor` unifies into katgpt-pruners; the un-gated
`test_prediction_scorer_basic` references the `partial_scoring`-gated
`EchoPredictionScorer`) — verified identical at HEAD `4ccb2f280` via
detached worktree; candidate for its own issue. Remaining backlog
(next sweep unit): 96 repo-wide (31 in unswept katgpt-pruners files +
katgpt-forward/dd_tree + katgpt-core). **T4 — DONE same day** (riir-clippy
`337b0fa`/`7554d7b` + katgpt-rs `436cdda10`): the `uninlined_format_args`
miss classes closed by a 30-probe matrix (all `MachineApplicable` vs real
clippy 1.97 pedantic). Real scale measured: **1434 findings / 339 files**
repo-wide (the earlier ~96 estimate was one target subset; tests/benches/
examples dominate). The three extensions: **(1) spec carryover** — `{:?}`
`{:.1}` `{:>12}` `{:#x}` `{:5}` `{:+}` with ident args inline as `{v:?}`
etc. (spec verbatim; same arg, same trait bounds); ABORTS on dynamic
specs referencing/consuming another positional arg — `{:1$}` (digit-$)
and `{:.*}` (dot-star precision-from-next) — the inline deletes the
referenced arg; scope-captured name refs `{:>width$}` + fill chars
`{:*<5}` carry. **(2) macro family** — `panic!`/`format_args!`
(template-first) + `assert!`/`debug_assert!`/`write!`/`writeln!`
(template SECOND) + `assert_eq`-family (template THIRD); top-level
comma-sliced args with leading slices untouched; `unreachable!` stays
OUT (probe-verified clippy-SILENT while `panic!` fires). **(3)
already-named** `{name}`/`{name:spec}` copy verbatim + consume nothing
(mixed templates inline only the positional side). Documented divergences
(clippy fires, we stay silent): `{0}` indices, `{:1$}`/`{:.*}`, named-arg
`$` refs (scan_args aborts on `w = expr`). Coverage on the ground-truth
sample `tests/nf_flow_goat.rs`: 4/34 proposable before → **34/34**, healed
end-to-end 34 → 0 (compiles clean, 7/7 tests pass). Remaining: the
~1400-finding / 339-file sweep (high-coverage now; the next sweep unit) —
**slice 1 landed 2026-08-18** (katgpt-rs `9e9e1f89`): the root `tests/` dir
measured 2348 findings / 165 files (94% uninlined_format_args); 10 files /
554 edits healed with 77/77 tests post-heal, incl. 4 feature-gated binaries
compiled+run WITH their gates (ega_attn 11/11, parallax_attn 5/5,
kimi_k3_backward 5/5, bake_precision+bfcf_tree 10/10). ~1794 findings remain
across ~155 root-tests/ files. **Gate-identification lesson**: 3 of the 4
whole-file gates are `#![cfg(feature)]` inner attrs sitting BELOW the
doc-comment header (lines 11/19) — grep `^#!\[cfg` over the whole header,
not just line 1. Feature-gated files are invisible to default-features
`--verify` (the riir-chain gate-coverage trap), so their gate is a
feature-on test RUN — stronger than check. Slice selection rule: prefer
default-compiled + 0-ignored files (validation = verify + plain test run);
feature-gated files are fine but need their gate identified + exercised.
**slice 2 landed 2026-08-18** (katgpt-rs `7751f771`): 23 default-compiled
files / 556 edits (uninlined_format_args 552, map_unwrap_or 4), net −217
lines. Validation: `--verify` 0 reverts; all 23 binaries count-identical to
pre-heal baseline (113 tests); rustfmt applied + clean; ONE pre-existing
flaky gate documented (`bench_290 g2_motif_mining_overhead_under_5pct` —
debug-timing flap, failed a pre-heal baseline run too, passes on re-run).
**Second gate lesson (slice 2)**: whole-file `#![cfg]` + `required-features`
are NOT the only invisibility class — goat_090_tower_search has a MOD-level
`#[cfg(feature = "unit_distance")]` at line 12 (compiles EMPTY under
default, 0 tests run; the header scan classified it default-compiled).
The complete gate scan = `required-features` in Cargo.toml + `^#!\[cfg` over
the whole header + **mod-level `#[cfg(feature` greps cross-referenced
against the default feature set** — then exercise each non-default gate
(goat_090 @ unit_distance: 10/10; bench_ldt/bench_217 @ 5-feature union:
compile clean + green). ~1238 findings remain across ~132 root-tests/
~1238 findings remain across ~132 root-tests/
files; the gated giants are slice 3: goat_234 (103 @ manifold_pruner),
bench_245 (41 @ still_kv), parallax_sigmoid_stability_grad_clip (29 @
parallax_attn — gate at line 49, below the header); all-`#[ignore]`
bench_block_diagonal/bench_octopus stay skipped (compile-only validation).
**slice 3 landed 2026-08-18** (katgpt-rs `cea73d5d`): the 3 gated giants /
173 edits (all uninlined_format_args), net −87 lines. The healer's
`--verify` is blind to all 3 (default-features check strips them), so each
gate was exercised via a full test RUN pre/post-heal, count-identical:
goat_234 11/0 (debug), bench_245 12/0 (debug; `t22_still_kv_compression`
alone ~17 min — check per-file test runtime BEFORE picking slice members,
a 17-min test doubles validation cost), parallax 3/0 (**RELEASE — its doc
marks release REQUIRED: debug is ~30× slower and the divergence steps
shift, a 4th validation-mode class: mode-locked, not just feature-locked**).
Real clippy @ feature-on: 0 uninlined_format_args remaining on all 3
(remaining per-file warnings = pre-existing out-of-scope pedantic:
cast-precision, doc backticks, single-char names). **Clippy-target lesson:
`cargo clippy --features X --test NAME` lints the LIB dependency too — the
target's total warning count is dominated by `src/` findings (69
uninlined_format_args from the lib appear in a goat_234 target run);
per-file confirmation needs a file-path grep, not the run total.**
~1065 findings remain across ~129 root-tests/ files.
**slice 4 landed 2026-08-18** (katgpt-rs `ccf9fb98`): 48 default-compiled
files / 277 edits applied (268 single-pass sites: uninlined_format_args 224,
match_bool 24, map_unwrap_or 20; +9 bounded-fixpoint compositions), net −27
lines post-rustfmt. Validation: 41 debug + 7 release test runs count-identical
pre/post. **Gate-scan lesson (5th class): mod-level `#[cfg(any/all(feature…))`
around the test mod** — the 3-part scan (required-features + `^#!\[cfg` header
+ mod-level `#[cfg(feature` cross-ref) still missed it because these gates wrap
`mod tests` with `any(` lists of NON-default features; under default features
the binary compiles EMPTY ("0 passed; 0 failed") — 14 such files found
(bench_dflare_modelless dflare*, test_rubric_scalar_collapse ropd_rubric,
test_122_toast_goat toast_tokenizer, stepcode, …), all excluded (compile-only
validation class). **The reliable detector is the baseline run itself: "0
passed" ⇒ exclude.** **Debug-slow class (6th validation nuance): a
default-compiled file can be debug-unrunnable** — funcattn_g6_bug_poc burned
33+ min CPU in debug (doc says `--release`, ~30× slow) vs 390 s green in
release; bench_657/574/246/275/104/176 all baselined in release mode instead.
**Flake rule addition:** bfcf_lsh_cms_goat's baseline failed once under load
(9p/1f; `g5_roaring_batch_speedup` timing flap — the slice-2 bench_290
class), 3/3 green post-heal — a mismatch on a perf-gated test needs a re-run
before it counts as a regression. **Pre-existing failure left unfixed:**
bench_104_mls_k_sweep::bench_mls_k_sweep fails deterministically at HEAD
(K=1 cos 0.8839 vs 0.9 threshold, Plan-104-era tuning, machine-marginal) —
fails IDENTICALLY pre/post; heal validated on same-signature.
**Clippy group note: `uninlined_format_args` is PEDANTIC-group in clippy
1.93** — default `cargo clippy` emits ZERO of them; per-file confirmation
needs `-- -W clippy::uninlined_format_args` (that run: 0 remaining in every
tests/ dir workspace-wide; ~376 remain in crates' src/ — katgpt-core 27,
katgpt-spectral 16, attn-match 6, pruners 5, root 3 — the future src/ sweep).**
~788 findings remain across ~81 root-tests/ files — ALL in feature-gated
files now (~50 gate groups; biggest: bomber-wasm 31, mech_attribution 30,
hla_attention 30, msa_adaptive_k 29, parallax_attn 25); standing exclusions:
14 empty-under-default binaries + 6 all-#[ignore] files.
**slice 5 landed 2026-08-18** (katgpt-rs `a75dc385`): 12 files / 205 edits
across 8 validation cycles, cheapest-feature-first ordering — default
(bench_105_gdn2_goat 5 + bench_core_optimization 33), hla_attention (25,
zero-dep feature — the cheapest possible cycle), ict_branching (20), ega_attn
(19), sink_aware_attn (18), msa_adaptive_k (29), parallax_attn (25),
dynamic_rank,bandit (31, one superset cycle covers both the bandit-only and
the both-features gates). All runs count-identical pre/post; 1 edit
auto-reverted by the compile gate (redundant_closure @ bench_483:506).
**Gate-cycle method (the load-bearing lesson): feature-gated heals MUST pass
`--verify-args "--features F"`** — the default-features cargo check compiles
gated files EMPTY (0 tests), so a plain --verify proves nothing about them;
the same feature set must drive baseline run, verify check, + post-heal run.
**bench_core_optimization validation nuance (7th class): all-#[ignore]
no-assertion benches CAN be cheaply validated** — its 9 tests are pure
measurement ("no assertions" per their ignore reasons) but a release
`--ignored` run completes in ~6 s, giving panic-checking validation where the
all-#[ignore] exclusion class would otherwise defer the file wholesale.
**New divergence class recorded (named-arg-with-rename):** 3 sites in 2 files
(bench_105:416/420, bench_256:496) — `{spread:.3}` + `spread =
gdn2_max_spread` where real clippy inlines the variable into the template
(`{gdn2_max_spread:.3}`); our proposer documents already-named args as
copy-verbatim (Issue 029 T4's conservative rule). Extending the proposer to
rewrite named templates is future work (needs template-string transformation
+ fixtures). **Tooling notes:** `.goat`-suffixed files map to underscored
cargo target names (bench_256_adaptive_k.goat.rs →
bench_256_adaptive_k_goat — check the [[test]] name field, not the
filename); a dry-run's per-file edit attribution MUST be parsed from the
`fix <path> (N edits):` review sections, NOT from suggest-span headers (a
session wasted baselining 36 mis-attributed files before catching it —
the suggest pass interleaves per-file, so edit lines following a span header
belong to the PREVIOUS file's fix section).
**slice 6 landed 2026-08-18** (katgpt-rs `a02b68c1` + the Issue-669 fix commit
`a7bf7c0d`): 32 gated files / 223 edits across 8 feature-gate cycles,
cheapest-feature-first — (1) hardware_aware_scheduler,smear_classifier,
adaptive_gamma_forecast,dllm combined 42 (light/zero-dep features combine
safely in one cycle); (2) kimi_k3_backward SUPERSET 46 — it implies
kimi_k3_loader, so one cycle covered both gate populations (5 files);
(3) elf_sde,bandit 24; (4) dense_mesh 23; (5) mux_latent_wire 16;
(6) tiled_attention,kog_cpu_fusion,toast_tokenizer 21; (7) slod,engram,
event_log,vocab_channel_pruner,coexplain_riir 26; (8) the bandit-family
stragglers (committee_boost,delta_mem,partial_scoring,proof_sketch_evolution,
bfcf_tree,and_or_dtree) 25. All runs count-identical pre/post; 0 compile-gate
reverts; per-file real-clippy grep clean on all 32.
**Bug found at baseline (katgpt-rs Issue 669, fixed same day):**
bench_delta_mem_modelless::test_phase1_state_interference ABORTED pre-heal —
the test passed 3-element value vectors to a single feature_dim=5 hasher
(production uses TWO hashers: key dim=feature_dim, val dim=3 per
OutcomeFeatures), so simd_dot_f32 read 2 floats past the features slice:
SIGABRT in debug, SILENT UB in release (stack garbage folded into every
hash_value). The abort killed the whole 16-test binary; the feature sits in
no pinned CI suite (the gate-coverage lesson again). Fix: pad the test
vectors + add `assert_eq!(features.len(), self.feature_dim)` in
project_into (mirroring the existing out.len() assert — the guard belongs at
the typed boundary where the intended dimension is known, not inside
simd_dot_f32 where caller bugs are indistinguishable from intent).
**Validation-class additions:** (8th) `#[cfg_attr(debug_assertions, ignore)]`
release-required files (ugc_g1b_gate: 0p/3i debug, 3/3 release in 1.4 s);
(9th) `harness = false` + `fn main()` targets have NO test-result line —
validate by exit code + output diff (bench_299_engram_goat: byte-diff showed
only compile-line + debug-timing jitter 698→756 ns, same PASS verdicts);
(10th) heavy-manual all-ignored files (kimi_k3_4b_a3_forward_verify ~17.7 GB
alloc, kimi_k3_phase6_perf 1.5 GB model download) are compile-only — the
aborting-test class needs `-- --skip test_name` for count-identity when the
failure is pre-existing (or fix it, as katgpt-rs Issue 669 did). **fmt note:** several
slice-6 files carried pre-existing rustfmt drift, so post-heal fmt converged
them (net +108 lines incl. fmt churn — the heal itself is line-neutral);
post-heal test runs predate the fmt pass, so spot-check one heavily-reformatted
target post-fmt (rustfmt is semantics-preserving by construction but the
spot-check is cheap insurance).
**riir-train slice 1 landed 2026-08-18** (riir-train `e6084429`, this repo
`7ed7c6c`): the sweep's first riir-train unit — engine `src/`
default-compiled files only, 30 files / 183 edits (131 uninlined_format_args,
37 match_bool, 5 map_unwrap_or, 4 single_match, 3 vec-with-capacity,
3 single_match_else). **Scope discipline (the empirical-gate method):** the
engine src/ tree is ~75% feature-gated (~25 distinct gates; only 31 of 118
preview files are default-compiled by parser, 30 confirmed empirically via
rustc dep-info `.d` files after the parse — `csp_answer_pruner.rs` +
`rust_syntax_pruner.rs` parse-DEFAULT but are gated inside `pruner_aux`-style
parents; never trust a parse-only gate map when dep-info can prove it).
Sibling riir-train Plan 339/340 files excluded (kimi_k3_train.rs 162 edits,
optimizer_lora_muon{,_sf,_tests}.rs). Validation: check --lib + clippy --lib
0 warnings + 1253/1253 lib tests (default build only — no gate runs needed
by construction). **Root-cause proposer fix landed same cycle** (riir-clippy
`7ed7c6c`): map_unwrap_or on a provably-bool closure + `false` default now
emits `is_some_and` (real clippy 1.97 pedantic's own suggestion; the old
`map_or(false, f)` output triggered the DEFAULT-group unnecessary_map_or —
the heal had created a warning where the original was clean, Issue 027
class, probe-verified on /tmp/probe_mapor); bool+`true` skipped (clippy's
own suggestion there leaves its own warning); 7 fixtures; 555/555 lib.
**Remaining in riir-train** (~3645 edits): engine tests/ 71 files / 1259,
examples/ 88 / 926, gpu 84 / 562 + the excluded sibling files — each needs
its own gate map (tests/ carry `required-features`; gpu pulls cubecl),
slice 2+ material.
**riir-train slice 2 landed 2026-08-18** (riir-train `22447d9d`): engine
`tests/` default-runnable slice — 11 files / 182 edits (176
uninlined_format_args + 6 map_unwrap_or→map_or chain collapses; zero
control-flow rewrites). **Gate-map lesson for TEST files (supersedes
dep-info alone):** a `#![cfg(feature)]`-gated test file COMPILES to an
EMPTY binary under default features and still emits a `.d` file — dep-info
said 17 default-compiled files, but `cargo test -- --list` proved only 2
run by default (goat_247 via collimation_lora riding `training_verification`
transitively + plan_313 ungated). Ground truth for "what runs" is the
per-config `--list` count, not the .d file. **5 validation configs, all
count-identical pre/post-heal (47 tests green both sides):** default
(goat_247 7 + plan_313 1 — the slowest single test at 433s),
jvp_velocity_supervision (6 a5b files / 22 tests; full_go_arena 7 @ 299s),
weaver (bench_314: 1 run + 5 ignored), gemma4_lora (plan320_c2: 4), remax_ppo
(remax_ppo_convnet_minatar: 10). **Pre-existing breakage found:**
plan320_c2_goat_losses under `--features gemma4_train` ALONE fails E0432
(unresolved `gemma4_lora` — the file's inner cfg names gemma4_train but the
import needs the separate gemma4_lora feature, which implies gemma4_train;
the gemma4_train-only combo is a broken gate nobody runs). Clippy 0
warnings in healed files across all 5 configs. **Fmt scoped OUT with
reason:** the 11 files carry 94 pre-existing rustfmt diffs at HEAD + no
rustfmt.toml + no CI fmt gate — wholesale fmt would drag unrelated
formatting into a mechanical diff (the heal adds ~10 net-new diffs, same
class as the pre-existing ones). **Remaining in riir-train** (~3463 edits):
engine tests/ gated remainder ~60 files / ~1077 (each needs its gate's
build+run — the 5-config pattern scales), examples/ 88 / 926, gpu 84 / 562
+ the 4 excluded sibling src files (~206).
**riir-train slice 3 landed 2026-08-18** (riir-train `91c5f68b`): engine
`tests/` **xhc_pretraining gate cluster** — the largest single-feature
cluster in the gated remainder. 7 files share the gate (phase1-6c),
5 carried edits: **64 edits (61 uninlined_format_args + 3
vec-with-capacity)**; phase2/phase3 were clean. **Gate-map completion
lesson:** a whole-file `grep -n '#!\[cfg'` is required — phase1's gate
sits at line 38 BELOW the head-30 doc-comment window, so a head-N scan
under-reports; the complete map = Cargo.toml `[[test]] required-features`
rows AND a whole-file inner-cfg grep (files can have BOTH — xhc does).
**Validation under `--features xhc_pretraining`** (isolated /tmp target):
compile 0 errors, clippy 0 warnings in healed files, 0 remaining
uninlined_format_args (pedantic -W check); tests count-identical pre/
post-heal all green: phase1 5/0/0 (0.15s), phase4 4/0/1 (67s), phase6
4/0/1 (534s), phase6b 2/0/1 (938s), phase6c 2/0/2 (494s). **Slow-binary
lesson:** the cluster runs ~35 min/side (phase6b 15.4 min alone) — check
per-file runtime BEFORE the baseline (the slice-2 rule), then (a) split
post-heal runs one binary per command under the 20-min timeout, (b) SKIP
re-running binaries whose files got 0 edits (phase2 108s + phase3 25s
saved — content unchanged means the baseline run stands), (c) unscoped
`cargo test -p` builds the 1253-test lib suite too — always scope with
`--test <name>` (an unscoped run OOM-killed here).
**Remaining in riir-train** (~3399 edits): engine tests/ gated remainder
~53 files / ~1013 (largest next clusters: lora_still 4 files, nextlat 3,
ciacot 4 AVOID real-model deps, goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206).
**riir-train slice 4 landed 2026-08-18** (riir-train `1c03e1eb`): engine
`tests/` **lora_still gate cluster** — the largest next single-feature
cluster. 4 files share the gate (goat_267 family, BOTH mechanisms again:
Cargo.toml required-rows + inner `#![cfg]`); 3 carried edits: **10 edits
(all uninlined_format_args)**; goat_267_lora_still_training clean (0 edits).
Pure CPU feature (`lora_still = ["riir-engine/lora_still"]`, 0 GPU deps).
Cluster is FAST — every binary sub-second (4+4+7+8 tests, 0.03/0.02/0.11/
0.02s), so combined post-heal runs in ONE command are fine here (the
slow-binary split-run pattern from slice 3 applies only when per-binary
runtime demands it). Validated under `--features lora_still` (isolated
/tmp target): tests count-identical pre/post (7/4/4 green both sides,
training 8 green baseline), clippy 0 warnings in healed files, 0 remaining
uninlined_format_args. **Side observation (follow-up candidate):** the
pedantic -W check surfaced **17 pre-existing lib-side uninlined_format_args
in lora_still-compiled src files** (collimation_lora.rs etc.) — feature-
gated lib surface the default-compiled src slice 1 never saw; candidate
for a future src/ gated-lib slice.
**Remaining in riir-train** (~3389 edits): engine tests/ gated remainder
~49 files / ~1003 (largest next clusters: nextlat 3, ciacot 4 AVOID
real-model deps (gemma2/qwen25), goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206).

**riir-train slice 5 landed 2026-08-18** (riir-train `6ed32675` +
riir-clippy proposer `6aa0c82`): engine `src/` **lora_still lib slice —
the slice-4 side observation, corrected + closed**. The "17 gated lib
findings" framing was WRONG in an instructive way: enumerating the
findings under `--features lora_still` then under DEFAULT showed **12 of
13 are default-visible** — `collimation_lora.rs` (7 findings) sits behind
`#[cfg(feature = "collimation_lora")]` in lib.rs but `training_verification`
(the ONLY default feature) transitively enables `collimation_lora` — a
**parse-only gate-map failure in the REVERSE direction** from the slice-1
lesson (slice 1: parser said default, dep-info proved gated; here:
lib.rs cfg said gated, feature-chain reality said default). Gate truth =
resolve the feature CHAIN (default → implied features → compiled mod
set), never the cfg attribute alone. The remaining finding split:
3 × `game_concept_graph.rs` `bail!` + 2 × `rosetta/mod.rs`
`anyhow::ensure!` were sites slice 1's heal structurally could NOT touch,
and only 1 (`lora_still_training.rs`) was genuinely lora_still-gated.
**Proposer extension (the root-cause half, riir-clippy `6aa0c82`):**
anyhow's `bail!` (template-first, like panic!) + `ensure!` (cond-first,
like assert!) joined the macro family table, + `macro_family` now
resolves the LAST path segment instead of `get_ident()` (single-segment
only) so qualified `anyhow::ensure!`/`anyhow::bail!` — the production
spelling — reach the family match; qualifier-agnostic like real clippy's
post-expansion behavior. Probe evidence = the live findings themselves.
5 new fixtures (bare bail/ensure, qualified bail with escaped quotes,
qualified ensure, qualified std::format, ensure-without-message negative
control) + a real gpart.rs clean-span entry (already-inlined `{version}`
+ no-placeholder templates must stay silent). Gates: 560 default / 612
latent_retrieval / 623 fix_verify lib tests, clippy 0; consumer proof:
all 13 sites healed end-to-end. **Heal: 4 files / 54 edits** (the extra
41 beyond the 13 lib findings = cfg(test) mod sites the `--lib` clippy
scan doesn't see + 3 map_unwrap_or→map_or) — lib tests count-identical
pre/post (default 1253, lora_still 1271), clippy 0 both states, pedantic
-W residue 0 both states.
**Remaining in riir-train** (~3335 edits): engine tests/ gated remainder
~49 files / ~1003 (largest next clusters: nextlat 3, ciacot 4 AVOID
real-model deps (gemma2/qwen25), goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206 —
re-check sibling state at slice time). The lib-side gated-surface scan
(other features' src residue) remains a standing follow-up — enumerate
per feature the same way (default diff vs feature-on diff) before
assuming a finding class is gated.
**riir-train slice 6 landed 2026-08-18** (riir-train `0f76ae41`): engine
tests/ **nextlat-family gate slice** — the map's "nextlat 3" cluster was
actually a **7-file gate family**: the 3 nextlat_training binaries + 3
superset-feature siblings (substrate_nextlat / dagger_lora /
compression_sweep — all `= ["nextlat_training"]`) + the 4-feature-union
goat_gate. **One feature cycle covers the whole family** (the kimi_k3_backward
superset pattern): `--features nextlat_training,substrate_nextlat,dagger_lora,
compression_sweep`. 6 files / 35 edits: 4 test files / 30 (uninlined 28 +
map_unwrap_or→map_or 2) + 2 SRC stragglers / 5. goat_248_nextlat /
bench_248_nextlat_mlp / bench_254_goat_gate scanned clean (0 edits — no
validation runs needed, the slice-3 skip rule).
**The cheap all-ignored validation win:** all 4 edited binaries are
all-#[ignore] benches — a debug `--ignored` run gives full panic-checking
validation at 0.5-13 s per binary (~16 s total; the bench_core_optimization
pattern from the katgpt-rs slice 6).
**Third enumeration blind-spot class (the slice-5 lesson extended):** the
union `--tests` residue check exposed 2 src files that BOTH prior
enumerations missed — `src/compression_sweep.rs` (439 map_or in PRODUCTION
code + 731 assert) + `src/collimation_game_attribution.rs` (400 assert,
default-VISIBLE via the training_verification→collimation_lora chain).
Chain-default + cfg(test)-mod = invisible to slice 1's parse-only gate map
(said gated), slice 5's `--lib` clippy scan (no cfg(test)), AND default
`--lib` sweeps. **Enumeration rule for any gate slice: run `--tests` (not
just `--lib`) under the gate's feature union, then heal the union-minus-default
delta INCLUDING src cfg(test) mods.** Validation: lib count-identical
(default 1253 / union 1382 pre+post), benches 4/7/4/6 `--ignored`, clippy 0
residue union `--tests` + plain default.
**Remaining in riir-train** (~3300 edits): engine tests/ gated remainder
~43 files / ~968 (next clusters: goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each; ciacot 4 AVOID
— real-model deps (gemma2/qwen25)), examples/ 88 / 926, gpu 84 / 562 +
the 4 excluded sibling src files (~206 — re-check sibling state at slice
time). The lib-side gated-surface scan remains the standing follow-up
(now with the --tests cfg(test) component folded into every gate slice).
**riir-train slice 7 landed 2026-08-18** (riir-train `639d2eec`): engine
tests/ **goat_296 per-file-gate family slice** — the map's largest named
cluster: 7 test binaries, each behind its OWN feature gate (no supersets —
7 independent gates). One feature cycle with the 7-feature union covers
all: sparse_subnetwork_training (zero-dep), off_principal_init
(katgpt-core/newton_schulz), pion_optimizer_goat (riir-gpu/pion_optimizer
— compiles clean on M3 Metal; CPU clippy+test cycle, no GPU-exclusivity
concern), module_rank_profile_goat, sparse_lora_export_goat,
cross_game_mask_transfer_goat, cross_game_retention_goat (= cpu_lora_train
+ cross_game_mask_transfer). **6 files / 104 edits**: uninlined_format_args
38 clippy findings = 104 placeholder-level edits + 1 single_match_else
collapse (cross_game_retention run_game); goat_296_module_rank scanned
clean (0 findings, no validation run — the slice-3 skip rule). Per-file
edit counts (the `fix <path> (N edits)` review sections): pion 36,
off_principal 34, sparse_subnetwork 22, mask 5, retention 5, export 2.
**Validation:** 6 binaries count-identical pre/post (3/2/5/5/2/3 all
green, ~2min/side — cross_game_retention 105s the slow one); clippy
--tests union residue 0 in healed files; src-side union-minus-default
residue 0 (the slice-6 standing rule check — NO stragglers this time,
the src surface for these 7 gates is already clean); default-features
run skipped with reason (test-file-only edits — the lib is bit-identical
under any feature set, so a default cycle proves nothing about them).
All 6 files rustfmt-clean at HEAD → rustfmt applied post-heal (the katgpt
pattern; re-indents the single_match_else body), net −49 lines.
**Remaining in riir-train** (~3196 edits): engine tests/ gated remainder
~36 files / ~864 (next clusters: degrpo/wasm_reward_shaping/ted/posterior/
mdl 2 each; ciacot 4 AVOID — real-model deps (gemma2/qwen25)), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206 — re-check
sibling state at slice time).

**riir-train slice 8 landed 2026-08-18** (riir-train `07f39f27` + the
proposer root-cause fix riir-clippy `fa820d0`): engine tests/ **the 5 small
clusters — degrpo / wasm_reward_shaping / ted / posterior / mdl — ONE
5-feature-union cycle** (all pure-CPU; posterior_training pulls
rank_one_spectral, ted_regularized_lora implies rule_init_lora). 10 test
binaries scanned (the ted cluster = goat_244_rule_init_lora +
rule_init_convergence — goat_244's gate is the INNER
ted_regularized_lora, not rule_init_lora); 7 clean → skipped (slice-3
rule). **8 files / 48 edits** = 20 clippy findings: uninlined 9 (37
placeholder-level edits), match_bool 4, map_unwrap_or 6, single_match_else
1 (goat_posterior_264:602 — the diagnostic's "equality check" text is the
single_match_ELSE lint per its help URL; the healer's rule choice was
correct).
**TWO proposer root-cause fixes shipped mid-slice** (the bench_241 sites
were invisible to the old proposer): (1) the `unwrap_or_else` arm —
`x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)` (default closure
FIRST, mapper second — probe-verified against clippy 1.95's own
suggestion on the live sites); (2) **macro-argument descent** — the 4
bench_241 sites sit inside `writeln!` args, which are OPAQUE token
streams to syn's visitor (real clippy matches post-expansion HIR);
visit_expr_macro/visit_stmt_macro now reuse the format-family table via
the shared `post_template_expr_args` helper, parse each post-template
arg as an Expr, run the same matcher — proc-macro2 spans survive the
token round-trip so edits slice the ORIGINAL file bytes. Named/
non-parsing args are skipped, not aborting. 8 new fixtures; 568 lib
tests (+8); clippy 0.
**Third slice-1 miss class closed: nested-path default file.**
lattice_operad/bench.rs is default-visible (ungated `pub mod` +
non-cfg(test) `pub mod bench`) — its map_unwrap_or_else fired under
default `--lib`; invisible to slice 1's parse+dep-info map. Post-slice-8
the engine default `--tests` residue is ZERO (fully clean).
**Validation (all count-identical pre/post):** bench_246_mdl 7/0,
goat_posterior_264 6/0, posterior_arena_proof 5/0, default lib 1253+1i,
union lib 1386+1i, bench_241's own 12 tests green under
wasm_reward_shaping. Union `--tests` residue 0 in healed files.
rustfmt applied ONLY to the 4 files fmt-clean at HEAD (mdl_curriculum,
ted_regularizer, lattice_operad/bench, degrpo); the 3 test files +
bench_241 carry pre-existing fmt drift (scoped out, the slice-2 rule);
post-fmt lib re-run count-identical (the slice-6 spot-check rule).
**Side observation:** the gpu-crate findings visible under this union
(loss_sdar 5, compress 3, training_loop/types 2, + 4 singles —
DEFAULT-visible in riir-train-gpu) belong to the gpu 84/562 bucket,
confirmed not lost — the gpu slice owns them.
**Remaining in riir-train** (~3146 edits): engine tests/ gated remainder
~26 files / ~814 (ciacot 4 AVOID — real-model deps (gemma2/qwen25); the
remainder is the long tail of 1-file gates + the manifold/bake/archetype
families), examples/ 88 / 926, gpu 84 / 562 + the 4 excluded sibling src
files (~206 — re-check sibling state at slice time).
**T5 — DONE same day** (benches/ swept,
katgpt-rs `4ccb2f280`:
bench_424:97 range_plus_one + bench_136:208 uninlined — range_plus_one
now ZERO findings across all targets).
Note: syn 2.0.119's `PatLit.lit` holds a `Lit` (not `Box<Expr>`) whose
token preserves a leading `-` — the negative-literal exclusion checks the
token text. Gates: 36 range/single_match tests (23 single_match_else) +
28 uninlined tests / 514 default lib / retrieval floors PASS / clippy 0
lib + all-targets.

**riir-train slice 9 landed 2026-08-18** (riir-train `d64bc8ae`): engine
tests/ **the manifold/bake/archetype family union** — ONE 9-feature cycle
(boundary_stages,federated_alignment,curvature_regularization,symmetric_boundary,
dual_direction + manifold_lora,tjs_lora + bake_lora + archetype_library_training;
all CPU — archetype pulls dep:riir-neuron-db + katgpt-core/committed_field_blend,
in-tree, no models). 12 test binaries scanned; 7 clean → skipped (slice-3
rule). **5 test files / 150 edits + 2 gated-src stragglers / 38 edits**:
uninlined 184 (placeholder-level), match_bool 3 (goat_manifold_261 + msa
rescue — `match cond { true => X, false => Y }` → if/else, spot-checked),
vec-with-capacity 1 (deep_manifold_integration_test:99, the 1000-push
losses vec). The src delta (slice-6 standing rule): archetype_library_train.rs
(16 — incl. the PRODUCTION hex_encode `{:02x}` at 1182) + tjs_lora.rs (22,
box-drawing GOAT tables in cfg(test)). **Filename gotcha re-hit:**
bench_300_t4_4_msa_rescue is a `.goat.rs` file (target name
bench_300_t4_4_msa_rescue_goat via the [[test]] name field) — a wrong-path
scan silently covers 4-of-5 files (the slice-5 lesson, now twice-observed).
**Test-artifact hazard (new):** issue_307_archetype_library_gates REGENERATES
`models/archetype_library_v1/*.neuron` + manifest on every run (Bin 400→432
bytes) — restored via `git checkout -- models/` before both commits; future
slices touching archetype tests must NOT stage models/ churn.
**Validation (all count-identical pre/post):** the 5 edited binaries 1/2/13/1/5
(msa rescue 1, deep_manifold_integration 2, bake 13, manifold_261 1, archetype
5), union lib 1339+1i (pre/post via the copy-out/revert/baseline/restore dance
— the src heal landed before the lib baseline), default lib 1253+1i. Union +
default `--tests` residue 0 in the engine; plain clippy 0 new warnings.
rustfmt applied to all 7 (all fmt-clean at HEAD); post-fmt re-run
count-identical. Known divergence recorded: `match sphere_pass { true =>
"PASS", ... }` inside println! args stays healer-silent (macro-arg descent
is format-family-only) — real clippy would flag it; the documented
we-stay-silent class.
**Remaining in riir-train** (~2958 edits): engine tests/ gated remainder
~38 files / ~664 (the prior ~26-file estimate undercounted; the honest
enumeration: lota_ternary x2, lr_qat, dl_qat, coa_meta, osc_rat_plus,
zero_qat, gpart, rosetta_lora, rosetta_filter, polarization_stop,
adapter_centroid, nf_riir, snr_filter, eql x2, curvature_curriculum, nitp,
peira, set_diffusion x3, multi_precision_npc, latent_functor, fol,
dendritic, d2f_quality, ccar x2, integration_244, memory_soup family x8,
bench_401 sibling-caution; ciacot 4 AVOID — real-model deps (gemma2/qwen25)),
examples/ 88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206 —
re-check sibling state at slice time).

**riir-train slice 10 landed 2026-08-18** (riir-train `ac082be1`): engine
tests/ **the long-tail + memory_soup mega-slice — the named list from the
slice-9 map is now COMPLETE**. ONE 23-feature-union cycle (latent_functor-
rank_k-transfer, osc_rat_plus, coa_meta_adapter, dl_qat_magnitude,
lota_ternary_lora, lr_qat_lora, fol_extraction+game_concept_graph, gpart,
adapter_centroid [papaya], nf_riir_lora, dendritic_lora+ns_csg, eql_explain,
multi_precision_npc, nitp_loss, peira_loss+peira_metrics,
memory_soup_training+collimation_lora, set_diffusion, ccar_routing_goat
[katgpt-band], multi_source_lora — all CPU, in-tree). 38 files scanned /
26 edited (433 edits: 412 uninlined_format_args + 10 match_bool + 6
identity_op + 3 single_match_else + 1 vec-with-capacity + 1
collapsible_else_if) + **5 gated-src stragglers** (93 edits: eql_symbolic 20,
eql_explain 15, loss_nitp 3, multi_source_lora 1, dendritic_training 1 —
incl. 2 PRODUCTION single_match_else if-let rewrites in eql_symbolic.rs,
spot-checked semantically identical) + 12 clean files skipped (slice-3
rule).

**Exclusions honored:** bench_253_rosetta_memory_soup DEFERRED
(rosetta_cross_crate = dep:riir-gpu — belongs to the gpu bucket), ciacot 4
AVOID (real-model deps), bench_401 sibling-caution (riir-train Plan 339/340 territory).

**Validation (all count-identical pre/post):** all 26 binaries +
goat_240_eql_symbolic (src-affected CLEAN file — copy-out/revert/baseline/
restore dance, 10/0/0 both sides; the slice-9 method, now twice-proven);
union lib 1523+1i (the biggest union lib yet — memory_soup/rosetta/gpart
enable many cfg(test) mods); default lib 1253+1i (src edits confirmed
union-gated); clippy `--tests` union residue: **2 documented divergences**
(eql_explain.rs:243 — raw-string template `r#"{{...}}"#` w/ escaped braces,
the conservative JSON guard; eql_symbolic.rs:1036 — `format!("{}", x)`
nested inside assert_eq! COMPARISON args, second-level macro nesting the
descent doesn't recurse into; both join the we-stay-silent class — 2/526 =
99.6% coverage) + 2 riir-train-gpu sites (loss_sdar.rs:304, training_loop/
types.rs:621 — the gpu bucket's known property). rustfmt applied to all 31
(all fmt-clean at HEAD); post-fmt spot-check count-identical. No models/
churn (no archetype tests run). Pre-existing noted-not-fixed: unused `b_len`
at multi_source_lora.rs:113 (gated-src warning, not a heal-class lint).

**Terminal-tool workaround confirmed:** `'.' is ambiguous in multi-root
workspaces` fires RELIABLY on `cd <abs-path> && <cmd-with-relative-paths>`
but NOT on the tool's `cd` parameter + absolute paths in the command body —
the slice-9 "transient" is deterministic; use the parameter form.

**Remaining in riir-train** (~2432 by arithmetic: 2958 − 433 − 93): engine
tests/ exceptions only (ciacot 4 AVOID real-model deps; bench_401
sibling-caution; bench_253_rosetta_memory_soup gpu-gated) — **the engine
tests/ long tail is DONE**; engine examples/ 88 / 926; riir-train-gpu 84 /
562 (owns the known loss_sdar/compress/training_loop findings + the 2 sites
above); the 4 excluded sibling src files (~206 — re-check sibling state at
slice time); standing follow-up: lib-side gated-surface scan per feature
(this slice retired 5 of the gated-src surface files).

**riir-train slice 11 landed 2026-08-18** (riir-train `a1197e06` + the
proposer root-cause fix riir-clippy `c73770e`): engine **examples/** cheap-
gate slice — **49 examples / 578 edits + 11 gated-src / 139 edits (60
files, +737/−1087 post-fmt, net −350 lines)**. ONE 36-feature-union cycle
(all CPU, in-tree). **Gate-map discovery:** the examples tree is 120/124
GATED (only 4 default-compiled) — the healer dry-run found 90 files / 985
edits, classified by feature-dep chain into cheap (57 files/616 —
cpu_lora_train ×15 is zero-dep; game_llmexec + wasm_proof_witness + a dozen
more are EMPTY features) vs heavy (31/358 — kimi/gemma4/gemma2_lora_gpu/
shine/weaver/flashmemory/teacher/strandset/go, deferred) vs sibling-WIP
(13 src/370 — the Plan-339 kimi/muon/weaver/xhc family, last commit 00:04,
no fresh WIP but their plans 339/340/341 are live). **Dep-info src delta
(the slice-6 standing rule, examples variant):** `cargo build --examples
--features <union>` `.d` files → 131 src files compiled → intersect the
healer's 74-file src list → exactly 12 in-scope (reward_projection_data 60,
svr_driver 38, loss_latent_pred 9, csp_answer_pruner 7, svr 6,
token_sampler 4, smooth_latent 4, svr_answer 3, g_rollout 3,
rust_syntax_pruner 3, action_hoare 2, lora_outlier_guard 2 = 141; minus
action_hoare after the proposer fix below = 139 landed). 49 src files /
208 edits are NOT in this union (gated by other features — the standing
follow-up owns them). **Validation (all count-identical):** union examples
build exit 0 (65 binaries — dev-deps incl. riir-games/civ compile anyway,
making the civ/dual_leo examples cheap); default lib 1253/0/1i; union lib
**1802/0/1i** (the biggest union yet); spot-runs: trajectory_evolve_demo +
zero_qat_calibration byte-identical, ties_merge_bench timing-only diff
with ALL gate verdicts PASS both sides; clippy residue 0 across
uninlined_format_args/match_bool/map_unwrap_or (examples + src); rustfmt
applied to the 32 fmt-clean-at-HEAD files.

**THE HEADLINE — first live compile-gate catch ROOT-CAUSED in the healer
(the action_hoare.rs E0308):** the `bool-to-bitpack` proposer rewrote
`count_satisfied(&[bool])` → `&[u64]` while its live same-file caller still
passed `&[bool]`. A signature-changing rewrite cannot be applied
mechanically — it breaks every time the pattern has a live caller (the
NORMAL case; a zero-caller fn is dead code). **Fix (riir-clippy `c73770e`):
both signature-changing proposers (`bool-to-bitpack` param type +
`static-str` return type) now carry a caller gate — they refuse when the
fn ident occurs more than once in the file (word-boundary scan; call
sites, fn-as-value, macro args, comments all count). New fixtures for
both (mirroring the production shape) + `param_name_preserved` renamed
`fn count` → `count_bits` (its own `.count()` tripped the gate). 570/0
default lib, clippy 0, verified end-to-end (0 edits on action_hoare.rs).
`fxhashmap` is now the one remaining body-local type-changing rewrite
without a caller gate (type-flow exposure, self-gated on an existing
rustc_hash import).**

**Process lessons (both self-inflicted, both recovered):** (1) a
"dry-check" fmt loop accidentally executed `git stash push -- $f` per
file (62 stash entries; the sibling's pre-existing autostash sat at the
bottom — `.plans/320`, Cargo.lock, plan320 example, svr_driver — PRESERVED,
its pop correctly aborted on conflict). Recovery = sequential `git stash
pop` (60/60 restored, verified file-set-exact vs the planned scope).
NEVER put a stash call in a check script — the check ran it before the
intended `git show` fallback; use `git show HEAD:<f>` only. (2) One
timing-gate flake (`lattice_operad::transfer::tests::bench_transfer_overhead`
111.9x vs 100x) fired in the full parallel suite while the katgpt-rs
sibling's release bench ran at 298% CPU (load 7.7-9.3) — passes isolated
AND at `--test-threads=2` (the slice-5 flake rule + bench_290 precedent;
untouched file).

**Remaining in riir-train** (~1715 by arithmetic: 2432 − 578 − 139):
engine examples/ heavy remainder 41 files / 407 edits (kimi ×13/202,
gemma4 ×17/56, gemma2_lora_gpu + gemma2_latent_cot ×6/36, shine ×2/20,
weaver ×2/6, teacher ×2/3, strandset 1/5, go ×2/4, flashmemory ×2/4);
sibling-WIP src (kimi/muon/weaver/xhc family ~366 — re-check at slice
time, plans 339/340/341 live); 49 out-of-union src / 208 edits (the
standing gated-surface follow-up); engine tests/ exceptions (ciacot 4
AVOID real-model deps; bench_401 sibling-caution; bench_253_rosetta gpu-
gated); riir-train-gpu 84 / 562 (owns loss_sdar/compress/training_loop +
the 2 known sites).

**riir-train slice 12 landed 2026-08-18** (riir-train `67a77667` + the
proposer root-cause fixes riir-clippy `f0b47b3`): the **riir-train-gpu
crate slice** — the 84/562 bucket closed as **56 files / 425 edits**
(dry-run ground truth 63/468; 7 files/43 edits excluded). ONE
32-feature-union cycle. **Exclusions:** 3 sibling-WIP (muon_dense_cuda.rs
+ _sampled.rs at fresh Plan-339 mtimes 00:03, bench_335_hyperball_goat.rs
Plan-335 — the Aug-17+ mtime find is the detector) + 4 pre-existing
broken gates (dllm+gdsd_loss: distill_gdsd.rs:337 E0063 DllmTrainingReport
missing eval_errors/eval_masked_positions/train_loss_errors — the
bench_gdsd_gpu_goat gate compiles NOTHING at HEAD; sft_warmup:
bench_sft_rl_pipeline 6× E0063 mux_config + E0432 loss_asft — unfixable
by feature selection). **Union shape lessons:** the union needed
dion3_row_selection added (the sibling's committed muon tests reference
dion3-gated slab_l1_norms/gram_ns_ortho_slab — muon_dense_cuda without
its dev feature is another broken-at-HEAD gate) + asft_loss +
lora_outlier_guard (dasd_lora_goat's stale feature row); katgpt-rs is
STRUCTURALLY in the dep graph (riir-engine → katgpt-rs → katgpt-backend →
katgpt-forward) — dropping kimi features does NOT avoid it, so the
sibling's mid-flight TreePath migration blocked batch-2 for ~15 min
(poll `cargo check -p katgpt-forward` until coherent, then race the
build). **TWO proposer root-cause fixes** (the slice's compile-gate
catches): question_mark reference-scrutinee (match &self.field →
`self.field.as_ref()?`, not `(&self.field)?` — real clippy's own
suggestion is MaybeIncorrect here, probe-verified via JSON applicability)
+ vec-with-capacity suffixed literals (0..5u64 → with_capacity(5), bare
digits infer usize). **Validation all count-identical:** union lib
576/16/3i @ threads=2 (16 = 15 CUDA-absent + distill_attention_hla_
converges CPU divergence, both pre-existing), default lib 186/0/0, all
23 runnable test binaries (incl. goat_195's pre-existing 0/2 red),
test_kimi_k3_cublas_forward_g1 compile-only; 3 full-parallel flakes (2×
muon optimizer gpu_step + training_loop toy) all pass isolated + at
threads=2 — contention class, sibling bench at 200% CPU concurrent.
**Fmt-scoping lesson:** the stdin-mode `rustfmt --check` (git show |
rustfmt) is LENIENT vs file-mode — 11 files classified CLEAN were
actually drift at HEAD; caught post-fmt via file-mode re-check, fixed by
revert + re-heal (11 files land unfmt'd per the slice-2 drift rule;
45 fmt'd). Always fmt-check via `git show > tmp; rustfmt --check tmp`.
**Test-artifact hazard:** bench_interventional_sft writes a results CSV
(rm'd, unstaged). Clippy residue 0 in healed classes (union + default).

**Remaining in riir-train** (~1290 by arithmetic: 1715 − 425): engine
examples/ heavy remainder 41 / 407 (kimi ×13 needs sibling Plan-339/340
files settled; gemma4 ×17/56 plan320-autostash-adjacent — re-check at
slice time; gemma2_lora_gpu+latent_cot ×6/36, shine ×2/20, weaver ×2/6
teacher ×2/3, strandset 1/5, go ×2/4, flashmemory ×2/4); sibling-WIP
src (kimi/muon/weaver/xhc family ~366); 49 out-of-union src / 208; the
7 gpu-crate exclusions (43 edits: 3 sibling + 4 broken-gate — the
broken gates need owner fixes, not heals); engine tests/ exceptions
(ciacot AVOID, bench_401 sibling-caution); engine benches/ DONE
(`4306aeab`, this cycle).

**riir-train slice 13 landed 2026-08-18** (riir-train `df0ab789`): the
examples/ heavy remainder MINUS kimi/gemma4 — **16 files / 78 edits**
(15 examples + shine_d2l_train.rs src, 1 edit) across ONE 8-feature-union
cycle (shine_training, go_training_arena, weaver, teacher_data_gen,
strandset_data, gemma2_lora_gpu, gemma2_latent_cot,
flashmemory_indexer_train). **Pre-existing broken gates found at HEAD +
repaired** (all riir-ai Issue 701/699 API drift, fixes determined by the
API's own docs): shine_train_meta_lora + bomber_14
`compute_baseline_loss` ctx_len arg; go_11_shine_routing ×2
`route_context_to_expert` Option return (let-else); **shine_d2l_train
test gate was 7/17-red at HEAD** (riir-ai Issue 701 H-K hard error on the toy
config's under-provisioning) — `memory_length: Some(4)` → `None` (derive,
the sanctioned shape) → 17/17 green. Validation: union examples build
EXIT 0; shine_d2l 17/17 pre+post; default lib check clean; clippy residue
0 in healed classes. **CRLF trap (new fmt class):** issue429_logit_drift.rs
has CRLF line terminators at HEAD — `rustfmt --check` calls it CLEAN but a
write-mode fmt CONVERTS EOL, exploding the diff to the whole file
(`@@ -1,574 +1,552 @@`, every line). Caught via diff-stat; recovered with
`perl -i -pe 's/\n/\r\n/'`. Check EOL before fmt-write: `git show HEAD:f |
head -c 400 | xxd | grep '0d 0a'`. **Exclusions:** plan320_gemma4_q4k_gpu_train
(mtime Aug 17 = sibling plan320 territory), issue446_single_layer_lora_poc
(mtime Aug 18 06:48 = sibling-fresh; its 1 uninlined site stays), weaver.rs
+ weaver/autograd.rs ×5 (sibling Plan-339/340 src), riir-ai gpu-dep
residue ×4 (not this repo). 2 residual `unnecessary_sort_by` (default-on
lint, no corpus proposer — the manual_is_multiple_of corpus-gap class).
**Remaining:** ~1168 by arithmetic (1212 − 44): kimi ×13/202 + gemma4 ×17/56
+ plan320 (sibling-adjacent, re-check at slice time); sibling-WIP src
~366; 49 out-of-union src / 208; the 7 gpu exclusions; tests/ exceptions;
benches/ DONE (`4306aeab`).

**map_unwrap_or array-literal guard landed 2026-08-18** (`2bf2a44`, found
live in a concurrent session's WIP): `x.map(f).unwrap_or(&[...])` →
`x.map_or(&[...], f)` relies on the CALL SITE forcing U for the unsize
coercion `&[T; N] -> &[T]` — probe-verified both `&[]` AND `&[1.0]`
defaults E0308 in inference-transparent positions (plain `let`) while both
compile context-forced (function arg with known `&[f32]` param); the
original `map(f).unwrap_or` form ALWAYS compiles (`map` resolves the type
before `unwrap_or` unifies). The proposer cannot see call-site context →
guard skips ALL array-literal defaults, both forms (direct for `unwrap_or`,
default-closure body for `unwrap_or_else`); casts exempt (`as` ascribes).
3 new fixtures; 577 default lib tests; joins the we-stay-silent divergence
class (real clippy still flags the shape; our output would not compile).

**engine benches/ slice BLOCKED-DEFERRED 2026-08-18** (dry-run ground
truth: bench_382_math_dedup ×1 [default-compiled] + bench_317_poincare_g2_strict
×23 [`poincare_phi_train`] + gpu benches bench_381_artifact_freeze_goat ×19
[`artifact_freeze`] + bench_421_muon_ns_dispatch ×1 [`lora_muon_optimizer`]
= 44 edits across 4 files): **every non-default build path is blocked by a
concurrent session's live katgpt-rs src/ heal WIP** (137 modified files,
mid-flight since ~09:29) carrying the exact E0308 the guard above now
prevents — `katgpt-forward/src/step.rs:196` `map_or(&[], |m| m.as_slice())`
(3 instances in their diff) breaks every riir-train build whose dep chain
compiles katgpt-forward (poincare, gpu-crate, AND — after fingerprint
invalidation — the default path too; the default-path baseline squeaked
through at 09:28, ~60s before their 09:29:27 edit). bench_382's heal was
applied + surgically fmt'd, then REVERTED unvalidated per the discipline
(an unvalidated heal is not a heal — their tree is the live demonstration).
**Re-scan at next cycle WRITE time: `stat step.rs` + one default-features
`cargo bench -p riir-train-engine --bench bench_382_math_dedup` probe; once
the sibling's tree compiles, the full 44-edit slice is ~30 min** (heal
re-application is 1 command; baselines: 382 exit-code/output, 317
exit-0-all-pass semantics, 381+421 exit+output).

**RESOLVED same day ~2h later** (riir-train `4306aeab`, pushed): the
sibling fixed their E0308 (step.rs mtime moved 09:29:27 → 10:05:41;
`cargo check -p katgpt-forward` clean — the mtime CHANGE was the fix
landing, so a moved mtime is not itself a block signal, only the compile
probe is). The 44-edit slice landed exactly as mapped: 42×
uninlined_format_args (output-identical) + 1× match_bool (382's
`sigmoid_f32_reference` — the rewrite RESTORES the "verbatim copy of
`crate::math::sigmoid_f32`" comment's truth, broken since slice 1 healed
the centralized fn to if/else while the bench reference kept `match`) + 1×
single_match_else (421's verdict match on `(r8, r16)` → `if let ... else`,
2-arm wildcard-second, sound for any compiling 2-arm match). Validation
(class 9, exit + output): all 4 pre/post exit=0; **b317 verdicts + R²
BIT-IDENTICAL** (0.9989/0.9255/0.7168 — the trained-phi fixture is
deterministic); b381 ALL PASS; b421 HYPOTHESIS CONFIRMED both sides
(io_pct 80.9/65.7 pre vs 73.0/58.3 post — same verdict, GPU timing
variance); b382 criterion structure identical (noise annotations only).
Post-fmt re-runs green (b317+b381) + --no-run compile checks (b382+b421);
rustfmt applied to all 4 (fmt-clean at HEAD; hunks confined to healed
regions — 421's single hunk @332-372 = exactly the rewritten block).
Clippy residue 0 under each bench's exact gate. **Two measurement notes:**
(1) cargo's `Running benches/...-<hash>` filename hash is crate+features
identity, NOT source content — pre/post logs show the SAME hash while the
`Compiling <pkg>` line in each post log proves the rebuild (do not
mis-read identical hashes as "the heal never ran"); (2) normalized-output
diffs on criterion benches show noise-threshold annotation churn
(`(%) high mild` vs `low mild`, `No change in performance detected`) —
these lines differ between ANY two runs of the same unmodified bench;
verdict lines are the load-bearing comparison. **The engine benches/
surface is now DONE** (0 findings remain across both crates' benches/).

**riir-neuron-db slice 1 landed 2026-08-18** (riir-neuron-db `5db2e3d`) —
the re-probe confirmed the block STILL LIVE (step.rs mtime unchanged
09:29:27, E0308 reproduced via a direct `cargo check -p katgpt-forward`),
so the sweep pivoted to the only buildable heal surfaces while katgpt-rs
WIP is broken: **riir-chain + riir-neuron-db (neither depends on
katgpt-forward — leaf crates on katgpt-core only)**. Dry-run sizing:
chain 105 files / 420 edits, ndb 66 files / 978 edits (946 uninlined —
pedantic-group, invisible to default clippy, hence the accumulation).
This cycle: **ndb src/+tests/ — 37 files / 248 edits** (uninlined 231,
match_bool 12, map_unwrap_or 8, single_match_else 5, range_plus_one 1;
rustfmt on the 14 fmt-clean-at-HEAD files, 23 carry pre-existing drift
→ left unfmt'd per the slice-2 rule). Validation ALL count-identical:
lib default 412p/0f/4i + lib --all-features 1090p/0f/14i + all 7 test
binaries under their exact gates (elastic_lifecycle, limit_recall,
dense_embed+diverse_retrieval+local_kv, ed_freeze_poc, seed_recall,
item_funcattn_scorer, hebbian_fact_store+spectral_shard). Clippy
all-features: 1 PRE-EXISTING warning (index/mod.rs:397 unused_assignments
— untouched line, heal was at 1665), 0 uninlined residue.

**Tooling lesson (cargo tree --bench is a FALSE-negative probe):**
`cargo tree -p <pkg> --bench <name>` FAILS with "unexpected argument"
(exit 1, stderr) — piped `2>&1 | grep -c katgpt-forward` counts 0 matches
in the ERROR TEXT and reads as "not in graph". This cycle that
false-negative nearly overturned the (correct) block diagnosis: all four
bench targets showed "0 katgpt-forward" while the actual build failed on
katgpt-forward via `riir-train-engine → riir-engine → katgpt-rs →
katgpt-backend → katgpt-forward` (+ riir-engine → katgpt-forward direct).
Correct probes: `cargo tree -p <pkg>` (no --bench) or an actual build.

**Gate-map lesson re-confirmed (3rd time) + the shared-checkout interleave:**
4 ndb test files carry inner `#![cfg(feature)]` gates at lines 36-97 —
BELOW the head-30 doc-comment window (slice-3 lesson); the whole-file
`grep -n '#!\[cfg'` is mandatory and "0 passed" on a default run is the
reliable detector. A sibling commit (`7d19917`, riir-neuron-db Issue 603) landed on the
shared checkout mid-cycle touching only `.benchmarks/`+`.issues/` (dot-dir
benches never compile into --lib/--test runs) — 0 file overlap, baselines
unaffected; `git add src tests` scoping kept my commit exactly 37 files.

**Baseline flake documented (thermal/load class, user-suggested):** the
first COLD build+test run failed 1 timing test in BOTH lib configs
(411p/1f + 1089p/1f — compile load + M3 thermal throttle under the 503s
cold build); warm re-runs passed clean 412/0 + 1090/0 twice each. The
slice-5 re-run rule applies: a post-heal perf-gate mismatch needs a
re-run before it counts as a regression.

**riir-neuron-db slice 2 (benches/ + examples/) landed 2026-08-18** (riir-neuron-db
`501288f`, pushed) — **29 files / 730 edits (729 uninlined_format_args + 1
match_bool), net −300 post-fmt; the ndb heal territory is now DONE** (benches
were the "slow future slice" + examples the 2-file tail). One `--all-features`
mega-union cycle (supported ndb CI config; every required-features gate
satisfied — ~19 distinct single-feature bench gates + 6 gated examples).
**Composition note (the slice-5 parser lesson re-hit):** the dry-run's
`[clippy] <rule> score …` lines are RETRIEVAL SUGGESTIONS interleaved after
span headers, NOT edits — the first composition parse showed "82 iter_next_loop
+ 80 while_let_on_iterator" and looked like a proposer misfire; the REAL edits
are the `[rule ] line N` entries (grep `\[[a-z_0-9]+ +\] line`), 729+1. Verified
against ground truth: ZERO actual `while let .next()` / `0..(x+1)` patterns
exist in the benches. **Validation (class 9, FULL pre/post baselines for all
29 — captured in one loop; benches are fast measurement runs, seconds each):**
exit codes identical for 28; `bench_466_commitment_baseline`'s soft timing
gate (verify_commitment ≥ 2000 ns) FAILED at baseline (3040 ns) under box
load 36-48 (3 sibling compute processes) and passes post-heal 4/4 (297 ns
idle → 433 ns loaded — timing scales with load; the format-only heal cannot
affect it). Normalized diffs benign (compile-log noise — 478's baseline
included the cold bench-profile rebuild, `cargo bench` ≠ the dev-profile
build; digit-width table padding; timing-row shuffles). Verdict flips all
load-class + improvement-direction: 003 WARN→PASS, 466 FAIL→PASS, 478
budget cells NO→yes. **The 1 control-flow edit** (bench_478:113 match_bool,
`match i >= 5 {true=>&earlier[..], false=>&[]}` → if/else): the let-annotation
`: &[[u8; 32]]` drives the coercion in both arms — compiles + verdict lines
identical + it sits in graph CONSTRUCTION (setup), not the measured loop.
Clippy residue 0 under --all-features benches+examples; rustfmt on the 14
fmt-clean-at-HEAD files (15 pre-existing drift, left unfmt'd); no CRLF; lib
untouched by construction; `--no-default-features` + the default-compiled
healed example check green post-commit.

**Parallel-session collision on the SAME unit (katgpt-rs Issue 665 class, yielded
cleanly 2026-08-18):** a second idle session sized + healed the identical
ndb benches/examples unit CONCURRENTLY (deterministic healer, identical
input → byte-identical 730-edit output; sibling pushed `501288f` mid-cycle).
Detected at PUSH time via `git diff origin/develop` post-fetch — the delta
was exactly ONE unique hunk. Yield: `git reset --hard origin/develop` +
re-apply only the delta. **Standing lesson: `git fetch origin` BEFORE any
sizing/dry-run on a repo not touched this session** — this clone's reflog
showed last pull Aug-17 (stale by 8 commits incl. the sibling's slice-1
heal `5db2e3d`); the initial 77-finding src residue + ~180-proposal dry-run
were measured against the PRE-slice-1 base and were phantom territory
(already healed). Real current-tree residue after fetch: 12 findings, all
classified below.

**match_bool guarded-fallback extension** (riir-clippy `b221ab8` + ndb
`6685329`): the sibling's 730-edit heal left bench_478:299 unconverted —
`match under { true => A, false if g => B, false => {} }` — a 3-arm shape
real clippy 1.93 FIRES on (with no autofix of its own) that the old
proposer's 2-arm/no-guards gate skipped. Root-cause extension: the
`try_guarded_fallback` arm (middle arm carries the guard, last arm = EMPTY
same-polarity fallback → `if s { A } else if g { B }`, the empty arm is the
implicit else; mirror polarity `if !s` supported; guard-on-first-arm stays
silent — needs a nested rewrite). 5 new fixtures (production shape + mirror
+ non-empty-last/same-polarity/first-guard negatives); 12/12 match_bool,
597/597 lib. **NOT the array-literal hazard class**: match_bool restructures
branch SYNTAX only — the coercion context (typed `let`) is preserved, unlike
`map_or(&[], f)` which moves the literal into a new inference position.

**ndb terminal residue classification (2026-08-18, current tree):** 7×
match-in-macro-args (`ed_freeze_gate_poc.rs:895,901,920,924,1034,1038,1042`
— match_bool inside println!/format! args, the documented macro-nesting
divergence class, 3rd+ occurrence) + 1× same class (bench_478:372) + 1×
named-arg-with-rename (merkle.rs:1135→1137 `movement_proof_bytes =
movement_proof_size`, the standing backlog class) + 1× pre-existing
unused_assignments (index/mod.rs:397 `best_sim`, not a heal class, slice-1
already documented) + sense_training.rs HEALED (`b693f64`, 2 uninlined
incl. the `anyhow::anyhow!` ident site; residue 0 under
`--features sense_training`; lib 412/0/4 count-identical). **ndb is
TERMINALLY classified: zero healable residue remains.**

**Additional baseline facts the sibling record lacks (my independent
pre/post runs):** bench_133 exits 101 BOTH sides (G4 clean-heal 53.677ns →
52.426ns vs 50ns target — machine-marginal timing FAIL, pre-existing,
same-verdict). bench_003's WARN→PASS verdict flip root-caused as a COLD-RUN
artifact, NOT a heal effect: first-execution-after-cold-build measured
645µs (WARN, above the 500µs soft gate); post-heal warm runs stable
415-419µs; the ORIGINAL pre-heal source rebuilt + run warm gives the same
405-443µs PASS band — the two versions are timing-identical. (Method:
save healed copy → `git checkout -- <file>` → rebuild → run ×2 → restore.)


**riir-chain slice 1 landed 2026-08-18** (riir-chain `7421245a` merged `ec610745`, pushed develop) — **root lib src/: 47 files / +243−325 (net −82 post-fmt)**. Composition: 159 uninlined_format_args sites (156 first-pass + 2 `anyhow::anyhow!` + 1 `{self:?}` newly reachable via the proposer fixes below) + 21 match_bool + 17 single_match_else + 14 map_unwrap_or + 1 redundant_pattern_matching + 1 range_plus_one + 1 collapsible_else_if. **Exclusions (sibling 031-T0.1 territory, named by the t01-durability remediation review):** `src/asset_lifecycle/` (4 files / 57 edits — the sibling's freshest code, Aug-18 mtime detector) + `src/wallet/tx_log.rs`. **Branch discipline honored:** branch `fix/clippy-heal-root-lib-src` → 1 commit → rebase onto origin/develop (10 sibling commits landed mid-cycle — 21 files, ZERO src/ overlap, verified before rebase) → `git merge --no-ff` → push. **Validation (all count-identical pre/post):** default lib 320/0/0 (== the pinned `test_gate.sh` suite count); all-features lib **1848/1/2** — the 1 failure is PRE-EXISTING at HEAD (`consensus::rsm_fv::fv_gates::g1_c1_offline_correctness`, panics `rsm_fv.rs:54` assert 0 vs 1000000, `chain_fv`-gated so invisible in default; NOT the heal's — flagged for the owner, plausibly the sibling's 031 durability work); check --all-features clean; clippy residue in-scope = 0 for healed classes except **2 documented comment-guard divergences** (`guard_gossip.rs:231` interior arm comments + `slashing.rs:326` trailing arm comment — the rewrite would displace them, healer stays silent, SAFER than clippy); 32/47 files rustfmt'd (fmt-clean-at-HEAD only, 15 carry pre-existing drift); post-fmt spot-check (slashing module 42/42 green on rebuilt lib). **Gate-map method:** lib.rs cfg walk + per-parent-mod inner-gate walk → 25 distinct feature gates spanned → validated under `--all-features` (a first-class supported config here, unlike feature-union reconstruction). **Healer counter gotcha:** the second `--fix --write` pass reported "74 edits / 7 files" but the git-diff delta was exactly +1 file/+3/−7 (the 3 newly-unblocked sites) — the counter is CUMULATIVE per touched file, not new edits; the git diff is the ground truth.

**Two proposer root-cause fixes shipped same cycle** (riir-clippy `c85ff1c`): (1) `anyhow!` joined the template-first macro family (last-segment matching made `anyhow::anyhow!(…)` resolve to `anyhow` → not in table → silent, while real clippy fires — found as 2 live residue sites in riir-chain shard_quorum.rs); (2) the `{self} is not a valid format-arg name` guard was a **FALSE PREMISE** — probe-verified (temp crate, clippy 1.93): real clippy fires on `format!("{:?}", self)` sites AND `format!("{self:?}")` compiles + runs (rustc explicitly allows `self` as an implicit capture). Guard removed (`r#raw` exclusion kept); 2 fixtures; 571/571 default lib, clippy 0.

**Third proposer root-cause fix — the manual_map closure-unsafe control-flow class** (riir-clippy `493df27`, found live by the riir-chain slice-2 compile gates): the rewrite moves the Some-arm body into a NEW closure — `?`/`return`/`break`/`continue`/`await`/`yield` in the body change propagation or fail to compile there. Found as E0277 at chaind `chainspec.rs:238` (single-line rewrite) + `main.rs:652` (MULTI-line — a single-line `map(|`+`?` diff scan MISSED it; only the compile gate caught it). Probe-verified real clippy 1.97 stays SILENT on the `?`-in-arm shape (only `unnecessary_lazy_evaluations` fires there). Fix: syn visitor refusing `Expr::Try/TryBlock/Await/Return/Break/Continue/Yield` in either arm body; 3 fixtures (try/return/await shapes); 9/9 manual_map + 348/0 clippy_lints. **Scan lesson: multi-line rewrites evade same-line grep scans — the per-gate compile check is the only trustworthy detector for this class.**

**riir-chain slice 2 landed 2026-08-18** (riir-chain `b225ca70` merged `bb3082cc`, pushed develop) — **tests/ + crates/: 56 files / +190−371 post-rebase** (57 pre-rebase; the chaind node.rs overlap resolved to THEIRS wholesale — the sibling's 032-R3 replaced a heal site with a 40-line reviewed error block, and `--ours` during rebase takes the new base's file, dropping my 6 other format edits there — acceptable, their active file). Composition: 16 root test files (spec_match family, goat_322 family, catchup family, goat_307, bench_014) + 24 chaind src + 6 chaind e2e + 6 chain-sdk (found by the WRITE pass's fixpoint — the dry-run per-file counter UNDER-reports; the sdk wasn't in the dry-run list at all) + 4 wallet-family files. **Gate-map correction (slice-3 lesson re-confirmed): `nd` in test_gate.sh rows is a MARKER COLUMN meaning `--no-default-features`, NOT a feature** — passing it to cargo errors "does not contain this feature: nd". Inner `#![cfg]` gates found BELOW the header in 4 files (settlement_conservation:57, spec_match:17, e2e_wallet_sync:16, e2e_siwr_attest:29) — the [[test]] required-features list alone under-reports. **Validation — 13 cycles, all count-identical or same-signature:** spec_match pins all == (spec_match 8, quorum 6, shard 6, slashing 8, split_key 7, settlement 17); goat_322 g1-g7 all pass; chaind lib default 167 == pin (pre+post rebase); e2e_node_cert 11p + siwr_attest 11p/1i == pins; e2e union same-signature (e2e_browser 0p/2f env-needs-browser, visibility 3p/1f env-port — pre-existing at baseline); wallet lib 128p; sdk goat@auction 5p/2f **same-signature — 2 PRE-EXISTING failures** (`g3_wallet_and_program_operations_interleave` + `g3_only_program_receipts_carry_program_provenance`, panics in UNEDITED lines — noted for the owner); clippy residue 0 in all healed files (union scans per gate; lib residue = excluded asset_lifecycle territory only). **Baseline discipline note: unpinned binaries need PRE-heal runs** — the e2e failures would have been indistinguishable from heal breakage without them. **Residue-scan gotcha re-learned:** uninlined warning TEXT ("variables can be used directly") ≠ lint name — grep the `index.html#lint` URL lines or count `warning:` + `-->` pairs, never the name alone.

**Remaining heal territory:** ndb DONE (slice 1 src+tests `5db2e3d` +
slice 2 benches/examples `501288f` + terminal residue classification with
sense_training `b693f64` + the guarded-fallback site `6685329` — 2026-08-18,
zero healable residue); **riir-chain DONE**
(slice 1 root lib src/ `7421245a` + slice 2 tests/crates `bb3082cc` —
2026-08-18; residual = the excluded sibling 031/032 territory
[asset_lifecycle/, tx_log.rs, platform_gaps.rs, naive_replay.rs,
e2e_replay_recovery.rs, wallet_store_crash_atomicity] — re-dry-run when
their plans close); **riir-ai small-crates slice OPENED the riir-ai
campaign** (see below). **riir-ai giants: engine/src DONE 2026-08-19**
(`1e050442a`, 174 files / 979 edits — see the engine slice record below).
**games-civ DONE 2026-08-19** (`53afc933e`, 142 files / ~1552 edits — slice
entry below). Remaining: gpu (sibling-adjacent), engine tests/benches,
games Layer-3 (2 files sibling WIP), poc, mmorpg (excluded until
wire_router settles).

**riir-ai small-crates slice landed 2026-08-18** (riir-ai `883d51825` via
the worktree dance — the sibling's live WIP on ffi/gpu/games/721 blocked
in-place rebase; cherry-pick from `/tmp/ai_wt` onto origin/develop, ZERO
overlap with their 5 mid-cycle commits; clone state restored per the
discipline: update-ref + reset --mixed + checkout of the 8
committed-elsewhere files): **29 files / 164 edits across riir-data (29) +
riir-tools (14) + riir-rag (28) + riir-agents (26) + riir-router (25) +
riir-rest (2) + riir-net (5) + riir-validator-sdk (35)** — the FIRST
riir-ai heal of the campaign (the workspace was never swept; full sizing
~8.5K edits, riir-engine 2620 / games-civ 1549 / gpu 1175 the giants).
Excluded: riir-games-mmorpg (sibling WIP on wire_router.rs) + riir-ffi
(excluded from workspace, standalone build). All 8 crates count-identical
pre/post (data 158, tools 20-bin, rag 38, agents 16, router 67, rest 30,
net 29, validator-sdk 27). **Toolchain: riir-ai pins 1.95.0 but the box
resolved GNU + the global msvc_stl_shim config breaks GNU ld — every
build needs `RUSTUP_TOOLCHAIN=1.95.0-x86_64-pc-windows-msvc`** (the
Batch-49 trap, riir-ai flavor). Clippy residue 0 in healed classes
(riir-data re-healed post-extension: 8 more edits incl. the new redundant
named-arg sites); rustfmt on the 14 fmt-clean-at-HEAD files (3 CRLF in
riir-agents + 13 pre-existing drift scoped out).

**riir-ai games-quest + games-shared slice landed 2026-08-19** (riir-ai
`1992bde61` via the worktree dance — origin moved TWICE mid-session (ANE
726 + 459-T2); cherry-pick applied clean, zero overlap): **56 files / 611
edits** (quest 426 across 34 files + shared 185 across 24; core-wasm scanned
clean 0) — quest: uninlined_format_args + single_match/else + match_bool +
map_unwrap_or + range_plus_one; shared across src/tests/examples incl. every
gated module. The 2 quest_ternary files (1 edit each) DEFERRED — the
quest_ternary_adapter gate compiles riir-gpu (active sibling 721 WIP).

**7 validation cycles, all count-identical or same-signature:** quest default
(lib 179 + 8 binaries), quest 7-feature union (lib 524 + bench_038/285/287/
349/455/lora binaries), waypoint_vs_tile_astar bench (harness=false, exit+
criterion verdicts), quest_grammar_demo (exit 0, timing jitter only), shared
default (560/0/2i + doctests), shared all-features (906/0/2i + quant 26 +
doctests 14i), zone_expert_demo (byte-identical). 5 pre-existing failures
same-signature: bench_038 g4 (debug-fail/release-pass mode-locked timing,
the bench_578 class), bench_285 g1+g2 + bench_287 g2 (the documented
GOAT-FAILED compression-drafter family), quest_lora t4_1. 1 pre-existing
needless_range_loop documented (quest_style_bridge:471 — counter drives
forward-position semantics; real clippy's zip suggestion is an owner call).

**riir-ai games-civ slice landed 2026-08-19** (riir-ai `53afc933e` via the
worktree dance, rebased over 2 mid-session sibling commits): **142 files /
~1552 edits** (dry-run ground truth 146/1549; mix ≈ 1420
uninlined_format_args + 74 map_unwrap_or + 15 single_match_else + 7
match_bool + 1 identity_op) across src (93) + tests (~50) + benches (10) +
examples (4). **4 validation cycles, ALL count-identical (timing-stripped
result-line diff = 0):** default 1821/0/33, U1 (npc_sleep_time,
civ_spatial_cognition,karc_regime_mux) 1875/0/33, U2 (civ_economy,civ_roles,
civ_commerce,civ_aging,civ_trajectory_collect,ict_bridge — the riir-chain
cycle) 1960/0/33, benches-examples union. 10 benches exit+output identical
(bench_005's 30-line latency/sink jitter proven INHERENT via same-binary
control — post-vs-post2 diff = same 30 lines; bench_392 single wall-time
line; bench_479 verdicts identical, ns-jitter only). per_npc_clr_demo
byte-identical. **phase_profile PRE-EXISTING FLAKY (documented not fixed):**
8 same-binary runs → 7×exit-0 + 1×exit-101 with VARYING overflow panic
sites (spatial_cognition:141 / map_tick/history.rs:87) — HashMap-order-
driven debug arithmetic-overflow panics, inherent to the example.
**Excluded: 4 files / 10 edits behind UNDEFINED features** —
crowd_attention_bench + crowd_coherence_bench are referenced by
[[test]]/[[example]] required-features + lib.rs cfg gates but NEVER DEFINED
in Cargo.toml → those targets can never build (cargo silently skips — the
riir-train gemma4_lora broken-gate class; owner call to define or remove).
6 manual fixes for documented divergence classes (2× `{ }`
empty-placeholder-with-space adjacent to literal `#`; assert_eq! 2nd-level
macro nesting; closure-inside-sim_msg! non-family-macro opacity; 1×
map_unwrap_or_else→map_or_else; 2× heal-created map_or(false,·)→is_some_and
terminal — the Unknown-receiver fallback tripping unnecessary_map_or, the
Issue-027 class). Remaining documented divergences: 3× array-literal `&[]`
map_unwrap_or (vocab_clusters.rs:46 in shared + cgsp_production_wiring.rs:
26,30 — the 2bf2a44 E0308-proof guard class). rustfmt on 34 fmt-clean-at-
HEAD files (108 pre-existing drift scoped out).
**THE WORKTREE-BUILD DISCOVERY (exFAT):** E: is exFAT — NO junctions, NO
symlinks, NO hardlinks (mklink /J → "Local NTFS volumes are required";
PowerShell New-Item Junction → "Incorrect function") — the /tmp worktree
dance is IMPOSSIBLE for riir-ai on this box (sibling path deps
../../../katgpt-rs break at /tmp, and a same-drive worktree at E:\git\*
collides in the lockfile via riir-chain's OPTIONAL back-refs
`../riir-ai/crates/riir-engine|riir-wasm` — cargo loads optional deps at
lock-generation time regardless of features, and two DIFFERENT paths for
the same package = hard error). **The workaround: local-only manifest
surgery in the worktree (never committed): trim workspace members to the
civ closure [shared+civ] + redirect the closure's `../riir-engine`/
`../riir-wasm` refs to `../../../riir-ai/crates/...` (the REAL repo
paths — same package as chain's refs → no collision).** Validation
fidelity holds: the real repo's engine/wasm differ from origin/develop
only by format-only heal commits. Commit scoping: `git add crates/
riir-games-civ` + the staged-path guard. Also: `cargo test --examples`
builds examples as LIBTEST-wrapped unittest binaries in deps/ (running the
hashed exe runs 0 tests, never main) — the canonical run needs the
non-hashed `target/debug/examples/<name>.exe`. And a stale-clippy-cache
trap on exFAT: incremental-cache hard-link failures made clippy re-report
ALREADY-FIXED sites (sim/mod.rs 953/988 kept flagging after the patch
until a fresh compile) — verify reported residue against file content
before acting.

**THE HEADLINE — let-chain fold bug found live + root-caused (riir-clippy
`5d24795`):** the Q2 union compile gate caught `if ((cond) && (let "collect"
| "gather" = words[0].as_str())) && (let Ok(q) = ...)` — the bounded-fixpoint
composition single_match → collapsible_if → collapsible_if parenthesized
let operands, and a parenthesized `let` NEVER parses. The un-parenthesized
let-chain form is edition-2024-only; the parse-only proposer can't see the
target's edition → refuse (helpers::contains_let, the map_unwrap_or
array-literal guard class; real clippy also gates the fold on edition ≥
2024). Same guard on needless_bool(+assign) (`!(let a = b)` doesn't parse).
5 fixtures; 612/0 lib. The 3 refused folds in fol_quest applied MANUALLY
(workspace edition 2024, clippy's own suggestions — the manual-is-multiple-of
precedent: hand-apply where the proposer legitimately can't see the info).

**COMMIT-SWEEP LESSON (new):** `git add <my-paths>` + `git commit` commits
the WHOLE INDEX — the sibling had staged riir-poc/lib.rs +
ci_feature_guard.sh mid-session and my first commit swept them (58 files).
Caught at the `git show --stat` check; fixed via soft reset + unstage theirs
+ re-commit. Standing guard for shared checkouts: `git diff --cached
--name-only | grep -v <my-paths>` MUST return empty before commit.

Residue 0 in healed classes (healer dry-run: only the 2 deferred ternary
files); rustfmt on 20 fmt-clean-at-HEAD files (36 pre-existing drift scoped
out, 0 CRLF); post-fmt spot-checks count-identical. **Campaign remaining
(~6.5K edits):** riir-engine src DONE 2026-08-19 (`1e050442a`, see the
engine slice record below — 174/979; examples DONE `85224024f` 15/151;
wasm tail DONE `f43545fde` 5/23; tests/benches remainder ~1480),
games-civ 1549/146, games 799/87 (2 files sibling WIP), poc 769/64
(ane_t0.rs sibling-active), examples 725/75, gpu 1175/163 sibling-
adjacent, mmorpg excluded until wire_router settles.

**riir-ai engine/src slice landed 2026-08-19** (riir-ai `1e050442a`, via
the worktree dance): **174 files / 979 edits** — the first riir-ai giant
healed. Composition: 809 uninlined_format_args + 58 single_match_else +
44 map_unwrap_or + 43 match_bool + 2 identity_op + 1 range_plus_one +
1 manual_unwrap_or. rustfmt on the 61 fmt-clean-at-HEAD files (113
drift, **4 CRLF deltanet files** — flashmemory_gqa/model_backward_recompute/
qv_lora/qv_lora_train, the slice-13 trap, excluded from fmt).

**THE WORKTREE PACKAGE-COLLISION DISCOVERY (the load-bearing lesson for
any future riir-ai worktree):** `git worktree add` at a sibling-resolving
location (`/Users/katopz/git/riir-ai-wt` — NOT /tmp) still collides:
riir-chain/Cargo.toml:734 `riir-engine = { path = "../riir-ai/crates/riir-engine" }`
resolves to the MAIN checkout from any worktree, so workspace resolution
(with -p, even for unrelated packages — optional deps of members are
locked too) puts BOTH engines in one graph → "package collision in the
lockfile". Fix: scratch-edit the wt root Cargo.toml members to the
minimal engine closure (engine + wasm + router + data; gpu-async
auto-joins via path-dep auto-membership — none reach riir-chain) +
remove default-members; restore before commit (never pushed). Engine's
own `riir-chain` mentions are all comments — the collision is purely the
sibling back-ref.

**Gate-map method for a 244-feature crate (parse-only is hopeless):**
dep-info ground truth from the default build — 94 default-compiled /
80 gated of the healed files. **The .d files list WORKSPACE-RELATIVE
paths (`crates/riir-engine/src/...`) — an absolute-path intersection
returns ZERO matches** (first classification said all-174-gated until
inspected). Straggler gate forms the cfg-walk missed: `all(not(wasm32),
feature)` (cached_kv→quest_style_bridge), `all(target_os="macos",
feature)` (npc_ane_backend→ane, coreml-native builds fine on M3),
`#[cfg + #[path]]` includes (transformer_still→lora_still, default-on),
and parent-mod gate propagation bugs — walk fixes by inheriting parent
feats. Validation = default lib (3155/1/3) + ONE 66-feature union lib
(4566/4/9) covering all 80 gated files — both count-identical pre/post,
same-signature pre-existing failures (transformer cluster-map fallback +
3× sheaf_personality_bridge).

**5 heal-created Unknown-receiver map_or sites** — the katgpt-core manual-
fix class struck at scale: `.as_ref().map(f).unwrap_or(true/false)` on
field-chain receivers (classify_receiver Unknown → conservative map_or
emission) where clippy's type info proves Option+bool. Manually promoted
to terminal forms (is_some_and ×4 / is_none_or ×1): fourier/
path_periodic.rs:291, ict_runtime/runtime.rs:377/380/400, npc_ane_backend
:197. The closure-bool detector ALSO misses plain bool expressions
(`p.wraps_x || p.wraps_y`) — the is_bool_method allowlist covers method
calls only; candidate proposer extension.

**Residue = 8 findings, all documented divergence classes:** 2× single_
match comment-only None arms, 1× single_match_else comment-guard
(transposition.rs:67), 1× match_bool guard-on-FIRST-arm (events.rs:235 —
the silent class), 2× map_unwrap_or array-literal `&[]` (dual_gram_pca
— context-forced return positions where the 2bf2a44 guard is provably
over-conservative but can't see context), 2× uninlined second-level
macro nesting (cwm_runtime/refinement.rs:557-558 — inner format! inside
outer format! args, the standing backlog).

**riir-ai engine/examples slice landed 2026-08-19** (riir-ai `85224024f`):
**15 files / 151 edits** — the examples territory CLOSED (23 examples,
8 clean). 9 gate clusters healed under ONE 8-feature example union
(entity_cognition_stack ×4, ane_npc ×3, sense_composition_bench ×2,
cwm_runtime/domino_lora/channel_conditioned_draft/motor_gated_rehearsal/
octree_ctc/none ×1 each). Validation = the katgpt-rs root-examples method:
all 15 exit-identical pre/post (11× exit 0; 3 pre-existing exit=1 —
motor_gated_rehearsal_smoke_gate SMOKE GATE FAILED, wasm_simd_bench
native-usage-error BY DESIGN (wasm-target helper), g8_crowd_scale GOAT
FAIL; train_domino timeout=124 both sides, a trainer loop) + 15/15
normalized-output-identical (digits/hex stripped). **Clippy residue in
examples/ = ZERO** — the `--examples` scan lints the LIB deps too (the
clippy-target lesson): the 8 warnings were 4 documented engine-src
divergences + 3 riir-wasm wasm_pruner single_match dep findings (NOT
this slice's territory — the wasm tail's). rustfmt on the 7 fmt-clean
files (8 drift, 0 CRLF). **Engine territory remaining: tests/ +
benches/ (~95 files / ~1480 edits — the [[test]]/[[bench]] required-
features gate map is the long pole).**

**riir-ai wasm tail landed 2026-08-19** (riir-ai `f43545fde`): **5 files /
23 edits** — the wasm crate CLEAN (14 files total). Includes the 3
wasm_pruner single_match sites the engine examples union scan flagged.
Validation: wasm lib 88/0/0 count-identical pre/post (incl. post-fmt);
clippy residue in riir-wasm = 0 (the `-p riir-wasm` scan lints the
engine dep too — the 5 warnings shown are the documented engine-src
divergences).

**riir-examples examples slice landed 2026-08-19** (riir-ai `50c084f61`):
**28 files / 278 heal edits + 1 manual is_some_and terminal fix, +171/−291
net −120** — mixed EOL (14 CRLF, 14 LF). Composition ≈ uninlined_format_args
+ map_unwrap_or + match_bool + single_match_else + single_match +
vec-with-capacity + hashmap-with-capacity. **4 feature groups, all builds +
clippy clean:** default (7), cheap union frame,percepta,go,plasma_binary,
nextlat-go,neuron-vessel-aoi,action_generator (10), games union
combat,quest-match,dungeon,plasma,npc-dialog,meta_routing (9),
go-latent-steering (1). Validation: all 27 runnable examples exit-code
identical pre/post — 15 byte-identical + 6 timing-noise + 5
nondeterminism-control-exonerated (post-vs-post2 diff ≈ base-vs-post) +
quest_match_tui TUI frame-timing + gemma2 trio + go_05 deterministic
error paths (HF hub cache absent / AutoGo server absent).
**Residue 0** on all 28 (dry-run: exactly the 43-file / 436-edit excluded set
remains). Clippy: FS4 1 pre-existing needless_range_loop (go_16:316,
heal-untouched region — the conservative healer-silent class); FS2 1
heal-created unnecessary_map_or (neuron_vessel_aoi_demo v5 — the tuple-
destructure closure gate class, Issue-027 precedent) fixed to is_some_and;
1 pre-existing collapsible_if in src/realtime_arena.rs (src/ not examples —
out of scope). **EOL LESSON (new):** header-only EOL sampling (first 200
bytes) is UNSOUND — 8 files were CRLF below an LF header; rustfmt
write-mode on Windows converts CRLF→LF whole-file. All 4 EOL-bombed files
reverted byte-exact (content preserved via pre-fmt stash copies); 14
originally-CRLF files scoped out; 1 file fmt-polished (gemma2_cpu_bench
−2/+1). **Broken gates documented (owner flags):** arm_c E0601 (default
`--examples` breaker — [[example]] block missing required-features while
inner cfg gates out main), ane-arena ×4 E0599 (macOS-only CoreML backend),
catchup_demo E0308 (stale riir-chain API — RESOLVED upstream since: the
example compiles AND runs green at HEAD, verified 2026-09-05; only its
~heal edits remain deferred), go_13 (empty-feature gate but
body needs go-training→riir-gpu). **Deferred (~436 edits):** all
riir-gpu-pulling example families (sibling 721/ANE WIP) + ane-arena (M3)
+ catchup_demo + arm_c + go_13 — re-arms when the sibling's GPU work
lands. **Push lessons:** /tmp worktrees VANISH on this box (the previous
session's commit object was lost with one — worktrees go under E:/git/
like the siblings'); an E:/git worktree of riir-ai CANNOT build feature
unions pulling riir-train-engine (riir-train's path deps resolve
../riir-ai = the MAIN checkout → "package collision in the lockfile"
between two riir-engines) — the main checkout is the only self-consistent
build site for those; the push used the ref-move + reset --mixed dance
(commit made in the worktree, pushed from the main repo, then sync).
Origin moved TWICE mid-session (engine/wasm/issues commits — zero
overlap, fast-forward clean). Post-push verification at final state: all
4 groups built clean (6m50s / 9m48s / 6m45s / 4m04s). **Campaign state:
examples DONE; sibling lanes active on games-civ (heal-civ2 branch,
commit 53afc933e staged for push in the prunable ai_civ_wt worktree) and
engine (src 174/979 + examples 15/151 + wasm 5/23 landed on origin).
Remaining: engine tests/benches (~1480), games Layer-3 (2 files sibling
WIP), poc (sibling-active), gpu (sibling-adjacent), mmorpg (blocked on
wire_router), + this slice's ~436 deferred GPU-gated example edits.**

**riir-ai engine tests/benches slice landed 2026-08-19** (riir-ai
`e8f758995`): **95 files / 1481 edits — THE riir-engine TERRITORY IS NOW
FULLY DONE** (src 174/979 + examples 15/151 + wasm 5/23 + tests/benches
95/1481). Composition: 1437 uninlined_format_args + 21 single_match_else
+ 11 map_unwrap_or + 7 vec-with-capacity + 4 match_bool + 1 semicolon
across 71 test files + 24 harness=false bench files. **Gate map (the long
pole):** [[test]]/[[bench]] `required-features` from the manifest +
whole-file `#![cfg]` scans → 5 default-compiled / 57 manifest-gated / 33
inner-cfg-gated; ~68 distinct features validated under ONE release
mega-union (the src-slice pattern applied to test targets — builds tests
via `cargo test --release --no-run`, benches via `cargo bench --no-run`).
**Validation (release, all 95 binaries pre/post):** 71 tests
COUNT-IDENTICAL; 4 pre-existing failures same-signature at clean HEAD
(bench_315 g3_throughput 2×-gate — debug-pass/release-FAIL mode-lock, the
near-threshold class; bench_336 g6e_no_regression_light; bench_338_g9
g9_perf_mmo_scale_10k_npcs; latent_functor_rank_k t41/t42/t43). 24
benches exit + normalized-output identical (2 benign digit-width padding
diffs; bench_363's 200ns latency gate LOAD-FLAKY — post-heal 3/3 green at
125ns under the p335+r sibling-rustc load, the bench_133 class).
Post-fmt spot-checks count-identical; rustfmt on 21/22 fmt-clean-at-HEAD
files (3 CRLF + 70 pre-existing drift scoped out); residue 0 (fresh
healer dry-run over all 220 files).

**Slice lessons (tooling):** (1) DEBUG baselines unrunnable for karc —
bench_332_karc_cross_game/goat are ~650s RELEASE runs (debug 30×+ — the
first debug-baseline attempt burned a 20-min window); release test
builds are CHEAP after the bench release build shares deps. (2) Direct
binary execution beats `cargo test --test X` under a 68-feature union —
each cargo invocation pays ~4 min of fingerprint checks; run the
target/release/deps binaries with cwd = package root. (3) The
base/post-output script bug (hardcoded `base_bench_out_` prefix — post
overwrote base outputs) repaired via revert-benches → rebuild → rerun
with mode-prefixed outputs → restore; always parametrize the output
prefix by run mode. (4) **Cargo.lock noise in worktree diffs**: the
minimal-members scratch surgery REGENERATES Cargo.lock (+2590/−12879
under the union) — always `git checkout -- Cargo.toml Cargo.lock` before
commit + filter diff stats by path. (5) `git diff --stat` without a path
filter sweeps the Cargo.lock churn — the apparent 14k-deletion heal was
actually 696/1152 across the 95 files.)

**Campaign state after this slice (2026-08-19):** engine territory
FULLY DONE (4 slices); games-civ landed by the sibling (`53afc933e`, 142
files / ~1552 edits); riir-examples landed by the sibling (`50c084f61`,
28 files). Remaining giants: games 799/87 (2 files sibling WIP), poc
769/64 (sibling-active), root examples 725/75, gpu 1175/163
(sibling-adjacent), mmorpg (blocked until wire_router settles) + the
~436 GPU-gated example edits. **Native rustfmt landed FIRST this session**
(riir-clippy `10b4ac6`) — future slices get automatic fmt; this slice
applied it manually (the heal predated the feature).

**Redundant identical named-arg class shipped** (riir-clippy `73ab2c2`):
`format!("{x:?}", x = x)` — the named arg EQUALS its ident — is redundant
under implicit capture; real clippy 1.95's MachineApplicable fix DELETES
the arg, template untouched (probe-verified on the live riir-data
`compression = compression` / `h2 = h2` sites). The proposer previously
aborted on ALL assign-shaped args; now `n = n` (both sides the same plain
ident) deletes, composing with positional inlining (rustc's
named-after-positional rule keeps indices stable). The RENAME class
(`a = b`, e.g. merkle.rs `movement_proof_bytes = movement_proof_size`)
stays backlog — its fix needs template rewriting. 3 fixtures (delete +
compose + rename-negative); 41/41 uninlined, 603/603 lib. **Follow-up
sweep opportunity: re-dry-run the already-healed repos (ndb, katgpt-rs)
for the new class — the identical-name sites there were left as "named-arg"
divergences by the older proposer.**

**slice 7 landed 2026-08-18** (katgpt-rs `805feb22`..`81ef2631`, 12
commits) — **the katgpt-rs root tests/ sweep is COMPLETE**: 40 files /
358 heal edits + 1 manual lint + 3 manual import-gate fixes across 12
feature-gate cycles, all count-identical or same-signature validated, 0
compile-gate reverts in the applied set. Cycle shape (cheapest-first,
unrelated light features combined per the slice-6 method): c1 11 files/45
edits (9 forward-only features in ONE cycle — dec_operators +
dflare_kv_routing + cs_kv_probe + rat_plus_bridge + toast_tokenizer +
vocab_coreset + interval_pruner + parallel_probe + unit_distance);
c2 7/17 (bandit warm: ropd_rubric, sdpg_bandit, stepcode,
sense_composition, questbench, fpcg_selector + go — reqwest is a lib-level
dep only, the tests are offline analytics); c3 2/15 (bfcf_lfu_shard implies
bfcf_tree; speculative_generator default-on satisfies the cargo gate);
c4 3/10 (d2f_3sr_warm_start + flashar_consensus, dllm warm); c5 1/7
(shard_kv); c6 2/21 (bench_277's 5-feature gate reduced to 3 explicit —
temporal_deriv + collapse_aware_thinking are default-on; + causal head);
c7 6/15 (bomber family incl. bomber-agent + g_zero); 8a 1/30
(mech_attribution); 8b 2/31 (bomber-wasm); 8c 1/2 (percepta_compile);
c9 2/163 (the two ex-"compile-only giants"); c10 1/2+1 (the missed
maxsim straggler, caught by the final verification dry-run).

**The headline — the "compile-only giants" were misclassified by two
prior slices and heal like anything else**: bench_block_diagonal_goat (93)
+ bench_octopus_goat (70) carry `#[ignore = "pure measurement benchmark
(no assertions), slow in debug; run with --release --ignored"]` — the
attribute-value form a literal `#\[ignore\]` grep misses, which is how
they got labeled compile-only. A release `--ignored` run validates BOTH
in ~15 s total (9.2 s + 5.7 s) — the slice-6 bench_core_optimization
class. Standing rule confirmed again: the baseline run itself ("0 passed;
9 ignored") is the only reliable gate detector; grep on attribute FORMS
lies.

**Three pre-existing failures found at baseline, documented not fixed**
(the bench_104 precedent — test semantics are the author's call):
(1) bench_fixed_vs_procedural fails DETERMINISTICALLY (ratio 15.39 vs the
1.5× CV bound; identical pre/post + on re-run) — the CV-ratio statistic is
broken at mean≈0: fixed-map mean is +0.06 so CV explodes to 72.7 while the
actual StdDevs (fixed 4.51 vs procedural ~5.0) SATISFY the variance claim;
the file's own history shows prior assertion softening (a5a4278f).
(2) bench_064_futamura_evaluator::proof_futamura_specialized_has_fewer_
dimensions — specialized=300 dims vs universal=216 (expects REDUCTION);
dormant direction (percepta_compile not in full/default); ALSO
debug-unrunnable (15+ min for 3 transformer-eval tests → release 235 s —
the funcattn class; release is the practical baseline mode).
(3) bomber_wasm_bench is MODE-LOCKED: debug fails 3 WASM perf gates
(is_safe_action 198µs vs <10µs etc.), release 11/11 — the file's own
header documents `--release` (the ugc_g1b class; validated in release,
debug same-signature).

**Tooling lessons:** (1) rustfmt on bare test files needs `--edition 2024`
(let-chains in bench_stepcode_modelless.rs:644 — bare-file rustfmt defaults
to a pre-2024 edition and hard-fails; first recurrence in the sweep).
(2) CPU contention splits cycles: the combined bomber-wasm +
mech_attribution + percepta_compile compile timed out at 25 min under the
sibling's parallel load — split into 8a/8b/8c and each compiled fine
(mech_attribution was even warm already); heavy WASM/MILP feature sets get
their OWN cycle, backgrounded with nohup + log polling. (3) A sibling
rebased my 9 in-flight commits mid-session (origin/develop moved under me;
reflog shows the clean replay, c10 committed on top, push fast-forwarded) —
the shared-checkout reality; validations all ran post-rebase so the final
tree is what was validated. (4) `manual_is_multiple_of` is a rust-1.93
default-on lint with NO healer proposer — fixed manually at bench_maxsim_
rerank.rs:131 ((rng.next() % 4) == 0 → rng.next().is_multiple_of(4));
corpus-gap candidate (the Issue-060 drive-by class, now with a second
occurrence). (5) The vec-with-capacity array-literal divergence now has
FOUR recorded sites (2 prior + bench_120_renyi_efficiency:159 +
test_120_toast_renyi_goat:31 — the iterable is a multi-line array literal
with no name the emitter can reference for .len(); extending the proposer
to bind a temp first is the standing backlog item).

**Final state of the katgpt-rs root tests/ dir: 2 findings remain, both
the documented vec-with-capacity divergence class.**

**benches/ sweep landed 2026-08-18** (katgpt-rs `c70669f8`..`0eadbc54`, 10
commits rebased onto the sibling's TokenPath commit `62e01c1a`) — **the
root benches/ dir sweep is COMPLETE: 42 files / 406 edits + 4 manual
fixes** across 7
cycles (c1 default-compiled 14 files/132; c2 probe/sink/belief 6/48; c3
sudoku/halt/eigenbasis/ict/delta_mem 5/118; c4 cgsp/eventlog/drift/causal/
geometry 6/35; c5 quant 1/9; c6 flashmemory 1/18; c7 kimi family 9/46) +
1 residue-fix commit. **Benches are a different validation surface**:
harness=false `fn main()` binaries (validation class 9 — exit code +
normalized output diff, no test-result line). c1 got the full pre/post
protocol (14 baselines captured pre-heal; 12/14 normalized-identical, 2
timing-column-padding-only, ALL exit codes identical incl. the 2
pre-existing failures: fpcg G6 timing gate exit=1 under load + alg-prob
RLE cached-cumsum panic exit=101 — a REAL pre-existing correctness bug,
documented per bench_104 precedent). Gated cycles validated post-heal
(exit + verdict sanity + computation-untouched spot-checks) — justified
because the edits are output-identical-by-construction (format-arg
inlining: same spec, same values, positional→named mapping) +
compile-gated under each cycle's EXACT feature set.

**Two tooling discoveries that change future sweeps:** (1) **`--verify`
mode writes edits UNCONDITIONALLY** — the `--write` flag only gates the
non-verify path, so the initial "dry run" (`--fix --verify` WITHOUT
`--write`) landed the full 406-edit set to disk while reporting "fixed 0"
(a verify-mode counter quirk); worse, the default-features verify check
compiled gated benches EMPTY — the exact slice-6 hazard, caught here by
re-running explicit per-set `cargo check --benches --features F` for all
6 sets (all clean). Standing rule: NEVER trust `--fix --verify` without
`--write` as a dry run — it is not one. (2) **Residue scans must filter
by warning TEXT, not `--> path` alone** — the uninlined warning message
is "variables can be used directly", NOT the lint name, so the slice-7
residue-grep pattern (`^warning.*uninlined_format_args`) matched NOTHING
and produced a false "no residue"; a main-checkout scan was ALSO
corrupted mid-session by a sibling's broken uncommitted WIP (the kimi
set failed compiling lodestar.rs → no bench warnings emitted at all) —
the isolated-worktree re-scan is the only trustworthy surface when
siblings are active.

**Heal-created pedantic warnings (the Issue-027 class, 3 more sites):**
`.map(f).unwrap_or(false)` → `.map_or(false, f)` trips clippy 1.95's
`unnecessary_map_or` (suggests `is_some_and`) when the proposer's
provably-bool gate can't see through tuple-destructure closures
(salience ×2) or `matches!` macros (bench_011) — all 3 fixed manually;
the proposer gap (extend provably-bool to these shapes) is the standing
backlog. Also 1 missed-site fix: `format!` nested in an if-expression
argument to `println!` (procrustes:174 — the second-level macro-nesting
descent gap, same class riir-train slice 10 documented).

**Final state of katgpt-rs root benches/: 2 findings remain** — both the
named-width-arg divergence class (sudoku_speculate 813/814,
`println!("║ {:<width$} ║", hero, width = w)` — pre-existing, pedantic-only,
healer-silent by documented design). Sibling coordination: a live
TokenPath→TreePath naming WIP broke the shared checkout mid-sweep — all
c4-c7 validation moved to `git worktree add /tmp/heal8_wt` (the 9 kimi
benches ran there with `KIMI_K3_MODEL_DIR` pointing at the main repo's
1.58 GB model.safetensors — the worktrees lack data/ dirs); the final
push used the worktree-rebase-then-push dance (`git push origin
HEAD:develop` from a temp branch) because the shared checkout's dirty
sibling WIP blocked in-place rebase.

**katgpt-rs crates src/ slice 1 landed 2026-08-18** (katgpt-rs `23c3b7a9a`; the
log-family proposer fix riir-clippy below): **spectral + dec + kv + attn-match
src/ — 42 files / 515 edits (507 pass-1 + 8 pass-2), net −256 post-fmt**.
Fresh full-workspace dry-run sizing first: **429 files / 4115 edits** across
all katgpt-rs src surfaces (katgpt-core 2775/239 the giant; then spectral 266,
speculative 139, attn 130, dec 122, kv 118, attn-match 111, root-src 114/35,
transformer 63, percepta 49, types 42, + a ~340-edit small tail across 16
crates). **The log-family proposer root-cause fix shipped mid-slice** (found
as 2 live residue sites): the `uninlined_format_args` macro-family table
lacked the ENTIRE log family — `log::info/warn/error/debug/trace` (+
same-named `tracing` macros), template-first like `format!`; real clippy
fires on them (probe evidence: katgpt-spectral `outlier_guard.rs` `log::warn!`
+ `log::error!` sites). Added `"info"|"warn"|"error"|"debug"|"trace"` to the
First arm — last-segment matching covers the qualified production spelling;
the tracing `target:`/`parent:` named-arg forms abort via the
template-must-be-single-string-literal guard (`template_arg.toks.len() != 1`);
3 new fixtures (qualified log fires / tracing target: silent / bare debug!
fires + ident-first non-format shape silent). The extension also widens
`map_unwrap_or`'s macro-arg descent to log-macro args (same family table).
Validation: all configs count-identical pre/post — spectral default 85 + AF
261, dec default 225 + AF 251, kv AF 254, attn-match AF 146; clippy residue 0
in healed classes all 4 crates; rustfmt on the 34 fmt-clean-at-HEAD files
(8 carry pre-existing drift, 0 CRLF).

**katgpt-rs crates src/ slice 2 landed 2026-08-18 evening** (katgpt-rs
`3ffc71215` + `9be3bb751`; the map_or terminal-state proposer fix riir-clippy
below): **speculative + attn + transformer + percepta + types + ALL 16
small-tail crates src/ — 91 files / 411 edits, net −279 post-fmt. THE
KATGPT-RS MEMBER-CRATE src/ SURFACE IS NOW DONE** (only the katgpt-core
giant 2775/239 + root-src 114/35 remain). Slice composition (src-only
sizing; the prior whole-tree counts included tests/benches): speculative
12/120, attn 8/43, transformer 7/19, percepta 9/49, types 9/15, sleep 3/28,
claim 5/24, moka-wasm 4/11, canon 1/2, band 2/12, sense 2/11, backend 2/4,
deprecated 2/9, proof-cert 1/8, micro-belief 1/8, tokenizer 4/7, sparse 1/5,
ruliology 3/4, quant 1/4, pruners 4/7 (587-heal stragglers), forward 10/21.
**The map_unwrap_or bool-default terminal-state fix shipped mid-slice**
(found as 2 live heal-created residue sites in percepta): (1) `contains_key`
joined the `is_bool_method` allowlist (the percepta scheduler.rs:550
`|&op| k.contains_key(&op)` ref-pattern closure shape); (2) `matches!` macro
bodies recognized as bool via `Expr::Macro` path check (the graph/types
`|d| matches!(d.kind, ..)` shape); (3) the stale `true`-default SKIP replaced
with direct `is_none_or({mapper})` emission — probe-verified on clippy 1.93:
both `map_or(false, bool_f)` → `is_some_and` and `map_or(true, bool_f)` →
`is_none_or` are clippy's own one-step terminal states (both outputs
lint-clean; `is_none_or` stable since 1.82; the old skip rationale "no
single parse-only step matches" predates emitting the terminal form). The 2
percepta sites fixed to terminal forms manually (the proposer fires on
`map().unwrap_or()` chains, not on already-fused `map_or`). 3 new fixtures;
577 default / 640 fix_verify / 629 latent_retrieval lib tests; clippy 0.
**Pre-existing conditions found + documented (owner calls):** katgpt-attn
AF was a BROKEN GATE at HEAD (E0433 TreePath missing import in gdn2
`tree_forward.rs` tests — invisible under empty default features) — repaired
mechanically (`3ffc71215`, unblocking AF validation; attn AF 386/0);
katgpt-forward AF has 3 deterministic failures (d2f_verifier q-row sum
1.06130 / dflash kv_routing step 6 / dflash conditioned_valid_probs — top
dflash.rs commit is the Jul-20 SIMD renormalization, d2f_verifier's Aug-15;
validated same-signature 279/3 pre+post); katgpt-pruners AF flake:
`go::tournament player_type_creates_instances` uses UNSEEDED
`fastrand::Rng::new()` — nondeterministic (green 3006/0 run on record,
6/6 isolated; author should seed it). **Toolchain discovery (the Batch-49
trap re-hit):** the box default host is GNU + `E:/git/.cargo/config.toml`
injects MSVC-syntax `msvc_stl_shim` link args globally — GNU ld chokes on
`/LIBPATH:`; every riir-clippy build on this box needs
`RUSTUP_TOOLCHAIN=1.97.0-x86_64-pc-windows-msvc` (matches the pin's channel;
katgpt-rs has no pin → rustup default msvc, unaffected). **Disk-full
incident mid-slice** (E: hit 100%, os error 112 killed tokenizer AF build):
freed 33G by deleting the disposable `target/debug/incremental` caches
(katgpt-rs + riir-clippy) — safe by design, cost is recompile time only.
Validation: all configs count-identical pre/post (def + AF per crate;
backend default-only — `ane`/`gpu_inference` are macOS metal/coreml traps);
rustfmt on 60 fmt-clean-at-HEAD files (32 pre-existing drift, 0 CRLF).
Documented divergences: 3 named-arg-with-rename (pruners reflection.rs —
2nd occurrence of the class, still backlog), 2 array-literal-default
(forward step.rs `&[1.0]`/`&[]` — the exact `2bf2a44` guard class), 1
len_zero verify-only (percepta legacy/mod.rs).

Remaining sweep territory: katgpt-rs src/ — katgpt-core 2775/239 (the giant,
multi-slice) + root-src 114/35 (kimi_k3/dllm dirs are sibling-adjacent;
worktree fallback proven); + the benches/ + examples/ surfaces of the
member crates (the 4115 sizing includes them). The member-crate src/
surface is COMPLETE.

**Member-crate benches/examples + katgpt-core benches/examples slices
landed 2026-08-18 evening** (katgpt-rs `b2f3ea933` + `0377ad22c`):
(1) non-core member benches/examples — 6 files / 65 edits (canon/dec/
speculative/spectral benches + types example); the 2 backend examples
(ane/gpu_inference, 6 edits) are macOS-only metal/coreml — SKIPPED on this
box, left for M3. Validation modes discovered per-target: canon_goat +
spectral 475 are RELEASE-REQUIRED (debug fails perf gates 242µs>50µs /
1000µs budget; release 17.9µs / 5/5 PASS — validate in release); dec560 +
spec136 are debug-verdict benches (exit 0 with designed "G2 FAIL"
measured-verdict output — same-signature target); types test_autovec
output BIT-IDENTICAL ("result = -0.84619355"). One bench per command:
multiple `--bench` flags run only the LAST (cargo quirk, twice-observed).
(2) katgpt-core benches+examples — **71 files / 990 edits in ONE mega-union
cycle** (52 features, all CPU/in-tree; ~50 distinct gates gate-mapped from
Cargo.toml [[bench]]/[[example]] sections — katgpt-core has NO `default =
[...]` line so ALL gated benches need explicit features). Composition: 971
uninlined_format_args + 14 map_unwrap_or + 3 single_match_else. Validation:
19 binaries FULL pre/post (the 14 control-flow-edit files + 5 biggest
format-only) — 6 examples bit-identical (0 diff lines), bench_420 sweep
table BIT-IDENTICAL (its "Quality claim NOT confirmed" exit-1 is the
PoC's designed verdict), GOAT benches same-signature (debug latency-gate
FAILs: 1349ns>500, 3112ns≥200ns, 88765ns>10µs, 3383ns>1000ns — the
release-required class; latency numbers jitter, verdicts identical); 52
files post-only (exit + verdict sanity — justified by the output-identical-
by-construction format-arg class + compile-gated under the exact union;
17 examples all exit 0); bench_440 debug-unrunnable (800 agents × 500 steps
never finishes) — release run completes post-heal (its G1 MARGINAL verdict
is the bench's own honest output). rustfmt on 38 fmt-clean files (33 drift,
0 CRLF); net −329 lines.

**THE PROPOSER ROOT-CAUSE FIX the slice's compile gate caught** (riir-clippy
`17622d2`): the a081b49 bool-default terminal-state emission was
RECEIVER-TYPE-BLIND — it emitted `is_some_and` on a `std::env::var` Result
receiver (katgpt-core bench_440:456, E0599 + follow-on E0282 — `is_some_and`
is Option-only). Fix: `classify_receiver` with three-way ReceiverKind —
(1) a param-type stack (the Issue-031-H1 enclosing-fn-stack pattern:
visit_item_fn/visit_impl_item_fn push `HashMap<ident, Option|Result>` from
signature annotations; a plain-path receiver consults the innermost scope);
(2) shape allowlists last-match-wins over the receiver text — Option
(`.get(`, `.find(`, `.first(`, `.last(`, `.pop()`, `.next()`, `.ok()`) vs
Result (`env::var(`, `.parse(`, `.parse::<` — the turbofish form broke the
naive pattern and was caught by its own fixture); (3) Unknown → the
always-compiling `map_or` (never a terminal form that could E0599).
Probe-verified on real clippy 1.97: Result receivers get `is_ok_and`
directly (skipping the map_or intermediate); Result + true default is
clippy-SILENT (no stable `is_err_or`) so `map_or(true, f)` IS terminal
there. 5 new fixtures (env::var, parse+turbofish, Result param false/true,
unknown fallback); 582 default / 645 fix_verify / 634 latent_retrieval lib
tests, clippy 0. **Residue divergence classes recorded (3rd/4th
occurrences)**: 2× named-arg-with-rename (bench_337_tropical_perf —
`println!("{iters}", iters = ITERS)` rebinding) + 2× second-level macro
nesting (bench_367 — `format!("{}={:.4}", k, p)` inside println! args,
the descent parses but doesn't recurse into nested Expr::Macro args) —
both join the standing backlog (the probe proved the healer fires on the
bare shape; the nesting is the gap).

**Environment lessons (this box, 3rd session)**: (1) /tmp is UNRELIABLE
here — some heredoc-created dirs vanished mid-session (Windows temp
cleaner selects by age/handles; `rel440` + `probe_res` deleted while
`pre_base`/`post_run` persisted) — probe crates + anything load-bearing
goes under `target/` (gitignored, stable), never /tmp; (2) background
`nohup ... &` jobs DIE with the tool's shell teardown (process-group
kill — the "vanishing" rel440 log was this, not the temp cleaner) — long
builds must run foreground with generous timeout, or the next invocation
re-runs them; (3) the multi-root `.` ambiguity + `cd` restrictions:
`cargo --manifest-path <abs>` from a project root is the escape hatch for
out-of-tree probes; (4) mega-union builds eat ~14G target disk (debug
lib + 71 binaries + release core ≈ 22G total) — reclaim `target/*/incremental`
after each union cycle (14G recovered twice this session); (5) `cargo build
--benches --features F` SILENTLY SKIPS benches whose required-features
aren't satisfied by F — naming the target (`cargo test --bench X`) errors
instead; a missing feature in the union (latent_trajectory_geometry was
dropped during composition) is invisible until an explicit run — always
cross-check the built-vs-expected target list.

**katgpt-core src slice landed 2026-08-18 night** (katgpt-rs `941c0d896`):
**135 files / ~1270 edits** (1170 uninlined + ~60 control-flow + 13
semicolon), net −697 post-fmt. Fresh src-only sizing first (the old 2775/239
included tests+benches+examples, already healed): 132 files / 1245 edits.
Baselines warm from the mega-union: default lib **1897/0/6** + all-features
lib **3974/0/13**, both count-identical post-heal and post-fmt. Residue
scan (all-features -W over every healed class): **7 documented divergences
only** — 2× map_unwrap_or array-literal `&[]` guard (breakeven/fidelity,
mux/freeze_thaw — the `2bf2a44` class) + 5× comment-guard match refusals
(curator:86, simplex:160, loop_:741, in_memory:107, cache:97 — comments
between arms / comment-only None arms; clippy's own note says "you might
want to preserve the comments" — healer SAFER). 2 manual is_some_and
terminal fixes (multi_agent_path/flow.rs — Unknown-receiver `map_or(false,·)`
emissions where clippy's type info proves Option; the Issue-027 manual-fix
precedent). rustfmt on the 106 fmt-clean-at-HEAD files (29 pre-existing
drift scoped out).

**THE semicolon_if_nothing_returned DISCARDED-POSITION EXTENSION + THE
CFG-PROMOTION LESSON (riir-clippy `5875b81`)**: the slice's -W residue scan
found 9 semicolon sites the old proposer never claimed — probe matrix
(`target/probe_semi`, 3 rounds vs real clippy 1.93) established the ACTUAL
lint scope: fires on the semicolon-less tail of ANY syntactically-discarded
block (if/match arms incl. else-if chains, unsafe/bare blocks, loop bodies,
semicol'd block-statements) gated on the block being **MULTILINE** (the
"consistent formatting" rationale — single-line fn tails/arms/blocks stay
SILENT, incl. `fn main() { helper() }`; the old proposer over-fired there —
2 fixtures corrected; the corpus entry was already multiline so G6 stands).
Valued positions never fire (E0308 soundness: valued fn tails, `let x =
unsafe {…}` bindings). The one clippy quirk NOT replicated: single-line
`while` bodies fire in real clippy but stay silent here (documented
under-approximation, zero observed production instances).

**The headline hazard — CFG-PROMOTION**: the first extension draft recursed
into ALL grammar-discarded block-like statements — and broke compilation
with 5 E0308s + invisible M3 landmines. Root cause: `fn block_max(…) -> f32`
with THREE `#[cfg(target_arch)]` switched blocks (aarch64 / x86_64 /
generic) — at PARSE time the earlier cfg blocks are mid-block statements
(grammar-discarded), but sibling cfg elision PROMOTES the survivor to the
fn's valued tail; adding `;` inside its arm breaks the target that compiles
it (x86_64 caught by the compile gate; the aarch64 twin at temp.rs:671
compiled nowhere on this box — an M3-only landmine caught by analysis, not
by any local gate). Fix: `carries_cfg(e)` — a mid-block `#[cfg]`-attributed
block-like statement recurses ONLY when the enclosing tail is already
discarded (unit fns stay sound: the promoted tail is still discarded).
Parse-time discarded ≠ cfg-time discarded — the grammar argument for
mid-block statements is only valid for NON-cfg'd blocks. 4 new fixtures
(both cfg shapes + multiline firing shapes); 15 proposer tests; 591/0
default + 654/0 fix_verify+latent_retrieval lib; clippy 0. Also 4 clippy
warnings fixed in the morning's map_unwrap_or.rs (explicit_iter_loop,
items_after_statements ×2, its own map_unwrap_or → map_or terminal).

**katgpt-types simd semicolon follow-up landed same night** (katgpt-rs
`f3c03bd6c`): 4 files / 36 edits (activations/research/elementwise/dot) —
the -W scan's dependency-crate residue, all `semicolon_if_nothing_returned`
in target-arch-switched UNIT fns (inplace mutators — promoted tails stay
discarded; sound on every target, spot-checked). 130/0 default + 244/0
all-features count-identical; zero residue. The 36 = 2× the clippy-visible
count (the aarch64 twins are cfg'd out on this box — the extension fires on
them via the same soundness argument the cfg-guard validates).

**katgpt-rs root-src slice landed 2026-08-18 later session** (katgpt-rs
`0716c2909`): **53 files / 137 edits, net −111 — THE katgpt-rs src/ SWEEP
IS NOW FULLY COMPLETE** (root-src was the last unit). Root-src: 36 files /
117 edits (63 uninlined + 25 map_unwrap_or + 14 match_bool + 6 single_match
+ 5 vec-with-capacity + 2 semicolon + 1 hashmap-with-capacity + 1
manual_unwrap_or) + member-crate semicolon-class residue 17 edits (attn
dash_attn 13 — the gated surface the member slice never saw + 2 in a
NESTED fn, kv cs_kv_probe 2, percepta hull 2, core causal_id 1, pruners
12) + 4 manual terminal fixes (is_some_and ×2 incl
self_distilling_bandit:202 unnecessary_map_or, context-forced map_or ×2
twins step_paged:60 + forward step:156). **Gate-map correction:** dllm is
transitively DEFAULT via `dmax_spd = ["dllm", …]` (the parse-only cfg map
said gated — the reverse of the riir-train slice-5 lesson); inference_router
is UNGATED (the compression_drafter cfg above it gates a re-export, not the
mod); kimi_k3/tiktoken's real gate is `kimi_k3_loader`. 8 validation configs
ALL count-identical (root default 201 + 6-feature union 399, attn union 193,
forward 124, kv 24, percepta 84, core 1897, pruners 587/611).
**Compiler-proof of the array-literal guard:** `map_or(&[], f)` E0308s in
plain-let position (`expected &[_; 0], found &[f32]` — the default arg
unifies U FIRST, then the closure's concrete type conflicts) while the
twin at a context-forced call site compiles clean — tried live on both
step_paged twins; :129 reverted with an in-source comment, forward
step.rs:196 stays documented. The healer's refusal to convert array-literal
defaults is PROVABLY safer than clippy's own MachineApplicable autofix
there. **Residue terminal state (all classes):** the documented divergences
only — 3× array-literal `&[]` (breakeven/fidelity:249, step_paged:132,
forward step:196), named-arg-with-rename (reflection ×3, backlog),
percepta len_zero:115 (verify-only), + 2 OUT-OF-CLASS default lints left
for the owner (rule_extractor:180 + slod:745 if_same_then_else — semantic
duplication, not a heal class). Rustfmt on 17 fmt-clean files (35 drift
scoped out, 0 CRLF).

**THE for_each_fn NESTED-FN ROOT-CAUSE FIX (riir-clippy `5875b81`
follow-up, this session)**: the semicolon residue scan surfaced 2 live
sites in katgpt-attn block_topk.rs the proposer missed — root cause:
`for_each_fn` (the SHARED fn-enumeration helper, both domains) never
descended into fns declared inside fn bodies (`Stmt::Item(Item::Fn)` at
any block depth — the `fn argtopk_simd() { unsafe fn inner() { … } }`
unsafe-wrapper pattern). Fix: `nested_fns_in_block`/`nested_fns_in_expr`
recursion through every block-bearing statement (if/match/loop bodies,
semicol'd + bare blocks, unsafe blocks); `let x = { … }` initializers +
closure bodies are the documented under-approximation. Fixture
`nested_fn_body_fires` pins the production shape; 592/0 lib tests; the
extension benefits EVERY fn-scoped proposer in both domains.

**katgpt-rs root examples/ slice landed 2026-08-19 (katgpt-rs `5b2d42970`)** —
the "examples/: ALL COMPLETE" claim was WRONG in an instructive way: it
covered member-crate + core examples; the ROOT `examples/` dir (227 files)
was never swept. Found via the identical-named-arg follow-up grep
(`bomber_13_reflection_qa.rs` `x = x, y = y`), which the dry-run then
exploded into **147 files / ~1011 edit sites** (849 uninlined incl. the new
identical class + 38 map_unwrap_or + 28 match_bool + 26 single_match_else +
1 collapsible_else_if + ~73 rust_perf). Validation = ONE 80-feature
mega-union cycle (all 80 distinct required-features + 20 no-entry files):
compile gate 0 errors / 147 built (0 silently skipped, verified against
`target/debug/examples/*.exe` enumeration — remember example binaries land
in `debug/examples/`, not `debug/`); direct-.exe runs (no cargo overhead)
pre/post: **147/147 exit codes identical; 90 byte-identical; 57
timing-noise; 16 structural ALL exonerated** (6 TUI frame-timing + 10
nondeterministic via the same-binary post-x2 control — the bench_133
method extended: if the post binary's own two runs differ structurally,
pre-vs-post single-run diffs carry zero attribution power; 7 of the 10
were uninlined-only edits = output-identical by construction, incl.
core_02_raven's ⚡/🐢 RNG flip). Residue scan (union + `-W` on all 5 healed
classes): **0 findings in examples/** (110 findings elsewhere in the
workspace — lib src — confirm the scan itself ran). rustfmt on the 144
fmt-clean-at-HEAD files (0 CRLF, 3 drift scoped out).

**Two proposer guard gaps root-caused same session (riir-clippy
`d0d301a`)** — both caught by the union compile gate, both
compile-visible-only (no silent wrongness):
(1) **entry-vs-contains-insert E0502**: value/key referencing the receiver
(`vocab.entry(key).or_insert(vocab.len())` — the Entry's &mut outlives the
inner & borrow; the ORIGINAL contains_key+insert was legal because the
immutable borrows end before insert). Guard: word-boundary receiver-ident
check on key+val text (datrie_01_bench).
(2) **hashmap-with-capacity E0425**: the iterable's root local bound
BETWEEN the map creation and the loop (unrelated-statement skipping
sailed past its `let`) — `with_capacity(all_names.len())` emitted before
the declaration. Guard: `iterable_root_ident` (reduces `&xs`/
`xs.iter()`/field-chains to the root local; `self`/multi-segment paths are
never use-before-decl) + skip when any stmt in (new()..loop) (re)binds it
— covers first-bind E0425 + shadowing wrong-length (bomber_17_feedback_goat,
3 maps). 4 fixtures; 12/12 + 18/18 proposer, 607/607 lib, clippy 0.

**katgpt-rs member-crate tests/ slice landed 2026-08-19** (katgpt-rs
`f2265e092`) — **53 files / 787 edits (758 uninlined_format_args + 11
match_bool + 7 vec-with-capacity + 6 map_unwrap_or + 3 single_match_else +
2 range_plus_one), net −298 post-fmt; THE katgpt-rs CLIPPY SWEEP IS NOW
FULLY COMPLETE** (src/ root + all 30 members, root tests/benches/examples,
member benches/examples, member tests — every surface swept).

**Gate-map corrections (2 enumeration blind-spot classes):** (1) katgpt-core
carries a ~70-feature `default = [...]` — most [[test]] required-features
(conformal, karc_forecaster, mag_mining, tilr, clr_weighted, linking_fold,
best_belief, bom_sampling, committed_field_blend, velocity_field_ensemble...)
are DEFAULT-ON, so their tests build+run under plain `cargo test -p
katgpt-core` — the "45 entries ALL gated" prior intel conflated required-
features with opt-in. Empirical ground truth = the `--no-run` built-target
list (52 test binaries under default). (2) **The inner `#![cfg]` scan MUST be
whole-file** — 12 more files gate via `#![cfg(feature)]` sitting at lines
10-41 AFTER doc headers (kda_backward:16, mla_backward:11, moe_backward:14,
moe_g1:18, bench_455_qb:41, weaver_real:27, data_probe:10, heterogeneous:14,
bench_578_avx2:29 all()/target_arch, capacity:31, bench_453:14,
velocity_uq:36 all()); a `head -60` scan is UNSOUND both directions (missed
these AND the terminal ate earlier grep output silently — the "0 passed" run
is the only reliable detector, twice re-confirmed).

**8 validation cycles, all count-identical or same-signature:** A core
default 24 binaries (7m45s; karc_g1_dh18720 285s + set_attention 76s the
heavies; data_probe + heterogeneous ran 0 tests under default — empty-cfg
binaries, compile-only) — COUNT-IDENTICAL; B core 12-feature union 15
binaries (analytic_lattice, babel_codec, variable_rank_domain_expert,
selection_propagation, engram_privilege, causal_head_importance,
heat_kernel_trajectory, sleep_time_anticipation, karc_batched_matvec,
karc_householder_eig_par, sink_aware_attn,
velocity_field_ensemble_heterogeneous) — COUNT-IDENTICAL; C attn
kda_backward,mla_backward; D transformer moe_backward,transformer_moe; E
spectral quantile_balance_router,manifold_power_iter_router; F speculative
weaver_runtime; G types ternary_group_scale,plasma_path,sigmoid_margin (debug
+ release); H moka-wasm default (native_puct_winrate 807s — the 13-min
lesson in action).

**3 pre-existing failures documented (bench_104 precedent, all
same-signature pre/post):** (1) kda_backward_grad_check
`gradient_check_multitoken_all_params` — numeric grad-check FAIL at HEAD,
max relative error **0.036237 BIT-IDENTICAL pre/post** across 70 edits in
the file (strongest-possible evidence the heal left computation untouched);
(2) bench_583_scale_hoist_goat `g2_hoisted_vs_folded` — FAILS IN RELEASE TOO
(its documented mode) — genuine pre-existing break, not mode-lock;
(3) bench_578_avx2_goat `g2_avx2_vs_scalar_speedup` — debug-fail/release-pass
(mode-locked, documented `--release` in header; validated green in release
both sides). All 6 map_or emissions carry INT defaults (no
unnecessary_map_or risk); vec-with-capacity sites all have in-scope iterables
(the d0d301a guards held — ZERO compile-gate catches, the first 53-file heal
with no proposer bug surfaced). Residue 0 (fresh healer dry-run). rustfmt on
18 fmt-clean-at-HEAD files (35 pre-existing drift scoped out, 0 CRLF);
post-fmt spot-checks green (6p/4p/1p).

**katgpt-rs sweep TERMINAL state (2026-08-19):** every clippy-class surface
swept — root src/tests/benches/examples + all 30 member crates'
src/tests/benches/examples. Remaining known: the 2 out-of-class
if_same_then_else sites (owner call), the documented divergence backlog
(named-arg-with-rename ×5+, second-level macro nesting, array-literal
vec-with-capacity ×4, comment-guard match refusals — healer SAFER than
clippy), + M3-deferred macOS backend examples.

**Parallel-session collision on the SAME unit (katgpt-rs Issue 665 class, third
occurrence — yielded cleanly 2026-08-19):** a second idle session healed the
identical surface CONCURRENTLY (local chain `d9d53dc0`..`418337a2`, 7
slice commits, 51 files / 781 edits — avx2 skipped as x86-gated; preserved
on local branch `sweep-backup-0819` in the katgpt-rs clone). Their
`f2265e092` pushed first (53 files / 787 — they included avx2 + their
rustfmt pass = the 11-file delta). Adjudicated at push time: theirs is the
superset → mixed-reset to origin + adopted their content wholesale;
spot-verified green post-yield. **Two facts from the losing side worth
keeping:** (1) the two healer runs produced DIFFERENT output on 11 files
(≈ same edit count, minor shape differences) — the d0d301a/12a168e guard
landings between the two builds changed proposer output; "deterministic
healer" is build-pinned, not eternal — same-unit collisions can no longer
assume byte-identical output. (2) **kda_backward_grad_check
`gradient_check_multitoken_all_params` is FLAKY, not deterministically
broken**: the winning session recorded it FAILING pre+post (max rel err
0.036 bit-identical), while the losing session's baseline AND post-heal
runs both PASSED 7/0 under the identical feature set — a numeric
tolerance sitting near the machine-state boundary. Cite it as flaky, not
broken-at-HEAD. (Losing side's process note: multi-binary `cargo test`
attribution — verify per-file `#[test]`/`#[ignore]` counts BEFORE judging
a "hang"; the moka-wasm baseline lines were misattributed (native_puct's
2 slow self-play tests were the 1117s runner, NOT wasmi; cost 3 timeouts +
a wrong lock-contention theory before the per-file grep resolved it).)

**Issue 031 (2026-08-17, RESOLVED same day + removed per noise-reduction
rule) — 3 healer defects from the katgpt-pruners 587-edit heal
validation (katgpt-rs `4b6675dc5`).** **H1 (CRITICAL)** — the
`comparison_to_empty` proposer rewrote `self.len() == 0` →
`self.is_empty()` INSIDE `fn is_empty` (3 sites: decision_explainer,
bfcp_region_cache, skill_catalog) → infinite recursion →
`STATUS_STACK_OVERFLOW` at test runtime — the first false-positive class
that produces a RUNTIME CRASH, structurally invisible to the `--verify`
compile gate (it compiles fine; only tests catch it; real clippy is
silent on the shape). Fix (`94cecbd`): enclosing-fn stack on the visitor
(`visit_item_fn`/`visit_impl_item_fn`/`visit_trait_item_fn`) + refuse
when receiver text is exactly `self` AND innermost fn is `is_empty`
(both forms: `len()==0` and `== ""`); non-self receivers inside
is_empty + self receivers elsewhere still fire (positive-control
fixtures); `clean_spans_fire_no_lint` extended with the exact production
shape. **H2** — `--domain` never scoped the `--fix` path: the proposer
loops ran BOTH `FIXABLE_RULES` (rust_perf) + `FIXABLE_LINTS` (clippy)
unconditionally (vec-with-capacity edits landed under `--domain clippy`).
Fix (`4a72c79`): `fix_domain_scopes()` — auto/None keeps both; explicit
domain runs only its own proposers; kernel runs neither; 4 bin tests
(`cargo test --bins` under the bin's required features). **H3** — the
verify baseline is default-features `cargo check`, so feature-gated
modules are NOT compile-gated (2 breaks shipped past verify: u8-const
`with_capacity` E0308 in monopoly + E0631 fn-path coercion in
roaring_membership — the exact Bench 031 classes escaping via module
gating). Fix (`f78dceb`): `manifest_nondefault_feature_count()` text
scan (multi-line default lists accumulate); verify warns + suggests
`--verify-args "--all-features"` when an owning package declares
non-default features and the caller passed no override — a warning, not
a default flip, because broken all-features sets exist (katgpt-dec `-D
warnings`). Gates: 519 default / 582 fix_verify / 4 bin tests / clippy 0
lib+bin under latent_retrieval,fix_verify. The katgpt-rs heal itself:
single_match_else 49→13 (remainder = documented deliberate skips), 8
fixable lints at 0, 3006/3006 lib tests.

**Plan 014 + Issue 032 (2026-08-19, both RESOLVED same day; issue file
removed per noise-reduction rule).** **Plan 014 — clippy-bench** (commit
`17e3d8b`): the frozen real-code healing bench that can falsify "we're
getting better" — 34 frozen fixtures (7 real pre-sweep mined + 27
authored-composite T3.5.5 multi-lint spans + clean/decline), scored
against real `cargo clippy` before/after on 4 axes (heal_rate,
created_rate=false positives, parse_safety, decline_correctness) + the
persist miss map; append-only `.benchmarks/score_history.jsonl` + a
zero-dep byte-stable SVG curve embedded in README; run via `cargo run
--features score_bench --bin cargo-heal -- --score-bench`. The
denominator never grows with the corpus — the fake-coverage failure mode
is structurally closed. Baseline run #1: heal 84.6% · created 3 (5.8%)
· safety 92.9% · decline 100%. **Issue 032 — run #1 caught a REAL
proposer bug on its first execution** (exactly what the bench exists
for): the `needless_range_loop` rewrite emitted its header + body-slot
substitutions as SEPARATE edits, so a foreign same-pass fix overlapping
one slot (`identity_op` on `hits[i] * 1` — same start, longer end) was
dropped by `apply_edits_traced`'s outer-first overlap selection while
the header edits survived → `for __elem in hits { hits[i] }` (dangling
`i`, non-compiling). Fix (`86ba7f1`): the rewrite is now ONE atomic
whole-loop edit (replacement = the loop's original text with all
substitutions spliced) — consistent with every other multi-site
proposer (`single_match_else`, `manual_map`, `map_unwrap_or` rewrite
the whole construct in one edit); a nested foreign edit is dropped
instead and re-fires on the next fixpoint pass (the documented nesting
semantics). **Run #2: heal 90.4% (+5.8pp) · safety 100% — the first
upward step on the curve.** 3 new tests (atomicity + both frozen
fixtures through the production fixpoint shape); 19/19 module, 623
default / 707 score_bench lib tests. Standing workflow: run
`--score-bench` after every proposer/corpus change so the history
accumulates; Phases 2 (perf/format axes) + 3 (real-fraction growth:
statement-wrap miner, stub manifests, crates.io source, cold/warm
memory lines) defer-marked in the plan.

**Issue 072 (2026-09-06, CLOSED + removed per noise-reduction rule; Plans 091
+ 092, [Bench 069](.benchmarks/069_issue072_t5_t6_generations.md)).** The
Frontier-evidence Challenger (J-Zero
modelless fusion, arXiv:2608.26582 M1/M2/M3/M6) — attacks T39/Research
135's measured starvation (98.5% of candidates unseen; "the bottleneck
is evidence volume, not selection math") by densifying pools with
CONSTRUCTION-labeled preference pairs: labels from the generator's
intent, never from any evaluator output. **PoC (Plan 091, exit criterion
MET 2-of-3):** `src/frontier_pairs.rs`
(200 resolution pairs derived from `corpus_gen` — good = the case's own
seed/planted truth, bad = a deterministic hash-rotated cross-code partner
repair; strategy players mapped to playbook ids where the doc-verified
inverse exists, `gen:{CODE}:{inverse}` elsewhere) + `PairRowDoc` in a
disjoint `pair:` store namespace (soft-judge ONLY — never EvidenceTier,
asserted on every outcome surface) + the T2 non-circularity fence
(`tests/frontier_fence_e2e.rs`, `#[ignore]`d: **RAN 20/20 agree (100.0%)
vs real rustc** — good arm compiles clean AND the broken form fails with
exactly `expect_code`) + the Zermelo Bradley–Terry ML fit (`elo.rs::
bradley_terry_elo`: order-free, bit-deterministic BTreeMap iteration,
mapped onto the standard base-1200 curve via `elo_from_lambda`; 7 gates
incl. the fixed-point-expected-score contract) + `src/dispersion.rs`
(outcome-class Shannon entropy over a fixed-size class table, the
`regime_of` control law — informative/empty/regenerate/saturated — and
`frontier_order`; both canaries pinned) + the T7 standing telemetry (the
`stats_report` dispersion line + vacuous-window flag).
**Measured (tests/frontier_poc_ab.rs, 4/4):** (a) unseen-candidate
fraction **0.786 → 0.429** pair-fed vs outcome-fed (outcome covers 3,
pairs cover 8 of the 14-player universe; 200 pairs, zero cargo) —
exit axis (a) PASS; (b) G8 PROXY — informative mean rank 2.00 (FIFO) →
1.00 (frontier) — PASS at the proxy; (c) retrieval floors GREEN — PASS.
**T5/T6 (Plan 092, Bench 069):** `src/score_bench/generations.rs` — the M4
k-ladder CONSTRUCTION-FIRST: atoms = the seed corpus's own parse-standalone
spans (38, one per vocabulary lint), composites wrap each atom in its own
module (zero interference by construction), deterministic stride selection.
Freeze vs real clippy: **58/58 verified composites with union-of-atoms gold
EXACT** (8 exclusions, all cascading from 2 bad atoms — `needless_match`
also fires `manual_map`, `uninlined_format_args` also fires
`useless_format`); per-k solve rates **100.0% FLAT at every k** → the
pre-registered **branch (B)**: a corpus-atom ladder cannot be harder than
proposers trained on that corpus. `report.rs`: `detect_saturation`
(WINDOW=5, EPSILON=0.005 strict, per-generation) reads the live history
**gen-1 SATURATED (marginal +0.0000 < 0.005 over 5 runs)**;
`--generation 2` runs the ladder behind the non-vacuity admission gate and
**honestly REFUSED (v2 1.0000 ≥ v1 0.9623, exit 1, nothing appended)** —
per T6's own spec. `HistoryRecord.generation` (serde default 1 — v1 rows
never renumbered; `ScoreRunRow` mirrors, schema-identity test extended);
per-generation SVGs; the saturation line after every bench run;
`--misses` (the frontier-ordered authoring queue: per-code pools =
store kept/reverted + Missed × queue occurrences, ordered by
`frontier_order`, conversion column = the real-G8 measure; `--misses
--report` = the era-split conversion ledger). In-queue
insight pinned in-test: a fully-saturated pool can never occur in the
queue (every entry IS a miss) — all-kept store history reads as LOW-entropy
informative. **Reopen conditions:** (1) difficulty axis for a generation 3
(persist-shape stratification / real-repo interference spans /
overlap-edit chains — Bench 069's ranked list; a harder generation
re-enters through the same freeze discipline, never a silent v2 rewrite);
(2) the real-G8 A/B — author N templates from the frontier-top of
`--misses-report`, compare Certified-conversion vs the historical FIFO
baseline; reopens when the queue has ≥3 entries + ≥1 authored template.
**REOPEN (1) EXECUTED same day (Bench 070): generation 3 = the
persist-stratified ladder — 4 authored persist-shape atoms (array-literal
map_or, comment-guarded match + hoist, the useless_format String terminal)
stratifying the 38 corpus atoms; the stratification gate (persist atoms
resolve 0) FALSIFIED two of five authored atoms on the FIRST freeze run
(the P3 rename rewrite had closed one class; clippy 1.98's new
`format_in_format_args` moved another off-vocabulary) and the replacements
landed; frozen 72 specs, **ADMITTED** (gen-3 87.5% < v1 96.2%), run #47
appended + `score_history_v3.svg`. En-route CLI find: the collision-salvage
merge had left `--generation` parsing inside the `--limit` arm (the flag
could not parse at HEAD; `--misses --limit` errored on the never-set
`misses_limit`) + duplicated `--score-report`/`--memory-stats` arms — all
repaired and live-verified. Next axes: the rustc-errors-side ladder or
vocabulary extension to rust_perf rules (both in Bench 070 §What remains);
v3's rate is a permanent DESIGN floor, not a frontier.
**REOPEN (1) EXECUTED AGAIN 2026-09-06 (Bench 071, Plan 093): generation 4 =
the rustc-errors-side ladder (Bench 069 candidate #4, the removed Plan-091
duplicate's salvaged T2/T3) — composite `corpus_gen` cases at k∈{1..4}
measured through the `--fix-compile` driver: a different oracle (rustc
coded-error reduction, exact) and a different repair domain; the label
shape reused `(spec_id, codes)` so the whole freeze engine ran UNCHANGED.
Frozen 76/76 (zero exclusions, zero drift), **ADMITTED** (gen-4 54.3% < v1
96.2%), deterministic ×2, rows #48/#49 + `score_history_v4.svg`. The
territory split is the headline: uncontested 100% (60/60 — the templates'
home turf at every width) vs contested 20% (only the E0505 template; the
delegate-only E0596/E0382/E0507/E0373 families are the structural persist
rows — the floor moves only when a template family ships, a floor-harness
gate). **Two instrument catches en route, both loud-by-design:** (1) the
freeze's first run caught `cargo check --all-targets` DOUBLE-compiling the
lib (every diagnostic twice) — fixed by `(code, line)` dedupe (the
driver's own `dedupe_by_site` class); (2) the bench's first live run
measured an INCOHERENT 45.7% (uncontested 0/60 — a code cannot reduce
without a template) — the scoring predicate in
`composite::finalize_attribution` was INVERTED (presence counted as
reduced; an empty final set counted as all-unproven); the instrument was
repaired and the garbage row REMOVED from the JSONL before the corrected
run became the series' first row. Lesson: a freeze label fence cannot see
a scoring-semantics defect — the first live bench run is itself a gate,
and an incoherent GREEN is a finding. The M5 admission gate + the
append/SVG/saturation tail are now ONE shared implementation exercised by
both runners (the v2 refusal re-verified byte-identical through it).
**En-route catches:** (1) the pair row initially carried a wall-clock
`created_unix` — the idempotence test flaked when two imports crossed a
second boundary (the full-suite run caught it; the field is REMOVED —
the row is construction-deterministic, dating lives in git/store
history); (2) `elo_table()` reads PERSISTED ratings — the A/B's
outcome-fed baseline needs an explicit `recompute_elo()` first (a
0-coverage baseline reads as a trivially-won A/B; the assert
`recompute.matches > 0` now pins the arm is non-vacuous); (3) a 1-0
outcome pool is SATURATED by the control law (zero-entropy-all-pass),
not informative — a draft fixture mislabeled it and the assert correctly
refused, the label now documented in the fixture; (4) the `.plans` 090
dual allocation (this issue's PoC plan vs Batch 117's ds4 plan, landed the
same window) resolved by citation weight — batch 117 keeps 090 (baked into
5 corpus concept strings + the batch gate), the PoC plan renumbered 091
(`b8d30e73`); (5) a FULL same-unit collision with a concurrent session on
T5/T6 (their `.plans/091_issue072_t5_t6_*` plan + T1.1 queue listing + T4
detector + T1.3 conversion ledger, three commits on origin mid-unit):
adjudicated per push-wins + superset — this unit's freeze/ratchet/admission
is the survivor, their queue listing + detector variant superseded, their
plan file removed as the duplicate, **their T1.3 conversion ledger KEPT as
complementary** (the era-split G8 instrument: `WIRED_STRATEGIES` baseline /
`FRONTIER_LANDINGS` rows / `WIRING_COMMIT` window / the
inventory-equality ratchet test); the unified CLI surface is their
`--misses [--limit]` (this unit's frontier table) + `--misses --report`
(their ledger). Full adjudication: Plan 092 §collision + Bench 069.
**REOPEN (1) EXECUTED A THIRD TIME 2026-09-06 (Bench 072, Plan 094): generation
5 = the rust_perf vocabulary ladder (Bench 070 candidate #1b) — the FIXABLE
perf vocabulary (11 rules) as 14 atoms (10 corpus addressable + 4 persist:
`persist_ref_array_capacity` = the surviving REFERENCE-array-literal vec-guard
class, `persist_coldstart_fxhashmap` = the Gate-0 no-rustc_hash-import class
that persists in PRODUCTION, `persist_caller_gate_str` = the static-str caller
gate, + the corpus fxhashmap atom persist-by-context). The oracle is a
bench-owned set of 11 per-rule MECHANICAL SHAPE DETECTORS (pure textual, never
a proposer call; compound polarity for shrink-to-fit because the vwc
exact-capacity fix also resolves its hazard) — rust_perf rules have NO external
detector, so independence is from the HEAL LANE, pre-registered as the
domain-shaped limit. Executed as a SEPARATE generation via the gen-4 pattern —
NOT a v1 re-baseline (mutating `vocabulary()` would corrupt the v1 series that
anchors admission + saturation). Frozen 28/28 (zero exclusions, zero drift) in
**0.05 s — the first CARGO-LESS freeze** (G2 discrimination + G3 stratification
run as always-on module tests). **ADMITTED at 69.2%** (36/52 members < v1
96.2%), deterministic ×3, rows #50-#52 + `score_history_v5.svg`. Territory
split matches the projection exactly: addressable 100% (36/36) vs persist 0%
(0/16); persist map fxhashmap ×8 / static-str ×4 / vec-with-capacity ×4. One
pre-registered projection FALSIFIED by measurement (the coldstart atom took a
`with_capacity(pairs.len())` cross-heal — hashmap-with-capacity is deliberately
permissive for param-bound bare-path roots, unlike VWC's stricter guard; the
persist verdict held because the fully-qualified return type keeps
`std::collections::HashMap` in the region). The persist frontier is now
MEASURED and pinned: a proposer extension that closes any class moves the
gen-5 rate and the ratchet sees it.
**REOPEN (1) EXECUTED A FOURTH TIME 2026-09-06 (Bench 073, Plan 095): generation
6 = the real-repo shape ladder (Bench 069 candidate #2, SALVAGED) — candidate
#2's premise (organic multi-lint spans minable at the bare-stub filter) was
FALSIFIED by mining across three extraction granularities (1,247 lone fns → 0
multi; adjacency runs → 1 same-class pair; file-header carves over 8 sweeps →
4) — the binding constraint is the v1 scope limit: ~90% of real fns reference
crate-internal types and never compile standalone, and the compilable crust is
lint-free (ndb 80 compiled / 0 lints; game-sdk HEAD 17/0). The SALVAGE mechanized
v1's own provenance note ("trait scaffolding added, lint spans verbatim") as the
FILE-HEADER CARVE (each candidate ships with its file's own use+type+const
context, tiered +macros+impls then +sibling fns), landing 17 real spans (13
single-lint + 4 organic multi, 6 classes, 4 repos) mined at the PARENT of each
repo's first heal sweep. Freeze: 17/17 verified at the BENCH instrument
(single-snippet), 0 exclusions — batch and single instruments agree on every
vocabulary label — drift PASS. **ADMITTED: gen-6 95.2% < v1 96.2%, deterministic
×3 + a 4th byte-stability run after a delimiter-only re-land.** Territory:
single 13/13 = 100% (real single-lint shapes solve at parity — generalization
measured, the "single < v1" prediction REFUTED); multi 7/8 = 87.5% — the one
miss is `single_match_else` in `gen6-transposition-impl1-000`, root-caused by
manual probe as the Issue-028 interior-comment guard (a DESIGN refusal re-found
organically — the exact thing a difficulty axis is for). Standing: the multi
rung is anecdote-sized (4 spans) until v1-Phase-3-class per-fixture manifests
land (the next rung = per-fixture path-dep manifests compiling the sibling at
the mining sha — owner-gated on the series justifying it). New tool:
`examples/score_bench_mine_v6.rs` (batch-oracle discovery + tiered carve +
census). CLI: `--generation 6` + `score_history_v6.svg`. Record: Plan 095
(§1a falsification table + §1b salvage) + Bench 073.
**REOPEN (1) EXECUTED A FIFTH TIME — the LAST axis — 2026-09-06 (Bench 074,
Plan 096): generation 7 = the overlap-edit-chain ladder (Bench 069 candidate
#3, manufactured maximal overlap: composites whose proposers' edits overlap,
exercising the fixpoint's drop-and-refire semantics, including one spec
engineered past `MAX_FIX_PASSES = 4`). The ladder dimension is CHAIN DEPTH
(k = expected passes; rungs 1–5, k=5 = capped). 15 authored specs in four
territories — flat (disjoint control) / chain / capped / refused. Probe
(T1, pre-freeze) confirmed the mechanics and cost two atoms (question_mark
has no match-form support; iter_next_loop's edit is header-only, not
whole-loop atomic) + mined the **capacity-hint nested-block blind spot**
(vec-with-capacity and hashmap-with-capacity are SILENT when the collection
sits inside an `if let`/match-arm block — intake). First freeze run excluded
2 (probe-gap: the probe validates parse, the compile fence is the freeze's
own instrument — E0308 return-type; E0425 cross-arm binding), both repaired;
landed **15/15, 0 standing exclusions, drift ×2**. The fence also found the
**created-chain subtype**: on d2-nrl-format the outer rewrite CREATES the
inner lint's firing site (`uninlined_format_args` cannot inline an index
expression; the rewrite-created bare ident `__elem` is exactly what it fires
on) — label = the outer member, chain depth still 2. **ADMITTED: 78.26% < v1
96.23%**, deterministic ×3 (rows #57–#59 + score_history_v7.svg). Depth
gradient CONFIRMED: flat 100% · chain 86.7% · capped 33.3% · refused 0% —
the drop-and-refire semantics fully absorb overlap at depth ≤ 4 (depths 3–4
measure 100%); difficulty lives ENTIRELY past the cap and in the refusal.
THREE measured findings, all pre-existing, all intake-not-fixed (Plan 096
non-goal): (1) **the needless_range_loop rewrite produces NON-COMPILING
output when a slot sits in a comparison context** (`if __elem > 2` is E0308 —
`&u32` vs integer needs a deref; `n += __elem` compiles because std ships
`AddAssign<&u32>`) — no fixture had ever covered the class (the corpus atom
is `total += v[i]` where `Add` exists); production `--verify` auto-reverts
it, the bench measures unverified by design and caught it (parse_safety
93.3%); (2) the `__elem` collision guard turns nested needless_range_loop
into a hard persist (refused 0%) — fresh-binding uniquing is the fix-space;
(3) the fix cascade EXPOSES deeper lints outside the fixable vocabulary
(created 3: map_identity + single_option_map from `x.map(|v| v)`,
suboptimal_flops from distance-sq's `r * r`). The Bench-069 difficulty-axis
list is now FULLY EXECUTED (#1b→5, #2→6, #3→7, #4→4); real-G8 A/B stays
data-gated. CLI: `--generation 7` + `score_history_v7.svg`. Record: Plan 096
(T1 probe addendum + freeze-fence amendments) + Bench 074.

**Issue 073 (2026-09-06→07, CLOSED — T1/T2 landed + T3 measured-NEGATIVE;
removed per noise-reduction rule; XBridge arXiv:2608.11676 transfer,
Research 535 katgpt-rs).** T1 anchor-primacy pins + T2 entity-perturbation
fixtures landed 2026-09-06 (`085a5e8`): all four fixer prompt builders carry
identity fields verbatim; the T2 twin construction (same-kind fn rename +
rule-label swap, mutation-verified) discriminates an anchor-primacy builder
from a summarizing one in both directions. **T3 (the Bt8 anchor-ledger A/B)
ran 2026-09-07 — FAIL under the pre-registered bar, twice**
([Bench 076](.benchmarks/076_issue073_bt8_anchor_ab.md)): follows-swap arm A
3/12 vs arm Bt8 2/12 (delta −1) at max_new 256, then 4/12 vs 2/12 (delta −2)
at max_new 1024 after the truncation confound was removed; strict-keep
no-regression PASS both runs. Failure mode is instrument-structural, not
arm quality: glm-5.3-flash under the trained instruction answers in
MINIMAL-EDIT-LINES style (`fn_sig_line` present in 0/24 run-2 generations),
so the identity-bearing signature line — where the renamed symbol lives —
never crosses the output for either arm, and the ≥10/12 bar presumes
signature-bearing outputs. **Zero prior-regression events in 48 calls** (the
model never once wrote the original symbol into a fix): the XBridge
identity-collapse mode did not manifest on this corpus, and at the
applied-module level the merge contract preserves the twin's identity lines
trivially. Verdict: `prompt_anchor_ledger` stays the landed opt-in builder
with its builder-level pins; nothing promotes; the A/B is mechanically
re-runnable (`fixer_regate --arm BT8 --max-new 1024 --gens <path>`, ~$0.005)
if a future fixer arm emits full-fn rewrites. Runner + helpers landed this
commit: `fixer_regate --arm BT8|BT8DRY` (BT8DRY = the offline instrument
gate, 12/12 pairs PINNED before any live call), `perturb_symbol_ident` +
`ident_mentioned` promoted to `cf_workers_ai` (one implementation shared by
the pins and the runner). T4 (LAM cross-tokenizer mapper) stays deferred
trigger-gated (zero cross-family drafter/verifier pairs exist; reopen on
consumer ask).

**Issue 074 (2026-09-06, CLOSED — T1–T3 landed (`0b1e9c9` render-gate hardening
+ the eager/lazy class, `339a5b4` the deferred gate-red fix unit, `d67f398`
record); removed per noise-reduction rule.** The trust channels no longer
stop at the rustc_errors domain: `rule_context_note` per-domain tables
(clippy_lints: manual_map control-flow capture, needless_range_loop borrow
shape, question_mark short-circuit scope, map_unwrap_or inference shape,
unnecessary_lazy_evaluations eager-on-every-path; rust_perf: static-str +
bool-to-bitpack signature-visible) render `⚠ semantic change: {note}` in the
general `--fix` review through the REAL formatter (`format_fix_review` split
from the printer so the byte pin executes what the CLI runs; notes truncate
to first line; mechanical classes byte-identical, pinned). Table ↔ FIXABLE
lockstep asserted. Score-bench runs #58/#59: rates bit-identical, the trust
axis (created 1.9%) unmoved. **T4 (`--suggest` instantiated display) stays
DEFERRED with its DRY constraint, reopen on consumer ask:** the suggestion
text must be the proposer's own output (the instantiation re-ranker already
runs propose_fix visitors under RerankMode::Structural), never a
presentation-layer hand binder — a parallel binder can emit a
plausible-but-noncompiling instantiation, the exact "suggestion that lies"
failure the verbatim-corpus-text design prevents. No consumer ask exists
today (zero `--suggest` display complaints since landing).

**Issue 036 (2026-08-23, RESOLVED-NEGATIVE + removed per noise-reduction
rule; harness `src/draft/jury_disagreement.rs` + `tests/jury_disagreement_poc.rs`
kept as the reproducible artifact).** The Jagged Judges (arXiv:2608.12645 §6.2)
transfer — retrieval-pool contention as a fix-instability predictor, gating
expensive verification onto contested spans. T1+T2+T3 measured on the full
fixture population (129 spans: 61 clippy + 28 composite + 40 perf):
`resolved=72 nofix=49 fixture_broken=8 compile_fail=0 lint_unresolved=0` —
**ZERO instability events among the 72 verdict-bearing spans**, so ρ is
uncomputable (no variance in Y) and G1/G2 fail vacuously. Stronger than the
ρ≈0 refutation the issue pre-authorized: the instability REGIME the paper's
predictor targets does not exist on the proposers' home fixtures at all —
the deterministic stack's verification failures live on real repo spans
with type context (Bench 031: ~14% first-pass compile-gate reverts), not on
minimal fixtures. Two measurement findings en route: (a) `pool_spread` is
DEAD on this corpus — every span's 12-deep pool contains 12 distinct rules
(the multi-scale line-chunk flooding the signal hypothesized never survives
`rank_into`); (b) the join MUST baseline-check the ORIGINAL span first
(`FixtureBroken` exclusion) — the first run's 8 "unstable" rows were all
Issue-022-class broken-original fixtures fabricating CompileFail verdicts
exactly the way a fix cannot be blamed for a span that never compiled.
Reopen condition (recorded, not scheduled): instability events must first be
shown to exist at a non-trivial rate on REAL-file spans via the `fix_verify`
compile gate — a different harness, not this one. Validation at close:
feature lib 745/0 · default lib 659/0 · retrieval floors 12/12 (G3) ·
clippy 0 in the feature state.

**Issue 039 (2026-08-24→25, RESOLVED — T5 verdict ReportOnly; removed per
noise-reduction rule; harness `tests/t39_elo_selection_ab.rs` kept as the
reproducible artifact).** The ELO-rated rules + evidence-grounded ruliology
issue over the unified trajectory store — T1 substrate decision (katgpt-rs
Issue 686 filed; adapt-locally standard Elo K=32/base-1200 with citation) +
Pod-row reservation; T2 the `src/elo.rs` math module +
`TrajStore::recompute_elo` batch pass + `--recompute-elo` CLI + the 3-store
backfill (headline: `uninlined_format_args` 1625.3 70W/0L; spans that beat
rules rate UP — span difficulty emerges); T3 the ruliology evidence axis
(LEXICOGRAPHIC TIE-BREAK ONLY — payoff primary, evidence reorders exactly
the parse-equal ties; `search_with_evidence` et al., unrated path
bit-identical); T4 the `EloTable::from_ratings` load constructor +
`TrajStore::elo_table()` bridge (prefers the non-base rating per player;
never recompute) + `self_evolve_audit.sh` §1b leaderboard (top rules,
draws, pending-recompute hint, Withdrawn excluded, toughest shapes) with a
live loop-closure proof (a NEW riir-game-sdk store surfaced + backfilled by
the hint's own command). **T5 — the selection challenge gate: ELO does NOT
replace BetaPosterior** ([Bench 047](.benchmarks/047_elo_selection_challenge_gate.md)
+ [Research 135](.research/135_elo_selection_gate.md)): at REAL pool sparsity
(measured from the stores — 98.5% of candidates unseen, 1.3% one-observation;
t26's assumed 0..=3 was ~40× denser) with paired common-random-numbers
McNemar + the promotion rule ENCODED (`promotion_gate`: strict win = discovery
strictly greater + z > +1.96 + no regret regression, both evo-noise cells):
EloStatic (the exact batch-loaded store form) significantly WORSE (z = −8.94
— frozen ratings give up within-trial learning, which is nearly all the
signal at this sparsity); EloOnline a microscopic real win (+0.0004,
z = +3.63 @ 0.6) but NS @ 0.2; the dense rule-aggregated production form
regime-dependent (+4.09 harsh-evo, NS negative mild-evo). Three findings
recorded: starved pools make selection differences ≤0.04pp (the Issue 026
doctrine CONFIRMED on store-measured numbers — the bottleneck is evidence
volume, not selection math); a frozen rating table is strictly worse than
count-updating at thin evidence (any future rating-based selection must be
online); and the clippy score-bench CANNOT measure selection at all
(`heal_fixpoint` bypasses `select_best_candidate` — selection gates get
their own harness). Verdict pinned by `t39_recorded_verdict_is_report_only`
+ a LIVE gate assertion re-deriving it every run. Bonus: BetaPosterior's
T9.3 promotion survives the real-sparsity re-test (margin over LinearBlend
holds: 0.9185 vs 0.9013 / 0.9399 vs 0.9190). Reopen triggers: store
densification (median candidate ≥2 observations) or evo-axis degradation.
**katgpt-rs Issue 686 CLOSED 2026-08-25** (Bench 675): the rating math now
lives in `katgpt_core::rating` (DEFAULT-ON there) — this repo's `src/elo.rs`
delegates to the f32 twins (`expected_f32`/`update_f32` + the `STANDARD_*`
consts), bit-identical, persisted store ratings do not drift (675/753/707
lib counts unchanged). Substrate-first found a FOURTH copy the issue never
listed (riir-games ruliology `ParadigmRanking` — the win/loss/draw form,
which set `update_scored`'s API shape) — all four consumers delegate now.

**Run #3 (fix `c8d0d34`, same day): heal 98.1% · created 1 (1.9%) — the
run-#2 miss map triaged to root causes.** Every persist + created was
probed against real clippy 1.97 BEFORE any code changed (per-fixture
detail ships as the `#[ignore]`d `per_fixture_diagnostics` test — the
standing triage workflow):

- **5 proposer extensions** (all probe-verified): `needless_return` bare
  trailing `return;` deleted via `delete_stmt_edit` (the old "unit fns
  keep it" note was wrong — clippy fires, suggestion deletes the stmt);
  `double_parens` single-paren call/method-call ARG wraps stripped
  (`push((v + 1))` fires in real clippy — the old "tuple break" note
  confused token shape with AST shape: `(a, b)` is `Expr::Tuple`, never
  `Expr::Paren`); `redundant_pattern_matching` 2-arm bool-literal match
  shape (→ `e.is_some()`, either arm order, wildcard opposite arm);
  `needless_range_loop` MACRO-ARG DESCENT — format-family macro args
  parse as `Expr`s (file-relative spans survive the token round-trip,
  the `post_template_expr_args` contract) and join the exact-slot scan;
  `{i}` named captures refuse (would dangle after the pat rename);
  non-family macros mentioning `i` refuse (conservative token walk);
  the `__elem` collision check walks macro tokens too (a pre-existing
  shadowing hole); `useless_format` single-positional-arg form
  (`format!("{}", e)` → `e.to_string()`, clippy's own suggestion) for
  non-bare-ident args — bare idents stay `uninlined_format_args`
  territory (clippy's own precedence; racing siblings on the same span
  would be nondeterministic).
- **`static-str` emits `const fn`** — the rewritten shape is trivially
  const-able and clippy's `missing_const_for_fn` (pedantic) fired on the
  non-const result (2 of run #2's 3 created); `const` is additive
  (callable everywhere a fn was, incl. fn-pointer coercion); skipped
  for async/unsafe. ALSO fixed a latent soundness hole: a trait-impl
  fn's signature MUST match the trait — the old `for_each_fn` walker
  (`ImplItem::Fn` covers both impl kinds) fired where the rewrite can
  never compile; static-str now walks trait-aware and skips trait impls.
- **2 residuals documented as boundaries** (not bugs):
  `single_option_map` (nursery design-opinion, "fn that only maps over
  argument") fires on `manual_map`'s OUTPUT — exactly what clippy itself
  suggests; `cargo clippy --fix` produces the same cascade. The one
  `useless_format` persist (`composite-range_loop_plus_format_args`):
  String-typed inlined-named `format!("{elem}")` still fires (probe —
  clippy is type-aware; it does NOT fire for u32), clippy's own fix
  (`elem.to_string()`) then fires `implicit_clone` on `&String`, and the
  true terminal (`elem.clone()`) needs the arg's TYPE — firing would
  trade the persist for an `implicit_clone` created (+1.9pp heal vs
  +1.9pp created, bench-symmetric), and `created` is the trust axis, so
  the persist stays. The parse-only frontier, honestly recorded.

Validation: 649 default / 733 score_bench / 712 fix_verify / 701
latent_retrieval lib tests green; clippy 0 warnings all states; every
new shape has positive + negative fixtures (tuple args, guarded arms,
named captures, bare-ident args, opaque macro mentions, `__elem`
collisions, trait impls, async fns).

**Plan 005 (2026-08-14) — latent retrieval + `cargo heal --suggest` CLI
(Phases 1–4 DONE).** `src/draft/` ships the span-level retrieval layer:
real `.rs` files → `AstChunker` spans → 8-D modelless embedding →
domain-centroid routing (annotation only) → fan-out retrieval against
per-domain latent indexes of the corpus → ranked rules + pattern
snippets. ZERO retrieval code reimplemented — composes riir-rag as-is
(`AstChunker` + `ModellessEmbedder` + `NeuronDbRetriever`) behind the
opt-in `latent_retrieval` feature. GOAT G1–G4 ALL PASS (Bench 018):
G1 100% (12/12 top-1 precision), G2 349µs/chunk (<1ms target, 2.9×
headroom), G3 104/104 lib tests, G4 `SuggestScratch` stable across 50
repeat scans. Two empirical deviations: fan-out retrieval instead of
gated routing (the 8-D centroids mis-routed fixtures — verdict is an
annotation) + multi-scale indexing/queries (corpus patterns are 1–5
line excerpts vs whole-item spans). The `cargo-heal` bin exposes it:
`cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/`
(smoke: 19 files, ~0.4s incl. index build). Phase 4 (2026-08-14) landed
the top-5 rust_perf `syn` AST proposers (`src/domains/rust_perf/proposers/`,
`propose_fix` dispatch + 36 fixture tests — see
[`.docs/11_domains/rust_perf.md`](.docs/11_domains/rust_perf.md) §Proposers).
Phase 6 T6.1 wired `--fix [--write] [--quiet]` with the span-preserving
edit splice (Bench 019, G1–G4 PASS). **T6.4 (2026-08-14)** migrated the
10 clippy_lints proposers to the same emitter — shared substrate in
`src/edit_emitter.rs` (feature-neutral, re-exported by both domains),
`propose_fix` signature preserved for `self_evolve`, `--fix` extended to
the structural clippy lints (`FIXABLE_LINTS`; needless_return,
redundant_field_names, manual_map, unnecessary_unwrap,
needless_range_loop, needless_lifetimes — 6) with bounded-fixpoint
application so nested fixes compose; the type-blind lints
(needless_borrow, useless_conversion, unnecessary_mut_passed, and
redundant_closure — demoted from FIXABLE by the T6.2 G5 real-file run
after it caught an E0631 fn-path coercion break) stay `self_evolve`-only.
The migration fixed four latent detection bugs in the old mutation-based
proposers (needless_lifetimes elision unsoundness incl. `FnMut(&'a S)`
positions + receivers; needless_range_loop `&v`-on-ref-param `&&Vec`
non-iterator + token-heuristic `v[v[i]]` false positive; unnecessary_unwrap
receiver mismatch) — see Plan 005 §Phase 6 T6.4 notes.

**Native rustfmt integration (2026-08-19, `src/fmt.rs`).** `--fix --write`
now rustfmts healed files AUTOMATICALLY — default-on, `--no-fmt` opts out.
This encodes the heal-slice formatting discipline as tool behavior so no
SKILL is needed: (1) the **fmt-clean-at-HEAD gate** runs `rustfmt --check`
against the ORIGINAL on-disk file pre-write — only files already clean get
fmt'd post-write, so pre-existing drift never enters a mechanical diff;
(2) **CRLF files are skipped** (rustfmt would convert every EOL — the
slice-13 trap); (3) the **edition comes from the owning Cargo.toml** (the
let-chain parse trap; default `2024`); (4) rustfmt missing / parse failure
→ warning + skip, never fatal. Dry runs report the per-file **fmt plan**
(`fmt: would-format` / `skip (pre-existing drift)` / `skip (CRLF)`) + a
summary line; `--verify` applies fmt post-compile-gate to kept files only.
Documented in README §"Automatic rustfmt" — the manual per-slice fmt-clean
classification ritual is retired for future consumers.

**Plan 005 Phases 7–14 closeout (2026-08-16) — the plan is COMPLETE as a
document: every task shipped, refuted, or defer-marked with a measured
reason.** No phase is left "not started".
- **Phase 7 CLOSED as a NEGATIVE RESULT** (commits `92b8436`+`76d5ad2`):
  every "the healer learns its way to better retrieval" mechanism measured
  flat-to-negative — domain-keyed EMA evolution **−1.0pp** vs the corpus
  centroid (T7.1, `tests/t71_evolved_directions.rs`; the class mean IS the
  cosine optimum, so online EMA is a noisier estimate of the same quantity);
  the router half rests on two stale premises (T7.2 — router is display-only
  + the 8-D directions are off the shipped TfCosine path); the
  trajectory-memory axis has **0/25 improvable held-out rankings and would
  break 19/25 correct ones** (T7.3, `tests/t73_memory_axis_headroom.rs`).
  What actually lifted retrieval was EVIDENCE, not learning (tokenizer
  +16.6pp, instantiation re-rank +12.0pp). Shipped from the phase: **T7.4** —
  `LatentFixMemory::to_json` emits a BLAKE3 commitment over the canonical
  trajectory serialization; `load_json_verified` reports
  `CommitmentStatus::{Absent, Verified, Mismatch}` (advisory not gate, per
  the `Corpus::thaw` precedent; pre-T7.4 files load as Absent).
- **Phase 9 CLOSED** — **T9.3 SHIPPED + GOAT PASS (Bench 028)**:
  `HealPipeline` candidate selection is now `SelectionMode::BetaPosterior`
  (DEFAULT) — the ε-quantile of Beta(1+S, 1+F) replaces the raw S/(S+F)
  rate, so a lucky 1/1 (Beta 0.224) no longer outranks a proven 18/20
  (Beta 0.729); selection regret **−8.4%** over the 1–5-attempt regime
  (−23.8% at 1 attempt; honest +2.9% crossover at 30 attempts documented at
  the enum variant — BetaPosterior is right for the thin-evidence regime
  the healer occupies, not universally). Unseen candidates now score 0.32,
  not 0.5. **T9.4 REFUTED four ways (Bench 029)**: `clr_vote` doesn't exist
  (ships as `clr_reliability_scores` + a `clr_vote` *parameter* on
  `compaction::gate`), a "draft" is a verbatim corpus row (no generative
  diversity to sample), the K=8 ensemble collapses (identity guard declines
  189/192, distinct drafts per producing case 1.00 of 8), and L2 accepts
  3/3 (no discriminative power). T9.1/T9.2/T9.2b DEFERRED —
  `DraftedFix.confidence` is written (`src/drafter.rs:246`) and read by
  nothing; Issue 023's findings (a cold calibrator emits a ZERO-WIDTH
  0%-coverage interval; coverage crosses 90% at exactly n=19 residuals)
  are recorded and binding for any reopen. Per-domain keying (not per-rule)
  is the amended default if T9.2 reopens.
- **Phases 10, 12, 14 DEFERRED after pre-flight (Bench 030)** — substrates
  verified REAL except `ConsolidationPipeline::can_freeze` (does not exist;
  4th stale substrate claim in the plan). Blockers are measured: T10.6's
  ≥10% gate vs **4.0pp total headroom** (24/25 held-out already correct at
  96.0% top-1); Phase 12 has no multi-step trajectories to classify (33/33
  single-step; `oscillation` unreachable by construction; `--fix --verify`
  already ships the escalation it wanted); T14.2's O(N)-scan premise is
  false (HashMap lookup 1.40× over a 100× size increase), T14.3 refuted by
  T7.3's measurement, T14.5 demands ≥5% from the same 4.0pp budget.
- **Phase 11 DEFERRED with a reopen condition (Bench 030)** — latency
  justification measured dead (ceiling 1.0655× vs the ≥1.5× gate,
  Issue 023); the quality gate targets an EMPTY set (L2 accepts 33/33 real
  proposer drafts — would-fail-L2 has zero members, G1 is a recall over
  nothing, G2 degenerates to trivially-true). The validator that does
  reject is `fix_verify`'s cargo check: first measured rejection rate
  **2/14 = 14.3%** (Bench 031), ground truth n=2. Reopen = re-specify G1/G2
  against `fix_verify` AND accumulate enough verify-tier revert data to fit
  + certify a rejection predictor. No `poincare_imagination` feature gate
  was created.
- **Phase 13 research half MET** — [Research 058](.research/058_convex_ast_vocab.md)
  (`kind + arity` colouring; compression gate re-specified to downstream
  retrieval top-1); experiment deferred on headroom (4.0pp).
- **Bench 027 collision resolved at closeout:** T3.5's G1-recovery bench
  (commit `0c5af53`, first) and Issue 015's verify-mode GOAT (commit
  `4b5c465`) were both allocated 027 same-day. Verify-mode renumbered
  **027 → 031** per the tie-breaker (fewer live references moves); T3.5
  keeps 027. Highwater now 31.

**Issue 016 (2026-08-16) — structural lint expansion 6→33 + `--verify`
tier 1→4.** 30 new lints distilled frequency-ranked from kornel's
100K-crate clippy run (v1.80–v1.83; `semicolon_if_nothing_returned`
41,601× — the most frequent structural lint in its top-10) + Li et al.'s
8.09M-warning crates.io study (top-5 lints = >80% of all warnings) ∩
clippy 1.93's `MachineApplicable` axis ∩ parse-only provability. 27
landed in `FIXABLE_LINTS` (semicolon_if_nothing_returned,
collapsible_if, collapsible_else_if, single_match, single_match_else,
redundant_pattern_matching, question_mark, range_plus_one,
uninlined_format_args, identity_op, needless_bool, needless_bool_assign,
match_bool, needless_match, redundant_static_lifetimes, double_parens,
unnecessary_lazy_evaluations, map_unwrap_or, filter_next, iter_nth_zero,
manual_unwrap_or, unused_unit, useless_format, deref_addrof,
wildcard_in_or_patterns, unnecessary_sort_by, iter_next_loop) + 4 in
`VERIFY_LINTS` (comparison_to_empty — 12 live `.len() == 0` hits in our
own workspace; while_let_on_iterator; single_char_pattern — type-blind
shapes the Issue 015 compile gate auto-reverts; manual_is_multiple_of —
the post-mining P7 row: `x % d == 0` → `x.is_multiple_of(d)`, probed vs
clippy 1.97 — parse-only cannot see dividend signedness and
`is_multiple_of` is unsigned-only, so the verify gate reverts
signed-dividend breaks; e2e proven on a scratch crate: unsigned site
healed Certified, signed site auto-reverted byte-exact). **P11 follow-up
(same day): `Expr::Cast` removed from the shared `is_postfix_safe` —
method calls bind tighter than `as`, so cast receivers now parenthesize
(`(x as u64).method()`); the P7 local exclusion subsumed + the two
identical local copies (`manual_map`/`unnecessary_unwrap`) deduped; also
closes the latent `!x as u8` = `(!x) as u8` wrong-condition class in the
negation users.** Notable spec guards:
identity_op's IEEE-754 split (`+0.0` is NOT identity for `-0.0`, so
unsuffixed `+ 0` never fires; `- 0`, `* 1`, `/ 1` are float-safe),
manual_unwrap_or/unnecessary_lazy_evaluations purity gates (eager-vs-lazy
evaluation timing), uninlined_format_args' trait-bound identity +
abort-on-any-spec conservativeness, iter_next_loop as the one correctness
lint (the rewrite IS the intended semantics fix). Corpus 10→40;
per-lint proposers under `proposers/` with buggy/clean fixture tests;
corpus-lockstep test (`all_buggy_codes_fire_their_lint`) + suggest
gretrieval test (`every_lint_retrievable_via_suggest`, latent+BM25 top-5).
One pre-existing latent_matcher span test relaxed top-1 → top-3 (corpus
growth legitimately re-ranked one query — manual_unwrap_or's fixture is a
stronger BM25 token match). GOAT: 369 default + 383 latent_retrieval lib
tests, clippy 0 both states. See
[`.benchmarks/021`](.benchmarks/021_clippy_structural_expansion_goat.md).

**Issue 017 (2026-08-16) — retrieval-precision eval at scale.** Closes the
12-fixture gap: the Plan 005 G1 claim (100% top-1 on 12 fixtures) is **dead
at scale — measured 58.3% clippy top-1 on 48 spans / 40-rule corpus**
(nearest-neighbor crowding is real; concentrated in tiny-token patterns —
`identity_op` alone tops 5 misses). The eval
([`src/draft/retrieval_eval.rs`](src/draft/retrieval_eval.rs) + the two
fixture data files) ships **130 spans** (48 clippy variants real-clippy-
verified at authoring + proposer-verified standing, 40 perf variants, 30
kernel self-variant probes via the deterministic `mutate_shape` rename map,
12 REAL production clean excerpts), per-domain + blended top-1/3/5 recall,
a miss taxonomy, the clean false-positive profile, router-annotation
accuracy (53.4% 2-domain / 46.6% 3-domain — feeds Issue 020), and a
committed regression gate with floors = baseline − 1 span (tie-order
slack). One command: `cargo test --features latent_retrieval --lib
retrieval_precision_regression_gate -- --nocapture`; kernel-inclusive
(~54 s): `--features latent_retrieval,kernel_heal`. Baseline at Issue 017
land time: clippy 58.3/75.0/87.5, perf 87.5/95.0/97.5, kernel
self-variants 100/100/100 (crowding alarm if it drops). **SUPERSEDED by
Plan 005 T3.5 (Bench 027, 2026-08-16) — see the block below.** **Side-effect root-cause fix:** the fixture
pass exposed our `needless_bool_assign` proposer matching the assign-RHS
form (real clippy calls that `needless_bool`) — proposer rewritten to the
real branch-assign shape + corpus/row/fixtures updated. Known drift
recorded, not fixed: `comparison_to_empty` implements `.len() == 0` (real
`len_zero`); widening to `== ""` is a small follow-up. GOAT G1–G4 PASS
(370 default / 391 latent_retrieval / 526 kernel_heal, clippy 0 new
warnings). See [`.benchmarks/022`](.benchmarks/022_retrieval_precision_eval_at_scale.md).

**Issue 018 (2026-08-16) — `self_evolve` promoted to DEFAULT-ON with
graceful oracle degradation.** The mechanism/ oracle split: the memory
half (`LatentFixMemory` — every applied fix is a trajectory, every
compile-revert a definitive fail) needs NO oracle and now runs in EVERY
build (modules unconditional incl. `--no-default-features`); the EMA
learning half needs trustworthy verdicts and no-ops with **exactly ONE**
log line when the `clippy_verify` oracle is unavailable (the same shape as
Issue 015's verify-absent path). New `EvolveRecorder`
([`src/recorder.rs`](src/recorder.rs)) — the standalone consumer for the
`cargo heal --fix` path (the HealPipeline is untouched): `record_applied`/
`record_reviewed` (memory-only), `record_outcome` (stats always, EMA
gated), `disable_learning(reason)` (runtime oracle failure), plus
**persistence** — a CLI process is short-lived, so the memory persists as
a v1 JSON envelope (`RIIR_HEAL_MEMORY` override, else the platform cache
dir → `riir-clippy/memory.json`; merge-on-load keeps live stats;
corrupt/unknown-version → `InvalidData` + continue fresh, never aborts a
run). `--fix --write` records written edits; `--verify` records kept edits
memory-only (a rustc pass proves non-breakage, not lint resolution) +
reverted edits as `success=false` (sound verdicts — the clippy verifier
cannot soundly verify span edits standalone, a non-compiling crate reports
"lint absent" = false success). Opt-out: `RIIR_HEAL_SELF_EVOLVE=0|off|
false|no` — fully inert (no recording, no persistence, no log). The
`self_evolve` feature stays as a default-on back-compat alias
(benches'/examples' `required-features`). GOAT (Bench 023): G1 contract
tests both oracle states + the production `--fix` accumulation path; G2
real-bin e2e (1→3 trajectories across runs, one degradation line per run,
inert opt-out); G3 405 default / 419 clippy_verify / 652 combined lib tests
+ clippy 0 warnings in 5 states + `ci_feature_guard.sh` all-layers; G4
`cargo tree` diff EMPTY (dep surface unchanged). **Honest finding:** the
G7 demo re-run aborted G6 at 35/40 PRE-EXISTING (proven by baseline
worktree at `a532b7c` — 5 Issue-016 corpus entries' `buggy_code` didn't
trigger under real clippy; filed as Issue 022 — **RESOLVED same day, see
the Issue 022 section below: G7 now runs end-to-end at 40/40 truthfulness
+ 50%→100% healing**). See
[`.benchmarks/023`](.benchmarks/023_self_evolve_promotion_goat.md) +
[`.docs/10_self_evolve/evolve_recorder.md`](.docs/10_self_evolve/evolve_recorder.md).

**Issue 022 (2026-08-16, RESOLVED same day) — corpus real-clippy drift
(5 Issue-016 entries).** Found during the Issue 018 GOAT: the G6 demo
aborted at 35/40 — 5 corpus entries' `buggy_code` didn't trigger the named
lint under real cargo clippy **1.97** (the toolchain upgraded since the
Issue 016 authoring; proven pre-existing via a baseline worktree run at
`a532b7c`). Root causes, each verified by temp-crate + JSON-probe:
`iter_next_loop` + `while_let_on_iterator` code **didn't compile**
(missing `mut it` — `.next()` needs `&mut self`; invisible to parse-only
proposers); `.iter().nth(0)` on a slice routes to 1.97's more-specific
`iter_nth` (`iter_nth_zero` needs a non-slice iterator — fixed to the
IntoIter shape); `single_match_else` fires only when the else arm is a
STATEMENT BLOCK (a bare-default else routes to `manual_unwrap_or[_
default]` — 10 shapes probed; the Issue 017 fixture had the firing form all
along); `comparison_to_empty`'s real shape is `s == ""` (ours was
`.len() == 0` = real clippy's `len_zero` — the proposer was WIDENED to
detect both forms, the corpus moved to the truthful shape). **GOAT (Bench
024): G6 40/40; G7 restored — baseline 50.0% → evolved 100.0% (40/40,
+50.0pp) on real cargo clippy, now over the full 40-lint corpus** (the
strongest oracle-present evidence for Issue 018's promotion); 408/429 lib
tests + clippy 0 in 4 states + CI guard all-layers; retrieval floors
RE-PINNED per the Issue 017 contract (clippy 58.3→54.2 top-1, blended
3-domain 78.8→77.1; the −2 spans are the len-shaped
`comparison_to_empty` fixtures — truthfulness wins, a corpus that
misrepresents lint behavior poisons `self_evolve` learning). Bonus
root-cause fix: the eval's determinism test caught a score TIE on the
top-5 boundary flipping via backing-map iteration order → `canonical_rank`
(score desc via `total_cmp`, `rule_id` tie-break) makes the measurement
reproducible (a substrate-level riir-rag tie-break remains a documented
follow-up). Probe-harness lesson: command-line-requested lints render
HYPHENATED in human output (`clippy::needless-return`) — grep JSON
(`"code":"clippy::..."`) instead. See
[`.benchmarks/024`](.benchmarks/024_corpus_real_clippy_drift_goat.md).

**Issue 020 (2026-08-16, RESOLVED-KILLED) — DomainRouter spectral routing
POC.** The falsifiable question: does a router-local **token-frequency
spectral block** (k=8 DFT coefficient pairs over a top-128-token vocabulary,
two orderings + structural suffix + a full-TF ablation — 7 variants) reach
the ≥85% routing-accuracy kill-bar (Bench 022's bar: below that, gating
trades correctness for a fraction of the 349µs/chunk fan-out cost)?
**Verdict: KILL for gating** — best variant 70.3% (tf-cosine-disc, 3-domain)
/ 75.0% (dft8-global+struct, 2-domain — but mis-routes the documented
fixtures). The load-bearing finding (the ablation splits the failure):
**full-128 TF cosine ≈ DFT-k8 over the discriminative vocab (70.3% vs
69.5%) — the k-coefficient compression loses NOTHING; the lexical signal
itself is the ~70% ceiling**. Domain membership is semantic, not lexical
(a perf HashMap rule + a clippy HashMap rule share a token spectrum — the
issue's predicted mode, now measured). Only a semantic-signal embedder
(768-D class) could re-open gating; the lexical branch is CLOSED.
**Salvage shipped (annotation-only, no gating risk):** `LatentMatcher`'s
router now builds with `TfCosine { order: GlobalFreq }` — annotation
accuracy 53.4%→69.3% (2d) / 48.3%→68.6% (3d) at 0.96µs vs 3.31µs per span
(global over disc ordering: equal accuracy, insensitive to the
enabled-domain set). A standing floor (63.6% both states) joined the Issue
017 regression gate — annotation decay now re-measures + re-pins, never
silently weakens. The 7-variant measurement matrix lives on as the
reproducible negative-result artifact (`run_router_eval` + the
`router_spectral_poc_report` test — the `gemma2_directions` precedent).
Baseline reproduced Bench 022 exactly (53.4% 2-domain) — harness valid.
Determinism (BTreeMap vocab + precomputed twiddle tables, lookup-only
HashMap) + cost (0.96–4.06µs release; the gate is release-only, the `g2_*`
Debug pattern — debug pays ~10× on the transcendental paths) both PASS.
Note: the two original mis-route fixtures now route correctly even under
the baseline (current-corpus centroids) — overall accuracy, not those two
fixtures, is the live failure mode. 408/435/592 tests + clippy 0 in all
states. See
[`.benchmarks/025`](.benchmarks/025_router_spectral_poc_kill.md).

**Issue 021 (2026-08-16, RESOLVED) — evidential-status axis for
`LatentFixMemory`.** The retelling-bias defense (arXiv:2608.11195 /
Research 049): the old schema (success/fail u32s + boolean
`record_outcome`) could not distinguish a full-validator admission from a
syn-parse-only success, and iterated retrieval launders the latter into
the former. Shipped (Bench 026): `EvidenceTier` (`#[repr(u8)]`
Certified/Heuristic/Withdrawn) on `FixTrajectory` with
absorbing-Withdrawn transitions (the riir-chain slashing-monotonicity
pattern: 3 consecutive post-hoc failures → Withdrawn; ONLY
`(success && verified)` — a fresh full-validator pass — promotes or
exits); `consecutive_fails` streak driver (reset on success — stats
climb honestly while the tier decides); trust-weighted retrieval
(`RetrievalWeights.trust` default 0.2: Certified 1.0 / Heuristic 0.5 /
Withdrawn EXCLUDED from suggestions, still reachable via `iter()`/`get()`
for failure analysis); certification admission
`record_certified` wired into the Stage 2 tuning loop (verifier-passed
fixes now land in memory as Certified — previously the tuning loop only
fed the adaptive pool); `record_outcome` gains `verified` on
`LatentFixMemory` + `HealPipeline::report_outcome` (the demo passes
`vr.ran`; G5's simulated outcomes honestly pass `false`);
`EvolveRecorder` keeps its 4-arg CLI shape (`heal.rs` untouched — its
call sites are compile-gate reverts where the flag is behaviorally
irrelevant) + `record_outcome_verified` for future full-validator
callers. Pre-021 memory files decode as Heuristic (`#[serde(default)]`).
**The honest A/B finding:** the issue's original A/B ("two entries,
equal similarity, different tiers") is NOT constructible through the
real API — `record_outcome` matches by `(lint_name, fixed_code)`, so
entries sharing that pair are ONE stats bucket (the first test run
failed on exactly this, proving the gate is load-bearing). The
falsifiable A/B is the within-subject certification delta (exactly the
0.1 trust gap, tolerance 1e-5) + the cross-entry small-edge flip (a
Heuristic entry with STRICTLY higher latent, found by runtime search —
the hash embedder has no locality — still loses when the edge is under
the gap). Design constraint measured: at trust 0.2, a different-lint
Certified entry can never outrank a same-lint Heuristic one — trust
breaks ties, relevance dominates. Scope-of-validity (the stretch goal)
deferred per the issue's own defer clause. 415/429 lib + full CI guard
all-layers (668 combined) + clippy 0 in default/clippy_verify/
latent_retrieval states. See
[`.benchmarks/026`](.benchmarks/026_evidential_status_axis_goat.md).

**Issue 015 (2026-08-16, RESOLVED) — `--fix --verify` compile-gate mode.**
The mode (feature `fix_verify`, implies `latent_retrieval`; landed in
`2ace442`) closes the loop IN the tool: apply edits with loosened gates →
baseline `cargo check --all-targets --message-format=json` → write → map
rustc JSON errors to file/line → revert ONLY the offending edits
(line-matched span-level, whole-file fallback) → re-check → report. Under
`--verify`: the vec-with-capacity usize gate loosens to `untyped_paths`
(plain-path bounds attempted; E0308 backs the revert — the scope gate
STAYS ON: use-before-decl is a correctness issue, not a type question);
`VERIFY_LINTS` (`redundant_closure`, `comparison_to_empty`,
`while_let_on_iterator`, `single_char_pattern`) join the fix set; the
`fxhashmap` dep gate text-scans the owning crate manifest for `rustc-hash`
(hard-skip without cargo — the measured 422-edit instance class). **GOAT
(Bench 031):** real-repo run on the 6 katgpt-pruners files holding the
Bench 019 G5 skipped sites — 14 edits proposed / 12 kept / **2
auto-reverted on the real repo** (the u8 `BOARD_SIZE` E0308 + the
`|v| from_bool_vec(v)` E0631 coercion, exactly the G5 run-1 failure
classes), every named skipped site landed compile-confirmed usize
(`hydra_budget` n, `collapse_detector` N_TRACES, `rosetta` len,
`episode_pruner` min_len, `monopoly` `Strategy::COUNT`), **1245/1245 tests
pass + no new clippy warnings** under the 15-feature covering set; landed
as katgpt-rs `212ed7942`. The GOAT run itself surfaced + fixed a root
cause: relative path args made `owning_package_root` walk to the EMPTY
path (os error 123 from `current_dir("")`) — now normalized via
`std::path::absolute` with a regression test. The `#[ignore]`d e2e
fixture test (`fix_verify_e2e_bad_edits_reverted_good_survive`, spawns
real cargo) is the standing acceptance proof. Honest scope note: a
full-crate katgpt-pruners scan now proposes **293 edits / 104 files**
(Issue 016's expansion) — full-repo verify batches are a consumer
decision, not a tool default; this GOAT was deliberately file-scoped.
See [`.benchmarks/031`](.benchmarks/031_fix_verify_mode_goat.md).

**Issue 040 (2026-08-25→26, RESOLVED — arms A/B run, verdict landed; removed per
noise-reduction rule).** The fixer re-gate POC — the falsifiable question
"does verify-filtered acceptance beat `decline` on the reachable-miss set,
under partial-credit metrics, on any arm?" — answers **YES** ([Bench
049](.benchmarks/049_fixer_regate_poc.md) §Arms A/B results): CF
llama-3.3-70b-fp8-fast via the wrangler OAuth bridge (`ai:write` scope),
**strict-keep 5/9 (arm A raw) / 6/9 (arm B pre-pass) vs decline 0/9**; the
pre-pass dominates every partial-credit axis (v2edit slice 18 vs 12 keeps,
parse 20/20 vs 15/20, merge 19/20 vs 13/20); never-wrong held (0 divergent,
0 regressions; the contract veto killed the one destructive generation per
arm — the Bench 039 route-2 veto live on real CF output). EM 0/40 reproduces
riir-train Plan 336's 0/60 — structurally capped by the 96.6%-unmergeable v2edit
references, corroborating the metric critique. **Proposal 005's fixer
QUALITY blocker LIFTS for the CF llama arm** (its own "CF llama arm OR
base-Bonsai arm" language; caveat #2 + Phase 3 updated in the same commit);
leak-posture sign-off remains the owner's separate gate. **qwen3.8-27b (the
owner's mid-flight switch) measured UNUSABLE on the free tier**: a thinking
model whose reasoning phase consumes `max_tokens` and returns
`content: null` at `finish_reason: length` — the same waste class as the
169-request incident, root-caused by the standing one-call smoke at a cost
of 118.1 n; `/no_think` unreliable (the model deliberates about honoring
it); `enable_thinking:false` ignored by the serving stack. `CfClient::generate`
now appends `/no_think` for qwen3-family ids regardless (documented
mitigation, inert for llama). Bonsai arms C/D stay blocked-optional
(GPU-exclusivity + no `.bcl4` checkpoint + the riir-train driver
hard-requires `--lora`) — they remain the serving-path follow-up, not a
verdict dependency. Cost: 1,433.2 n session (server truth; account day
closed 9,742.4/10,000 — cap respected); llama ≈ **$0.00025/fix measured**
(6.8× under the issue's estimate).

## CF ops surface + the `.env` contract — full record (was AGENTS.md lines 3044–3177; Issues 044/045/051/062/065 resolutions + dashboard-auth state + deploy hazard)

## CF ops surface + the `.env` contract (Issues 044/045 — read before touching ops)

This repo hosts the **LLM ops plane**: the `riir-kat-housekeeping` worker
(`cloudflare/housekeeping/` — hourly cost/budget cron), the
`riir-ops-dashboard` worker (`cloudflare/dashboard/`), and the nightly
generator (`scripts/ops_nightly.sh`). Credentials live in **`.env`
(gitignored — never commit, never print values)**; [`.env.example`](.env.example)
is the documented template (per-var purpose, consumer, minting steps).
AI sessions: treat `.env` as read-only local state — pipe values into
`wrangler secret put` via stdin (`grep '^NAME=' .env | cut -d= -f2- | …`),
never echo them.

**Issue 051 (groq-first LLM routing) RESOLVED + removed 2026-08-31 per the
noise-reduction rule (full session records in git history —
`git log -- .issues/051_*.md`). All T1–T15 landed and live-proven: the free
leg serves first (`LlmRouter::from_env` no longer requires CF creds to
build — the T9 fix), T7's live window absorbed 2/2 calls for $0.0000
metered / 0 neurons, T14/T15 close both "empty backlog" falsification
halves.** The one defer, preserved here because the file is gone: **T8 —
`fixer_regate` arms stay CF-pinned BY DESIGN** (the arm rows are MODEL
comparisons; mixing a free router would confound the comparison the arms
exist to make) — re-open only with an explicit free-tier arm. The
scaling-direction note (owner): this ladder is the TEMPLATE for more free
tiers — a provider list + per-provider quota state, not a hardcoded pair.

**Issues 044 + 045 RESOLVED + removed 2026-09-01 per the noise-reduction
rule (full narratives in git history — `git log -- .issues/044_*` /
`git log -- .issues/045_*`).** All engineering in both is landed,
deployed + live-verified (every claim re-measured against PROD on
09-01, not prose): the dashboard (six-source one-URL HTML; token gate
deployed fail-open + the T13 CSP/no-store/redaction hardening
`82c5032d`; the T12 per-day attribution fix + reconciliation card
`6b8c8a10`), the nightly runner (T14 toolchain preflight + T16
exit-2-on-failed-deploy + T16.1 `env -u` credential-shadowing fix +
T16.2 post-deploy verify + T16.3 least-privilege-arm fix; the CI twin
reconciled in `55fd477`; launchd INSTALLED and kickstart-GREEN under
the REAL launchd environment 09-01 — regen 15.5 min, verify_dom PASS,
artifacts `1143a62`, deploy `verified`, housekeeping tests 45/45), the
housekeeping Worker (T7 `ca93c8de` → T8+T9e `64bea65c` → the T9 series
+ T9f `2e07d125`; full 31-day cursor backfill executed to
`next_end:null`), and the KAT mint swap-in (worker half `a07b9e6`
deployed `693972ec`; server half riir-dapps `10ffb18`, exactly-once per
`(date, neurons)`). The residuals, preserved here because the files
are gone:

**Issue 065 (the ops cost model vs the CF bill) RESOLVED + removed
2026-09-04 per the noise-reduction rule — full narrative in git history
(`git log -- .issues/065_*`); evidence in
[Bench 067](.benchmarks/067_cf_bill_reconciliation.md) + the committed
`.benchmarks/cf_bill_pin.json`; its residuals are items 1–2 below and the
owner steps (set `MONTHLY_BUDGET_USD`; override
`CF_BILLING_ANNIVERSARY_DAY` once the real day is known).**

1. **Dashboard auth gate CLOSED 2026-09-03 (Issue 065 T8) → FLIPPED PUBLIC 2026-09-04 (owner call):**
   `DASHBOARD_TOKEN` set via `wrangler secret put`; live-verified all four
   branches (anon → 401 / wrong token → 401 / correct → 301 with token
   stripped + cookie / cookie → 200). **2026-09-04, owner call ("it should
   be public"): `AUTH_MODE="public"` now serves every caller**
   (`x-dashboard-auth: public`; `wrangler.toml` `[vars]`, version
   `9f25ff9f`) — the token gate is DORMANT, not deleted: the secret stays
   set as a private door, and deleting the `AUTH_MODE` var re-arms the 401
   gate with the same secret. Public mode deliberately bypasses BOTH the
   token gate and the silent fail-open branch (that one still answers
   `OPEN-INSECURE` for any future accidental state). The T13 payload caveat
   — the page carries repo names, lint counts, costs, fixer internals — now
   applies to anonymous visitors BY OWNER DECISION. The old
   `UNCONFIGURED_POLICY` residual is moot while public mode is armed; if
   the public mode is ever retired, revisit `deny` before relying on the
   secret alone. `verify_auth.mjs` covers the public branch (14 scenarios).
2. **Push paging owner-gated**: `ALERT_WEBHOOK_URL` + optional
   `MONTHLY_BUDGET_USD` verified ABSENT live 09-01. Collection +
   cap-crossing detection are LIVE (since 08-29); crossings land in
   KV + `/status alerts_today` and page nobody.
   **The MONTHLY_BUDGET_USD trap is now FIXED (Issue 065 T7, 2026-09-04,
   deployed `1314c925`, live-verified `/status` → `bill_model:
   "issue065-t7"`):** the old alert summed per-day MARGINAL `est_usd` rows
   (from-unit-zero — 4.7× high, containers-only, and the family that was
   99% of the bill unwatched). The alert now reconciles the account-wide,
   period-scoped CF bill (`src/bill.js`, ported from `scripts/gen_dashboard.py`,
   pinned to `.benchmarks/cf_bill_pin.json` by `test/bill.test.mjs`), with
   KV ops collected and degraded collectors named unwatched. Setting the
   secret is now safe; the period start rides the `CF_BILLING_ANNIVERSARY_DAY`
   `[vars]` pin (16, unfalsifiable bound 08-15..08-26 — override once known).
3. **KAT mint go-live (044 T5 / 045 T6)**: armed-but-INERT until the
   kat-service production deploy (riir-dapps surface) lands AND the
   owner pushes `KAT_SERVICE_URL` + `KAT_MINT_TOKEN` (verified absent
   09-01). The POST is exactly-once per `(date, neurons)`; 409/network
   failures fail-soft — a mint failure never fails collection.
4. **T7 first SCHEDULED-tick confirmation** (2026-09-02 01:23 local):
   the launchd-environment question was answered 09-01 by kickstart, so
   the scheduled tick is confirmation only and needs no agent. If it
   fails, the log + missing artifacts make it visible; re-open then.

Still true and stable: the 4 core vars are all SET in `.env`
(`GROQ_API_KEY`, `CF_API_TOKEN`, `CF_DEPLOY_API_TOKEN`, `READ_TOKEN`);
the worker collects neurons + containers hourly (`token_set:true,
read_gated:true`; anon `/hours` → 401); settlement is automated
(Issue 057): the nightly appends a settlement ledger row per completed
account day that drifts from CF GraphQL, commits it, and deploys.

**Deploy hazard (Issue 044 T16.1, load-bearing):** wrangler honours
`CF_API_TOKEN` as a deprecated alias of `CLOUDFLARE_API_TOKEN` — an
analytics-scoped value in the deploy env silently breaks auth. Every
`wrangler deploy` in this repo runs under
`env -u CLOUDFLARE_API_TOKEN -u CF_API_TOKEN -u CLOUDFLARE_API_KEY -u CLOUDFLARE_EMAIL …`.
Keep that prefix; a bare `npx wrangler deploy` is the bug.

**Issue 062 (TPR × healer corpus fusion, katgpt-rs Research 527 F5) CLOSED-NEGATIVE +
removed 2026-09-03 per the noise-reduction rule (full narrative in git
history — `git log -- .issues/062_*.md`; verdicts in Benches 062 / 063 /
065 / 066, all committed).** The arc: Phase 1 measured the unmeasured axis —
OOD generalization is AT CHANCE on both the healer corpus (52.0% → 4.0%)
and rustc_errors (97.7% → 48.8%), which justified Phase 2 — and T2a fixed a
REAL upstream bug en route (ndb `Bm25Index::search` tie-truncation returned
a per-process candidate SET from a `RandomState`-seeded HashMap;
score-desc-then-key-asc total order before the truncation, ndb `aeb24e0`,
riir-ai `.issues/849`, 12-process determinism pin in riir-rag). Phase 2
refuted the consumer route: **T4a (Bench 065) — route (b) REFUTED as atomic
memorization** (ID top-1 climbs 21.1% → 81.1% across the admissible `d` grid
while OOD stays at chance; decisive row `d = 4`, selected by a rule stated
before it was applied, margin 0.0pp vs the 20pp bar; preregistration
amendments A1–A3 recorded, A3's refit-admissibility check fired at `d = 8`),
and **T6a (Bench 066) — the counterfactual-surgery gate is INCONCLUSIVE**
(the gate population is structurally starved); T6 stays UNWIRED with its
reopen condition recorded in the bench. **Consequence: katgpt-core `tpr`
stays opt-in** — there is no route to a consumer on this corpus, and that is
a consumer-side refutation, not a defect in `tpr` (its upstream G1–G4/G8
passed on planted corpora, which G8's own record calls a positive control,
never evidence about real corpora). Harnesses kept as reproducible
artifacts: `src/draft/{withheld_pair_ood,tpr_fit,tpr_t4a_withheld,tpr_t6a_counterfactual}.rs`
+ `tests/t062_*.rs` / `t063_tpr_structure.rs`. Reopen: the corpus gains
multiple fillers per role (the measured step-function axis), or a
structured-corpus consumer materializes.

## The `riir-clippy` auto-continue skill — full description (was AGENTS.md lines 3178–3212)

## The `riir-clippy` skill (auto-continue idle loop)

`.agents/skills/riir-clippy/SKILL.md` (symlinked into `~/.agents/skills/`
for global discovery — canonical home is THIS repo, global is a symlink
never a copy) encodes the idle-time workflow for the whole workspace:
(1) fix open issues, (2) finish in-flight plans, (3) clippy sweep across
repos (heal-driven), (4) resolved-issue hygiene (doc then rm), (5) batch
mining + heal dry-run (the centerpiece — includes the proposer adaptation
loop: wrong suggestion → fix the proposer + fixture, never suppress),
(6) boundary-guard routine check, (7) substrate-first drift audit,
(8) benchmark regression sweep, (9) online unblock search, (10)
defer-marking sweep, (11) online perf search, (12) paper research via the
research skill, (13) doc-sync. One unit per cycle, commit + push each
unit. The cross-session resume state (mining queue snapshot: next batch
targets, sibling-blocked files, corpus count) lives in
`.distill/001_mining_queue_snapshot.md` (moved out of the skill file
2026-08-18 after it grew to 104KB and broke Zed's loader) and is
updated in the same commit as each batch. Triggers: "continue",
"auto continue", "idle", "nothing to do", "next batch", "Batch 32+".
Auto_prompt handoffs are gated by the handoff summary's (3) What
remains section — "Nothing" falls through to the loop (the sanctioned
auto-continue path), listed work resumes first (the skill's precedence
rule, 2026-09-02).
**Issue 019 RESOLVED 2026-08-16 (removed per noise-reduction rule; gap
analysis in git history — `git log -- .issues/019_*.md`):** T6's full
clean cycle completed via queue items 10–13 — item 10 defer-marking sweep
(riir-train 387 defer-marked with unblock conditions; riir-ai 529
queued-not-stalled note; riir-chain 026 §5.5 checkbox fix), item 11 online
perf search (8 rust_perf candidates → the Batch 48 pipeline, recorded in
the SKILL.md mining snapshot), item 12 paper research (Research 053
EvolveMem retrieval-config co-evolution, Gain verdict), item 13 doc-sync
(5 repos clean/deferred-by-rule + game-sdk vocabulary fix `40f0971`).
All 13 queue items have now been exercised ≥1× through the idle loop.
T7 stays blocked on sibling WIP (`ternary_deltanet_gpu_forward.rs`).

## Production adoption — full record (was AGENTS.md lines 3213–3525; verdict evidence + resolved issues 037/038/050/059/060/061/066/067/068/069/070/071)

## Production adoption — `cargo heal` as the dev-time default (2026-08-24)

**Verdict: GO** (live trial, M3). The healer is now the DEFAULT tool for
mechanical clippy findings across the workspace, ahead of manual fixes and
`cargo clippy --fix`:

- **Global binary**: `~/.cargo/bin/cargo-heal` → symlink to
  `target/release/cargo-heal` (built `--features fix_verify,clippy_verify` —
  rebuild after healer source changes; the symlink keeps resolving).
- **Trial evidence**: 12/12 planted fixes landed, 2/2 planted divergence
  classes correctly refused (comment-guard + named-arg-rename), tests
  bit-stable, compile gate 2 checks / 0 reverts, idempotent re-run; real dry
  run on `riir-ai/crates/riir-games/src` → 222 edits / 54 files available.
- **Dev-time skill**: `cargo-heal` (`.agents/skills/cargo-heal/`, symlinked
  globally) — usage, divergence table, discipline, post-mining logging.
- **Post-mining** (term adopted 2026-08-24): mining
  improvement candidates from the healer's OWN usage artifacts (memory.json
  trajectories, misses, created-warnings, verify reverts, score-bench delta)
  — vs **pre-mining** (Protocol A, external sources). Intake lives in
  `.distill/001_mining_queue_snapshot.md` §"Post-mining intake"; consumed by
  idle-loop queue item 14 (Protocol C in the skill). Seed inventory 2026-08-24:
  8 ranked classes (macro-nesting family, array-literal guards,
  named-arg-with-rename, created-warning pair, outcome-verdict wiring, …) +
  the measured gap that 8,235 memory trajectories carry ZERO outcome verdicts
  (the sweep campaign validated manually; the verdict loop never closed — P5).
  **P5 CLOSED 2026-08-24** (Issue 037 T2, `667562a` — verdict matrix +
  clippy oracle + durable landing; see below).

**Issue 038 (2026-08-24, RESOLVED + removed per noise-reduction rule) —
self-evolve memory onto the neuron-db Warm-tier store (kill the JSON
envelope).** Owner's call ("log should store in neuron-db local kv? no need
to parse back and forth? … latent first with dag and ast"). Under
`latent_retrieval` the repo-local `.heal/trajectories.kv` store
(`src/traj_store.rs`: `TrajStore`/`TrajRowDoc`/`LearnRowDoc`) is the ONE
backend — the CLI opens it up front for all `--fix` paths, per-key Pod rows
with incremental puts, WAL encryption at rest; the envelope is neither
loaded nor saved when the store is active (light builds keep it by design).
One-time migration on the first persisting run: envelope present + store
empty → verbatim import → rename `memory.json.migrated` (once-only fence,
idempotence e2e-pinned by `tests/store_backend_e2e.rs`). Row decision: NO
separate Pod split — the row IS `TrajRowDoc` (text-carrying JSON doc row;
`record_outcome`'s exact-match reads `fixed_code` bytes, so the text lives in
the row). DAG/AST storage DEFERRED with a named consumer (Proposal 002's
deep-fusion E — file when E3-learn demands chains). Commits: `dd71f78`
(initial 038+039 scaffold) → `e4ce8ba` (the full landing: T1-T6, G1-G8;
default lib 660/0, full-feature 762/0/8i, clippy 0 both states) →
`efa254c` (durable EMA learn rows — 037-E3's store half) → `c9d718b`
(`--memory-stats` store-branch envelope parity + the F3 cadence line from
`last_seen_at`). Record: [`.docs/10_self_evolve/latent_fix_memory.md`](.docs/10_self_evolve/latent_fix_memory.md)
§Persistence + `scripts/self_evolve_audit.sh` §1 (store-first).

**Issue 050 (2026-08-28, RESOLVED + removed per noise-reduction rule) —
corpus + score series onto the Warm tier.** The corpus-count and score-run
series are now STORE rows (`CorpusCountRow` / `ScoreRunRow` in
`src/traj_store.rs`), not hand-appended jsonl: `cargo heal --corpus-stats`
counts the four corpus domains by source-walk (compiled flag via `cfg!`,
absent domain never emits 0), stamps today's rows, and `--json` emits the
`{today, corpus, scores}` contract the dashboard consumes;
`--import-series` is the idempotent one-time jsonl→store import (jsonl rows
yield to a first-hand `source-walk` stamp — same-day precedence pinned by
test). `--score-bench` dual-writes every run into the store (best-effort,
never fails the bench). The dashboard readers (`gen_dashboard.py`) are
STORE-FIRST via that JSON contract; the committed `.benchmarks/*.jsonl`
files REMAIN as generated projections — `.heal/` is gitignored, so the
committed jsonl is the only cross-clone durable record and the git-mined
corpus layer stays as the drift cross-check. Landed with T1 (the recorded
jsonl bridge, `586402c`). Record: git history — `git log -- .issues/050_*.md`.

**Issue 066 (2026-09-04, RESOLVED + removed per noise-reduction rule) — the
corpus series' docker counts were wrong since the domain landed (the
source-walk needle counted constructor plumbing, not entries).** The docker
corpus uses a positional shorthand `rule()` constructor with no per-entry
`rule_id:` text, so the `--corpus-stats` source-walk needle `rule_id:` (and
its `gen_dashboard.py` mirror) matched only the 2 plumbing occurrences —
historical stamps read docker ≈ 2 while the runtime count was 27→31, and
`load_corpus_history`'s store-wins merge preserved the poison across every
regeneration (the dashboard showed docker collapsing 31→2→31 across days
that never happened). Fixed in four pieces: **T1** the needle is now the
8-space `    rule(` constructor call + the count-pin test
`corpus_stats_needle_tests` pins EVERY compiled domain's walk == runtime
count (generalizes beyond docker) (`c9863d6`); **T3** the poisoned store
history repaired through the only sanctioned dated writer
`examples/repair_066_corpus_store.rs`, each value verified against
`git show <rev>` BEFORE writing, provenance `repair-066` so a future
first-hand stamp may supersede (`d731cac`); **T4** store-vs-mined
per-domain divergence LOGGING (never fails, never overrides — commit-time
vs stamp-time is a definitional boundary, not skew) + the mined-layer CWD
anchor it exposed (`_count_in_rev`'s `git show`/`git ls-tree` inherited the
caller's CWD — run off the repo root every count read 0 and the merge then
"won" those zeros over real store rows; both anchored to `REPO_ROOT`)
(`4239dbdd`). Record: git history — `git log -- .issues/066_*.md`.

**Issue 059 (Proposal 007 remainder — ops signer T2.2 + counsel gate Phase 3) RESOLVED
(2026-09-04) by owner decision — file removed 2026-09-05 per the noise-reduction rule
(canonical decision record: Issue 068 — removed 2026-09-05 with the tracker, digest in
the **Issues 061 + 068** entry below; live tracker: Issue 061 (same removal); full narrative in
git history — `git log -- .issues/059_*.md`).** Both open items were blocked on NON-code
decisions and belong to the USDC custody rail (Tier 2 — not proposed, demand-gated),
while the shipping arm is the custody-free KAT-stake arm that was already live.
**Ops signer WON'T-BUILD NOW** (068 D2): it serves only outbound USDC unstake refunds,
which exist only in the Tier-2 USDC custody rail, and it is the highest-blast-radius
component in the system (a custody key with send authority); the "missing"
RPC-provider/custody-posture answers are premature, not missing; re-open trigger =
Tier 2 authorized (a real inbound USDC deposit intent / an operator fiat-in ask),
re-opening as its own plan with real load numbers; accepted cost recorded: at Tier 2
the longest-lead item starts from zero. **Counsel gate DECIDED BY ARM SELECTION**
(068 D1): the custody-free KAT-stake arm ships under 005 caveat-#7 unchanged and was
already live (`staking_share_bp` 500, `yield_ceiling_bp` 22 armed in
`EpochParams::DEFAULT`; the ordering constraint enforced structurally at
`riir-dapps/src/kat/ledger.rs:1350`), and the USDC arm is NOT proposed to counsel;
counsel engagement rescoped to ONE narrow Tier-1 question ("may a non-transferable,
non-redeemable prepaid service credit move between two independent account holders
inside our own closed loop without reclassifying the product?"), asked when a Tier-0
demand signal exists — explicitly not a legal opinion. **Copy review no longer
counsel-gated** (068 D3 — the KAT-stake arm has no escrow window/venting/refund story
to disclose); residual engineering (extend the pinned copy set to the KAT-stake
surface: the per-tranche 7-day lockup clock, the no-promised-rate line) tracked in
068 + 061.

**Issue 060 (Proposal 008 remainder — autonomy re-open, dopamine levers, advisory
soak) RESOLVED (2026-09-04) by owner decision — file removed 2026-09-05 per the
noise-reduction rule (canonical decision record: Issue 068 — removed 2026-09-05 with
the tracker, digest in the **Issues 061 + 068** entry below; live tracker: Issue 061
(same removal); full narrative in
git history — `git log -- .issues/060_*.md`).** **Advisory-only is 008's SHIPPED END
STATE by measurement, not a waiting room**: the G5 verdict is FAIL (wash), measured
and pinned with a regression pin asserting the verdict; Bench 004 axis-E adds that no
agent-reachable `decay_factor` keeps mining alive to a 1000-epoch horizon at launch
scale. The v2-candidates plan NOT commissioned (068 D4) — it re-opens on a
MEASUREMENT (the devnet soak showing a frozen constant leaving value on the table),
never on a calendar; commissioning today would have needed an owner supply-ratio
decision on no data. **Dopamine levers (T3.1 FairRoll jackpot table, T3.2 streak
bonuses) WON'T-DO, not deferred** (068 D5): the guard pins `REWARD_EV_EPSILON_BP = 0`
("EV ≤ base exactly") + `REWARD_MAX_MULTIPLIER_X = 10` make a zero-EV jackpot a
presentation layer over a constant, and gambling-adjacent mechanics/copy on a token
just decided non-transferable and closed-loop add exposure to the surface 005 caveat
#15 calls most screenshot-likely; the guard-side caps STAY. **T4.1/T4.2 autonomy ramp
CLOSED** (terminally foreclosed by the G5 verdict; re-opens only inside a future
proposal). **T2.3 advisory-mode soak UNBLOCKED and re-homed to Issue 061** behind the
devnet ceremony rung (T2.0) — also the measurement that could re-open the v2 plan.
Landed record retained: T0.1–T0.3 scaffold/bounds/telemetry, T1.1–T1.4
signals/selection/guard/simulator, T2.1–T2.2 advisory transport + `dao-audit`
artifacts, T2.4 shallow-DAO voting (G9-GOV), the decay-bound resolution with the
bound unchanged (riir-dapps `3577986`, Issue 006), and Proposal 010's ledger re-open
landed end-to-end.

**Issues 061 + 068 — the KAT product loop (rollout tracker + the launch-tier decision
record) RESOLVED 2026-09-05 — engineering complete; every remaining row is an OWNER act
with a named unblock — both files removed per the noise-reduction rule (full text in
git history — `git log -- .issues/061_*.md .issues/068_*.md`; by-number citations
remain valid).** 068 was the owner decision record; its digest survives here.
**Tiers:** Tier 0 (devnet + testnet, operator-owned accounts only, no fiat, no custody)
AUTHORIZED, needs no counsel; Tier 1 (KAT rail beyond operator accounts / the operator
loop on mainnet) gated on one narrow counsel question + 005 caveat #7's Phase-1 exit
list, asked when a Tier-0 demand signal exists; Tier 2 (live USDC custody, live
Stripe/Solana rails) NOT PROPOSED — counsel + measured demand, re-opens on a real
inbound USDC deposit intent. **D1** ship the custody-free KAT-stake arm, do not propose
USDC. **D2** ops signer WON'T-BUILD-NOW (Tier-2-only, highest blast radius; accepted
cost: if Tier 2 lands, its longest-lead item starts from zero). **D3** copy review
needs no counsel — DONE 2026-09-05 (one shared gate predicate
`kat_stake_client::caveat15_violation` drives the module tests AND the integration
binary; the 4-item disclosure checklist pins the stake copy set; `$`-adjacent and
promised-rate canaries FIRE; 21/21 + 9/9). **D4** advisory-only is 008's shipped end
state; v2 plan not commissioned (re-opens on a measurement, never a calendar). **D5**
dopamine levers WON'T-DO. **D6** autonomy ramp foreclosed. **D7** the devnet ceremony
rung DONE (riir-deployer `73159a0` — five-conjunct permit, four steps through the real
binary, `devnet.toml` authored, 12 gate tests). **D8** the advisory soak on devnet
EXECUTED 2026-09-05 (riir-dao `e7b1f89`: the `dao-soak` driver + first run — 16/16
epochs clean, breaker NORMAL every epoch, 16/16 provenance-verified, `dao-audit verify`
OK, byte-identical re-run, $0 cap-0 guard-only; **live GLM leg blocked by credential
scope** — both CF tokens valid for the account but 403 on `ai/models/search`; owner
unblock = an AI-scoped token + re-smoke, then `dao-soak --epochs 16` live ≤ $0.02;
record: riir-dao `.benchmarks/004_d8_devnet_advisory_soak.md`). **D9** create
`gist-rs/cargo-heal` — STILL OPEN, the one owner act that blocks a shipped surface
(without it both install one-liners are unrunnable and nobody can install `cargo-heal`
at all): `gh repo create gist-rs/cargo-heal --public --description "cargo-heal — binary
releases"`. **061 at removal:** Phases 0/1/3 COMPLETE; Phase 2 T2.1 + Phase 4
T4.1–T4.3 + T4.4a steps 1–4 PASS live (step 5 = the refund-leg design pick, owner,
Plan 079); T2.2/T2.3/T4.4 testnet = owner acts sequenced after D7's clean rehearsal.
**Standing owner asks, in unblock order:** (1) D9's one command; (2) an AI-scoped
`CF_API_TOKEN` → `dao-advisory --smoke` → the live soak; (3) the testnet ceremony
(T2.2/T2.3/T4.4); (4) the T4.4a step-5 refund-leg pick; (5) Tier 1/2 rows behind
demand.

**Issue 069 (four duplicate doc numbers — two work streams, one counter) RESOLVED
2026-09-05 (file removed per the noise-reduction rule; owner GO given by the session
directive; riir-clippy `58e7c1d` + katgpt-rs `29d95956`).** T2 executed per the
citation-weight arbitration (katgpt-rs `scripts/citation_weight.py`, every site read):
`.plans/015_batch69_unaligned_word_load` → **084** (the kat phase0 substrate keeps 015,
10 vs 4), `.plans/026_dirty_encoding_write_elision_batch72` → **085** (the kat redaction
plan keeps 026, 8 vs 3), `.research/083_elo_selection_gate` → **135** (the oMLX distill
keeps 083, 12 vs 2), `.research/086_mnt_io_room_list_dod_batch73` → **136** (the
dockerfile inventory keeps 086, 27 vs 4); highwaters `.plans` 83→85, `.research`
134→136 (the issue's measured 80/132 were stale at write time — 081–083/133–134 had
been taken by later batches; the write-time re-scan rule working as designed). **17
citation rewrites** across 9 files (the 13 listed + 4 stragglers caught by the
both-names grep, incl. one live `**Plan:**` link); dated NUMBERING-FLAGS queue-log
records left as history per the katgpt-rs Plan-587 precedent. T3 recorded (§Numbering
Discipline, option (a) — one counter per directory, enforced). T4: katgpt-rs
`scripts/numbering_drift_floors.txt` riir-clippy ratchet `4 → 0`; sweep:
`riir-clippy files=233 dup=0 stale=0 malformed=0`. Side-findings left for their owners:
`riir-mmorpg-examples` `.issues/.highwater` carries the `-n 97` corruption (the
katgpt-rs Issue-725 `echo -n` class — the sweep's remaining exit 1) + its files floor
35→36; riir-train (13) / seal-game-editor (12) duplicates sit at their ratchets by
design. **RESOLVED 2026-09-06:** the mmorpg allocator was repaired upstream in the
owning repo (mmorpg `adc8877` — `-n 97` → `097`, value- and padding-preserving, now
git-tracked; a same-unit duplicate repair from the 4090 box yielded cleanly at the
rejected push, the katgpt-rs Issue-665 class working as designed) and the floors row's observed
count went 35→36 (katgpt-rs sweep commit). riir-train/seal-game-editor remain at
their ratchets (branch split / read-only).

**Issue 070 (ThoughtComm α-fusion + rerank diversity guard, arXiv:2510.20733)
RESOLVED 2026-09-05 (file removed per the noise-reduction rule; `906b152`).** **T1
measured — the α-fusion arm is dead on arrival for rule-id fusion, by measurement not
assumption:** through the healer's own fan-out (`LatentMatcher::suggest_span`, pool 12)
over 129 spans (28 composite + 101 Issue-017 labeled; domains Clippy+Perf+Docker),
cross-domain rule-id overlap is **0/129** and byte-identical pattern-text overlap
**0/129** (per-domain corpora are disjoint by construction and were authored
independently); instantiation co-presence (α≥2 domains both firing proposers) is
**2/129 = 1.6%** (both witnesses: Clippy `needless_range_loop` × Perf
`vec-with-capacity` on `Vec::new()`+push-loop composites — already co-present in the
merged pool). T2 **closed with the numbers recorded, never built** (RRF's rank
information is strictly richer; a concept-level join would need a hand-authored mapping
table = new corpus structure, a non-goal). T3 **landed** — the diagnostic-only
rerank-pool collapse detector: `rule_family` (leading `-`/`_` segment),
`CollapseVerdict`, `detect_collapse`/`detect_collapse_rule_ids` in
`src/draft/instantiation_rerank.rs`, surfaced as `EvalReport::collapsed_pools` + a
print section (prints `none` when empty — default output unchanged); G1a
planted-collapse DETECTED, G1b real-composite-pool diverse control PASSED, no-crash
gate; lib 1193→1200 (+7), clippy 0, retrieval floors unchanged (selection
bit-identical). T4 foreclosed (its reopen condition was "only if T2 lands"). Standing
measurement harness: `cross_domain_agreement_rate_report` (3-process stable). The
paper's consensus≠accuracy discipline (Appx C.2) survives in the collapse detector — a
converged pool with the oracle outside it is now a first-class diagnostic instead of an
invisible Issue-024 recurrence.

**Issue 067 (Trace-as-State growing-input reopen — arm G2) RESOLVED 2026-09-05
(file removed per the noise-reduction rule; `e053671` + `910a285`; record:
[Bench 068](.benchmarks/068_issue067_arm_g2_trace_as_state.md) + git
history).** Bench 051's cross-rule/condition-LAST negative was RECALIBRATED, not
overturned: with the two Research-530 (arXiv:2609.02702) deltas applied —
**same-rule-only channels** (the `{lint}:` boundary-safe retrieval prefix + a
family-scoped pattern fold over the row's own `lint_key`) and
**condition-first placement** (channels between the instruction header and the
code module — `[z, C]` dominated `[C, z]` in 26/27 paper cells) — the arm
G2 A/B on llama-3.3-70b-fp8-fast (25 samples, planted 4×3 Certified scratch
store via the kept-reproducible `examples/plant_067_store`) **PASSes the
pre-registered gate exactly**: reachable-miss strict-keep 6/10 vs arm B 5/10
(the one flip: `reachable:redundant_clone` vetoed → clean keep — a COVERED
lint, channels live), zero kept-wrong both arms, prompt tokens +9.9% (≤ 2×
bar). Honest caveats carried in the bench: the margin is exactly the bar on a
thin n, exemplars are constructed (real fix semantics, hand-varied shapes —
not organic store history), and B-before-G2 ordering (the scoring axis is
deterministic clippy findings, not latency). **The growing-input class is
REOPENED for the healer** — Bench 051's "second negative closes this class
permanently" clause does not fire; this was the first properly-placed
negative-to-positive cycle. Production adoption: NO flag flip — the fixer
service's prompt builder gains the same-rule channels only when its store has
fired keys for the queried lint (the T1 trigger log line is the per-lint
green light); until then the channels degrade to arm B byte-identically,
which this bench measured as safe. Ops note recorded in the bench: the CLI
default model has drifted to glm-5.3-flash (401s under both credential paths);
the working path is the wrangler OAuth bridge refreshed OUTSIDE a directory
carrying `.env` (wrangler auto-loads it and masks OAuth).

**Issue 037 (E2E self-evolve proof + adoption observability) — CLOSED
2026-08-26** (file removed per the noise-reduction rule; full session records
in git history — `git log -- .issues/037_*.md`). **ALL GATES GREEN**: A1
cadence MET on day 3 — 2026-08-24 (riir-ai `703856080` T5), 2026-08-25
(riir-ai `6ca4f00e3` P9), 2026-08-26 (riir-viewbridge `c54c658` — first
clippy sweep of that repo, 6 edits / 5 Certified, tests count-identical
102/0/3 + a sibling riir-train heal) — commit + store evidence both, judged
with the audit script's word-boundary grep fix (a bare `--grep=heal`
substring-match on "health"/"healthy" commits had fabricated a false 3rd day
earlier the same day; caught + fixed in the sibling's in-flight script WIP —
the honest 3-day evidence stands independent of it). E1–E5 + A2/A3 were
already MET (E5 2026-08-25: P9 closed — ruliology/player.rs verify-revert
cluster root-caused, proposer guard `64820c1` + score-bench v2 fixture;
narrative: git history). **T7 discharged — Proposals 003 + 004 UNBLOCKED**
(their Blocked-by lines updated in the same commit). KAT (`Proposal 005`) Phase 0 unblocks with
this close.

**Issue 071 (docker tier-2 buildcheck gate red — a registry-blocked box misread
as "Docker 29.7.2 removed the check") RESOLVED same day 2026-09-06 (file
removed per the noise-reduction rule; record:
[`.docs/11_domains/docker.md`](.docs/11_domains/docker.md) header + git
history).** The filing's T1 premise was WRONG, and the correction is the
valuable half: direct stream-split probes on the same daemon (Engine 29.7.2,
buildx v0.36.1) proved the check set had NOT drifted — `LegacyKeyValueFormat`
still fires (`FROM scratch + ENV RUST_LOG info`), findings ride stdout, the
`ERROR:` resolve-failure summary rides stderr, stdout empty on failure. The
real failure: the three check-only fixtures carried `FROM alpine:3.20`, so the
tier-2 oracle paid a registry round-trip per call; on this box Docker Hub
resolves DeadlineExceed (~60 s × 6 calls ≈ the recorded 399 s red) →
`Unavailable` verdicts, which the gate honestly refuses. The filing recorded
the verdict as `OracleSilentOnBuggy`; that arm is not reproducible under the
current daemon, and the 399-s-each determinism fits the Unavailable path
exactly — the Issue-723 lesson (repair the instrument first, decide the
disposition second) re-proven, since every originally-proposed disposition
(re-pin cross-ref / re-arm syntax / retire rule) would have been wrong.
Repair landed: (1) the three tier-2-only fixtures now ride `FROM scratch` —
BuildKit-frontend verdicts with no DL code, so the base contributed nothing to
any oracle and hadolint never checks them (tier-1 skips non-DL incumbents) —
the tier is registry-free, **399 s red → 2.7 s green 4/4 on the same daemon**;
(2) `buildcheck::classify()` (pure, unit-tested daemon-free) gained the
measured completion-marker backstop — no `Check complete` on stdout lands
`Errored` even with no `ERROR:` line, so a future buildx stream reroute can
never produce a fake `Ran(empty)`; (3) the gate's failure output names
`docker version` and separates could-not-check from disagreement, with the
misdiagnosis recorded in the message itself. Validation: lib docker 26/26,
tier-2 4/4 (2.74 s), tier-1 hadolint 3/3, refusal 13/13, t5 10/10, g4 4/4,
clippy 0 both states.

## Guard-landing protocol — full record incl. the measured story (was AGENTS.md lines 3526–3583)

## Guard-landing protocol — a guard is not landed until the residue is counted

Landing a proposer guard stops the healer **emitting** a broken class. It does
nothing about what the un-guarded healer already emitted and that already
landed in consumer repos. The cycle used to end when the fixture + positive
control went green here; that left every swept consumer carrying whatever came
before, with no record of how much.

**Measured, not hypothetical.** The `227637e` guards (`manual_is_multiple_of`
on ambiguous `{integer}` receivers, `redundant_closure` over a deref coercion)
landed 2026-08-30 and were **correct** — every one of the 32 sites riir-ai
found the next day would have been suppressed by them. They were *pre-guard
residue*. One day separated the landing from an audit that found 99 errors
across 50 targets, 32 of them these classes.

**The rule (mining-queue item P-a, closed 2026-08-31).** On landing a guard for
class C, run the sweep and paste its table into the landing note. Fix or file —
never silently leave it unmeasured.

```bash
scripts/consumer_residue_sweep.sh <lint> [<lint> ...]        # the screen
scripts/consumer_residue_sweep.sh --grep '<regex>' <lint>    # narrow first
```

Target set: `scripts/swept_repos.txt` (derived from the BOUNDARY.md workspace,
git worktrees excluded — the file documents how to re-derive it). Exits
non-zero when residue exists.

**The screen is the claim** (item P-b): `cargo clippy --workspace
**--all-features** --all-targets --keep-going`. Not `cargo check` — both
guarded classes are accepted by check's typeck and rejected by clippy's, so the
healer's own `--verify` compile gate is blind to them *by construction*. Not
default features — all 32 sites sat behind non-default ones, and the
immediately preceding fix of the same E0689 class recorded `--workspace
--all-targets: exit 0` while 16 more live sites existed at that very commit.

**Two traps the script was caught by, and now handles** — both of which
reported "clean", which is why it prints a per-repo diagnostic total as a
liveness witness:

1. **`cargo clippy` on an up-to-date build emits nothing.** The first version
   reported 0 sites for `riir-neuron-db` while a live `manual_is_multiple_of`
   sat in `src/`: the units were fresh from an earlier run, so no lint pass ran.
   A quiet zero from a cached build is indistinguishable from a clean repo.
   Handled by `cargo clean -p <each workspace member>` inside a sweep-local
   `CARGO_TARGET_DIR` — local crates re-check, dependency builds stay warm, and
   the repo's own target dir is never touched (a concurrent session there is
   neither slowed nor invalidated).
2. **The JSON shape is `"code":{"code":"clippy::<lint>"}`** — not
   `"code":"<lint>"` — and `byte_start` *precedes* `file_name` in the span
   object. Guessing either backwards also returns a confident zero.

Verified against ground truth before use: 1 `manual_is_multiple_of`, 0
`redundant_closure`, 1 `unused_assignments` on `riir-neuron-db`, stable on a
warm second run. The `unused_assignments` case is the de-dup working — the same
site is reported once per compilation unit under `--all-targets`, and identity
is `(file_name, byte_start)`.

## Plan 001 multi-domain architecture — full record (was AGENTS.md lines 3639–3665)

### Plan 001 multi-domain architecture (2026-08-13)

Plan 001 generalized the framework from a clippy-only healer into a pluggable
multi-domain framework behind the [`HealDomain`](src/domain.rs) trait. The
three domains (clippy_lints, rust_perf, kernel_opt) share the same
L0→L1→L2→L3→L4 pipeline; each domain owns its corpus + validator. **Proposal
012's GPU kernel optimizer is absorbed as the `kernel_opt` domain** (Phase 3)
— the user's correction ("why it can't be clippy?") overrode Proposal 012's
"new repo" recommendation. The trait abstraction makes future extraction a
move, not a rewrite.

**Cross-domain auto-routing** — Plan 005 Phase 1/2 (2026-08-14) shipped
the unified `cargo heal --suggest` CLI with `--domain auto`, revising the
original "not in scope" stance: `auto` **fans out across all
feature-enabled domains** with the router verdict as a per-span
annotation, NOT a gate (empirical finding — the 8-D structural centroids
mis-routed fixtures, starving the correct domain; re-gating via tuned
centroids or the 768-D embedder is a future tuning option). Each domain is
still feature-selected; requesting a feature-off domain is a hard error.
Since Phase 6 the bin also runs **`--fix [--write] [--quiet]`** — the
span-preserving edit-splice emitter + the rust_perf + structural-clippy
proposers (T6.1/T6.4), hardened by the T6.2 G5 real-file run on
katgpt-rs/katgpt-pruners (Bench 019 §G5 addendum: 3 proposer bug classes
caught by the compile gate + fixed at root cause; 12 files / 20 edits
landed, compiled + 1233 tests passed; katgpt-rs `8c331cd9`). Kernel
proposers remain Phase 5 (deferred — concept-grade corpus).

## `kernel_opt`-specific layers — full record (was AGENTS.md lines 3666–3685)

### `kernel_opt`-specific layers (Phase 3)

The `kernel_opt` domain adds two layers on top of the shared pipeline:

1. **`KernelExpertRouter`** (`src/domains/kernel_opt/router.rs`) — wraps
   `katgpt_core::variable_rank_domain_expert::pick_domain<15, 8>` (katgpt-rs Plan 558).
   Activity vector = kernel signature `[M, N, K, dtype, hw, is_attention, is_fusion, is_ir_transform]`
   (8-dim — Plan 004 extended from 5 to 8 dims by adding 3 operation-class
   one-hot axes, resolving the Plan 003 C+O unreachability); domain directions
   = per-category centroids (Plan 003: hand-crafted centroids wired to the
   88-rule corpus via `KERNEL_CATEGORIES`; **15/15 categories reachable**;
   mining via `CommittedFieldBlend` deferred). The `build_activity()` helper is
   the consumer-facing constructor. Modelless argmax — zero allocation.

2. **`KernelThermalRouter`** (`src/domains/kernel_opt/thermal.rs`) — the
   FlashAR/QGF thermal-tier pattern: Plasma→Hot→Warm→Cold→Freeze escalation.
   `ThermalTier` is `#[repr(u8)]` (1 byte, sync-efficient). The scaffold
   returns Plasma for all calls (modelless-first mandate); Warm/Cold/Freeze
   require 4090 CUDA + LoRA (deferred to riir-train).

## Feature Flag Discipline — full record (was AGENTS.md lines 3760–3809; per-feature batch narratives + corpus-growth history)

## Feature Flag Discipline

- `default` = `["clippy_lints"]`: clippy lint healing domain + L0 (corpus) + L1 (ternary drafter) + L2 (native rust-analyzer pruner)
- `clippy_lints` (default-on): the clippy lint healing domain.
- `clippy_verify` (opt-in): Phase 3 lint resolution verification via real `cargo clippy` re-run. Slow (~130ms/verify), requires cargo+clippy on PATH. Implies `clippy_lints`.
- `rust_perf` (default-on since 2026-08-13, Plan 001 Phase 2): rust performance healing domain. **94 rules** (Batch 62 ->67; Batch 64 ->69; Batch 66 ->71; Batch 67 ->72; Batch 68 ->73; Batch 69 ->74; Batch 65 ->76; Batch 73 ->78; Batch 74 ->80; Batch 75 ->83; Batch 78 ->88; Batch 83 ->89; Batch 85 ->91; Batch 86 ->94) (Issue 014 / Bench 020 expanded 46→50; Batch 48 / Research 054 expanded 50→58 with the 8 online-search candidates; Batch 53 / Bench 038 expanded 58→62 with the 4 online-search candidates; Batch 59 / Research 066 expanded 62→63 with the first EXTERNAL-source distill via the `distill` skill; Batch 61 / Research 068 expanded 63→65 with the 2 oxc-article rules) from the `rust-optimize` skill checklist + nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs profiling guide (Research 032). Promoted after Bench 013 GOAT G1+G5 PASS; re-confirmed by Bench 015; Issue 012 (Bench 016) added 4 sorting/IO/copy rules; Issue 013 (Bench 017) added 3 data-structure-choice rules; Issue 014 (Bench 020) added `regex-hoist-lazylock` (5443-5675× — the corpus's largest win), `arc-str-payload` (5.4-11.4× + 42× bytes), + the 2 NotBenchable metric-mismatch rules (`tokio-guard-across-await`, `bounded-channel-backpressure`). Batch 48 (2026-08-16, Research 054) added 6 more ParetoDominant rules (`collect-append-to-extend` 8.5-12.8×, `vec-insert-front-deque` 7.8-115.3×, `vec-macro-zero-fill` 32-2505×, `unwrap-or-else-lazy` 1.9-2.3×, `hashset-insert-bool-return` 1.19-1.31×, `filter-map-fuse` 1.37-1.64×) + 2 honest annotations — `filter-count-to-any` ParetoDominated 0.82-0.90× (the folk-wisdom refutation: LLVM VECTORIZES the count loop while `any`'s early-exit forces scalar iteration — `any` wins only for ≲10%-position hits), `iter-copied-deref-map` Mixed (0.80× at 1M on sum folds). The `perf_bench_demo` gate runner accepts `PERF_BENCH_FILTER` (comma-separated rule-id substrings) for batch-scoped runs. Batch 53 (2026-08-17, Bench 038) added 4 online-search candidates — 2 ParetoDominant (`distance-sq-no-sqrt` 1.21-1.24×, `hashmap-with-capacity` 2.4-3.1× with allocs 12→1 — the rehash ladder re-SipHashes all entries per resize) + 2 honest annotations (`powi-vs-powf` all-Tie refutation: LLVM lowers powf(2.0) identically to powi(2) in release — the folk wisdom only holds for debug profiles / exponents ≥ 3; `soa-hot-field` latency-bound Mixed: a serial f32 add-reduction hides the layout win, SoA pays on throughput-bound field consumers — count/min-max/transform). **16 of 73** rules carry `context_notes` (11 verdict-class: 4 ParetoDominated + 7 regime-fragile/Mixed; 2 metric-mismatch-class — consumers should treat as context-dependent). Batch 65 (2026-08-22, Research 081, the hotpath.rs profiler distill) added the 2 contention rules — both measured ParetoDominant, both regime-annotated: `align-64-false-sharing` (concurrency; **4.7-9.4× at 2-12t, 1t honest tie**; **the beyond-source finding: `align(64)` is INSUFFICIENT on 128B-line Apple Silicon** — `hw.cachelinesize`=128 there, 64-B-strided slots leave even-index pairs sharing [0,128) lines, recovering only 1.16-2.44× of the 3.3-10.8× full-separation win — pad to the ARCH's coherence granularity or `crossbeam_utils::CachePadded`) + `channel-send-batching-unpark` (concurrency; 1.16-1.29× + allocs 3232→1624 @100K — the honest margin is pipeline-overlap: a 1P/1C harness hides the receiver's park/unpark latency off the critical path, the source's 6.25× needs the pair ON it; **flush with `mem::replace`, never `mem::take`** — take-regrow pays ~7 allocs/batch and measured WORSE on the alloc axis than the buggy arm). Annotated now **20 of 78** (11 verdict-class + 2 metric-mismatch + 7 regime-caveat incl. the 4 new measured-winner notes — a new annotation class accepted by the drafter token check). Batch 73 (2026-08-26, Research 085/086, the mnt.io Matrix-RSDK room-list DoD distill — Plan 029) added 2 ParetoDominant rules: `sort-key-cache-precompute-out-of-lock` (caching; **24.3-37.2x across the sorts-per-update axis** with the amortization curve visible in the axis, allocs 164497→17 @16 sorts — comparator lock+clone per invocation → maintained sort-key cache refreshed once per update; REGIME: sorts-per-update ≈ 0 = pure refresh overhead, annotated; the source's 78× is END-TO-END, our isolated mechanism reads 24-37×, never cite 78×; **bench-protocol discovery: a plain reverse() is ONE detectable run for the adaptive driftsort (~512 comps at N=512 vs ~5100 via fixed Fisher-Yates) — the comparison count is the axis the rule lives on**) + `lock-guard-narrow-field-accessor` (concurrency; **11.5-12.7x**, allocs 100000→0 @100K — whole-payload clone under read-guard → narrow accessor returning only the consumed sub-field; composes with the cache rule — production `RoomListItem.refresh_cached_data` reads through narrow accessors). Both suggestion-only (type-blind for a parse-only proposer). Batch 74 (2026-08-26, Research 088, the simd-lite/simd-json SWAR distill — Plan 032; the Research 085 pool-drop RE-PROBE: the prior fetch failure was MCP zread indexing, not the repo) added `swar-digit-chunk-parse` (arithmetic; **ParetoDominant 2.35×/1.75×/2.52× @ 8/12/19 digits with the honest 1.01× tie @ 4** — the source's own README regime note; per-digit `val = val*10 + (b-b'0')` serial chain → the SWAR 8-digit fold [magic 2561/6553601/42949672960001] guarded by the `0x3333` digit validation, Muła ← simdjson ← simd-json provenance cited in-source; the exhaustive gate caught a `u64::from_le` vs `from_le_bytes` transcription bug pre-landing) + `runtime-feature-detect-per-chunk` (dispatch; **MEASURED Mixed 0/0/3 ties on rustc 1.97 aarch64** — the std::detect machinery const-folds to near-zero through an `#[inline(never)]` boundary; the source's cost class [outlined `target_feature` calls per 8-digit group] survives for outlined dispatch/non-literal strings/heavier probes — the honest-annotation record, the B68 varint precedent). Both suggestion-only. Batch 75 (2026-08-26, Research 089, the GreptimeDB prom-read conversion distill — Plan 033, found via idle-loop online perf search; single-function article with criterion 100-sample numbers, grade A-) added the 3 grouping/borrowing rules: `encoded-layout-borrowed-view` (allocation; **ParetoDominant 41.98/39.38/29.95× at R=10K/100K/1M, allocs 4,000,004→0 + bytes 156 MB→0 @ R=1M/S=100/L=4** — the dictionary-expansion trap: a generic to_string accessor silently re-expands a compact encoding, "the 3 strings become 1000"; fix = the per-layout borrowed-view enum `Option<&str>`; REGIME S≪R; never cite the source's 35% anomaly) + `group-keys-borrow-allocate-on-first-sight` (allocation; **ParetoDominant 1.92/2.27/2.37×, allocs 70,133/700,133/2,100,133 → 8,019 FLAT at S=1000 — the 7R+2S→8S+30 model EXACT**; the decision content: std `entry()` APIs require an OWNED key, forcing per-row key materialization — the manual hash index `HashMap<u64, Vec<usize>>` + FULL compare + first-sight clone + one final sort is the escape; R·L copies become S·L; 3 correctness contracts encoded incl. the collision candidate-list + prefix-trap unit gates; mirror regime of `btreemap-sorted-iteration`, which gained the completing rider — the two-row sorted-read-frequency table) + `sorted-input-previous-row-fast-path` (complexity; **Mixed 1/0/1 — adjacent (run=100) 12.92× FASTER, interleaved (run=1) 0.99× tie** — the hit-rate regime; "correctness does not depend on it", the hash fallback is the contract; DERIVED isolated bound ~2.1-2.8×; family note resolves the tension with the early-exit-removal rules). All 3 suggestion-only. Annotated now **28 of 85**. GOAT: 696/0 default + 714/0 bench-state lib + clippy 0 both states `--all-targets` + full CI guard ALL LAYERS + retrieval floors HELD (perf@1 96.2→97.9 band — corpus growth improved perf retrieval; 14/14 green) + verdict-stable ×2. Lane B audit verified all quoted numbers (premise corrected: riir-train DOES carry optional arrow/parquet — the entries are std-only regardless; the 36.2%/8.9% profile lines are the PR author's secondhand, flagged). **Landing lesson (the generator-lesson class): a pseudocode_parses gate is necessary but NOT sufficient — Lane A's buggy snippet carried a TYPE error syn cannot see; the bench compile is the only type-check new snippets get.** Batch 76 (2026-08-27, Research 090, the Meilisearch LMDB/vector-store distill — Plan 034, found via idle-loop online perf search; 16-min CTO-level engineering article, production-verified on a 19M-embedding customer trace) added 2 ParetoDominant rules: `debug-stat-full-scan-out-of-hot-path` (complexity; **168.91×/1929.27×/5851.00× at N=10K/100K/300K, B=100/op** — the mean-degree sanity statistic full-scanned the vector store per insert batch: "Removing this reduces the time from 559s to about 2–3s and brings the complexity down to O(n)"; a diagnostic eagerly computed in a hot path is a hidden O(collection) tax that GROWS with store size — fix = remove/gate/maintain-incrementally; G1 unit gate: the maintained aggregate matches the scan EXACTLY on mean-bits) + `eager-pointer-collect-lazy-fetch` (caching; **1903.70×/5414.62×/21353.90× at N=100K/300K/1M, K=1,000 touched, allocs 1→0 + bytes 35.7 MB→0** — the ImmutableItems/ImmutableLinks full-scan pointer map "polluting the cache with useless, unnecessary disk pages just by iterating over all of them" → FrozenReader's lazy on-demand fetch, "only the necessary pages are loaded"; building an everything-index to serve a subset pays the scan TWICE — time + cache eviction; REGIME K/N ≪ 1 one-shot, repeated-same-subset wants the persistent-index rule instead). Both suggestion-only. Pool drops: the Qdrant Gridstore article (C+ for rule-mining — architecture piece; its dense-sequential-keys→Vec candidate KILLED at dedup adjacent to o1-lookup + dedupe-hashset-vs-vec) + the TLS read-txn pooling (architectural) + the LMDB C patch itself (upstream API). Batch 77 (2026-08-27, Research 091 — the lucebox PR #625 rope.cu distill, Plan 035's crossover) added `runtime-int-exponent-powi` (arithmetic 7 -> 8; **ParetoDominant 2.57x/2.81x/2.66x @ 1K/100K/1M on M3 aarch64 — CROSS-ARCH RE-GATED same day on x86_64-pc-windows-msvc (4090 box, same harness, verdict-stable ×2): all-Tie 0.99/1.00/1.00 — the win is the slow-libm-host regime, an idiom swap on fast-libm x86 (the newton-refinement inversion class, Batch 62); amended in the annotation; 0 allocs, rel-error gate <1e-12** — the runtime-integer-exponent case `powi-vs-powf`'s annotation explicitly deferred: a runtime exponent gives LLVM no instcombine bailout so `powf` STAYS a libm call in release; GPU motivating evidence: the source's RDNA4 RoPE kernel 692 -> 76 µs/launch, 9.1x). GOAT: 696/0 default + 714/0 bench-state + clippy 0 both states `--all-targets` + retrieval floors 14/14 HELD + verdict-stable ×2 + G1 equivalence gates. **Generator lesson: the edit_file JSON layer strips one escape level (`\\\"` → bare `"` silently un-terminating Rust string literals) — string-heavy corpus entries go through python here-docs with explicit rustlit() escaping, never the structured-edit tool.** Batch 78 (2026-08-27, Research 092 — the Datadog RTDB/Monocle external distill, Plan 036, found via idle-loop online perf search; blog-only source, fixtures original) added 2 rules, 86→88 (layout 7, concurrency 8): `narrow-counter-overflow-promotion` (layout; **MEASURED ParetoDominated 0.64-0.66x, honest-annotated** — the memory-for-speed trade: the alloc axis halves EXACTLY (8N→4N bytes, 1 alloc each side) but update-heavy streams pay 35-55% for the per-update `is_empty` guard + `checked_add` branch; the flat ratio across 100K/1M/4M = branch-bound not bandwidth-bound; the trade PAYS in memory-footprint-bound systems — Datadog's motivation was the 2,100 GiB DDSketch infeasibility, never speed; G1: totals bit-identical incl. u32-boundary crossings + the all-promoted regime; distinct from `usize-index-to-u32` — domain-bounded index fields vs runtime-escape-hatch value counters) + `cache-line-block-partitioned-lock` (concurrency; **ParetoDominant 1.12/3.91/2.98/3.03x at 2-12t, verdict-stable x2** — one `Mutex<Vec<u64>>` → 512 aligned `Mutex<[u64;8]>` blocks; ~8x fewer lock-collision opportunities; DIFFERENTIATED vs `align-64-false-sharing` in-rule: that pads independent slots against FALSE sharing, this partitions the TRUE sharing of one array at lock granularity — they compose; **the first ALLOC_AXIS_EXEMPT rule**: contended macOS pthread_mutex allocates nondeterministically in the slow path (probed: global-mutex 12t = 41/44/42/41/41 allocs across runs; uncontended = exactly 513 for 512 locks) — the exemption lives in bench.rs `ALLOC_AXIS_EXEMPT` with the probe evidence, a documented platform class not a harness bug; TWO harness compile-class catches en route: `std::thread::Scope` takes TWO lifetime params + is invariant over 'scope — the spawn-body-inline shape (the align-64 convention) is the only smooth form, helper fns with `&Scope` params fight the borrow checker). Annotated now **31 of 88** (29 + the B78 pair: one ParetoDominated honest-loser + one measured-winner regime note). GOAT: 703/0 default + 721/0 bench-state lib + clippy `--all-targets` 0 both states + FULL CI guard ALL LAYERS + retrieval floors 13/13 HELD (blended 92.1/96.0/97.0) + verdict-stable x2 both rules. Batch 83 (2026-08-28, Research 097 — the arXiv:2608.26368 SIMD bit-slicing predicate distill, Plan 044, the FIRST queue-11 arXiv-listing-walk pool consumed after the MCP search quota) added 1 rule + 1 rider, 88→89 (simd 7→8): `bitplane-transpose-repeated-predicate-scan` (simd; **ParetoDominant 1.96/2.16/1.93x at 10K/100K/1M, 64 filter scans/call, verdict-stable x2: 2.08/2.20/1.96x** — encode-once bit-plane transposition + zero-decompression bitwise predicate evaluation for repeatedly filtered integer columns: the two-mask MSB→LSB recurrence [gt |= eq & plane / eq &= plane] is pure AND/OR word ops, b/32 the bytes, 64 values/word op; **the honest ~2x is the LAYOUT half — Rust's scalar arm auto-vectorizes [~0.126 ns/elem vs the source's non-vectorized Java 0.65], never cite the source's 10.7x NEON headline isolated**; regime gates: amortization [encode O(N·b) once, source break-even 7-25 scans, single-scan/write-once OUT — the source's own 1M-row single-pass cells ran sub-1x], b≤16, range predicates win / wide-type equality loses [source 0.5-0.7x], IN-lists ≤~k=8 [k·b passes], no point lookups; **DOMAIN CONTRACT: the predicate constant must fit the column width — the G1 gate caught the unguarded form marking ALL 513 rows greater on an out-of-range threshold [c=8 vs b=3]; the fix is one branch per QUERY [paper §3.5 bias transform]**; data-oblivious side property: plane throughput is selectivity-constant) + the `branchless-conditional` doc rider (the source's scalar loop swung ~4.5x with selectivity on branch prediction [5.4K→1.2K M elem/s] while the plane form stayed constant <6% — external measured evidence for the rule's regime). Drops: the string/dictionary lane [Java-baseline-inflated — Rust memcmp is already SIMD], JVM memory-economics, FOR/bias as standalone [collapses into narrow-field rules], lo==0 skip [clippy flags unsigned comparisons]. Batch 85 (2026-08-28, Research 098, the arXiv:2608.23841 cflow "Pipeline-Native Transformers" distill — Plan 046; executed by the idle loop in the quiet window after the sibling's docker batch) added 2 rules, 89→91 (layout 7→8, io 3→4): `weight-tile-consumption-order` (layout; **MEASURED ParetoDominant 4/0/0 — 1.32x/1.35x/1.35x/1.56x at n_in=4K/16K/32K/64K, verdict-stable x2, allocs 1/1 + bytes equal at every size, G-det stable x3** — row-major single-token matvec re-walks x per output row → weights repacked ONCE at load time into consumption-ordered 64×512 tiles, the x slice L1-resident per tile, TILE_ROWS× less x traffic; the win GROWS monotonically with working set; M3's 128 KB P-core L1-d shifts the source's Sandy-Bridge crossover 4× right; **the honest surprise: even the L1-resident arm (row+x = 32 KB ≪ L1) measured 1.32× — the research's "expected Tie" was wrong-direction, annotated weaker-not-absent (the paper's own 1×-L1 cells ~1.12×)**; rel-error G1 gate over 3 shapes incl. 2 ragged; composes with transposed-weight-register-gemv [tile for cache, transpose within for registers]; suggestion-only — a parse-only proposer cannot prove the repack is load-time, a mid-loop repack would regress) + `software-prefetch-storage-bound` (io; **the honest annotation — SW prefetch on a bandwidth/storage-bound stream is useless-to-harmful: source A/B PF ±0.1% at 64 MB, PF=1 4% WORSE at 4.7 GB, the dedicated prefetch thread burned ~48% of decode time walking expert regions at stride 256, disabling it alone = 1.92× (their single largest decode win)**; two-regime [compute-bound CAN hide latency]; x86 source-cited only — no stable aarch64 prefetch intrinsic, the powi-vs-powf cross-arch precedent; the aarch64 PRFM arm DEFERRED on the asm-friction clause; proposer-less by design, the filter-count-to-any rationale). **The drafter annotation-vocabulary contract test CAUGHT the first note draft missing verdict tokens — the contract fires on NEW annotated rules (the check works as designed).** Landing-lesson (the B37/edit_file class): the corpus category-count COMMENTS are plain `//` inside a fn body — an edit whose new_text used `///` prefixes produced E0658 "attributes on expressions" (doc comments are illegal on statements) + fuzzy matching clipped prefixes on 2 more lines; 3 fix rounds; use plain `//` in fn-body comment blocks and re-grep after multi-line comment edits. GOAT: 770/0 bench-state + 752/0 default lib + clippy 0 both states `--all-targets` + retrieval floors HELD 13/13 zero re-pins + G1-G5 + G-det PASS (guard run backgrounded: nohup + log-poll — the full ci_feature_guard takes >60 min after a corpus edit [L3 --each-feature recompiles the lib ~32×], two foreground runs timed out). GOAT: 718/0 default + 736/0 bench-state lib + clippy 0 lib both states + all-targets clean in batch files (the 11 remaining = the sibling's pre-existing gp_acquisition set) + retrieval floors HELD (clippy@1 89.6 identical, perf@1 94.3 blend@1 92.1 — gate PASS, the two perf misses top at OTHER rules) + verdict-stable ×2 + G-det stable (allocs 0/128 — the fixed arm's two scratch masks per scan, hoistable). Annotated now **37 of 94** (Batch 86: +2 measured ParetoDominant + 1 NotBenchable-speed size axis). Count pins walked incl. B77's STALE benchable header (74 → true 78; the header had been left at 74 when the test moved to 76 — fixed to 78/88 at the header, the (10/88) Not-benchable line, and the seventy_eight test rename). Batch 59 (2026-08-18, Research 066, the thermite distill) added `division-by-invariant-multiplication` (arithmetic; **2.03-2.13x ParetoDominant on M3 aarch64**, 3/0/0, allocs 0/0 — loop of `x / d` with RUNTIME-invariant `d` → precomputed libdivide-style magic multiplier + shift; CONSTANT divisors are already LLVM strength-reduced, the regime is exclusively runtime-invariant `d` applied ≥~3x; x86-64 DIV at 30-90+ cycles should see a wider gap; the exhaustive equivalence gate caught TWO pre-landing bugs in the ported construction — floor-vs-ceil magic at d=3 x=3 + u32-wrap carry loss at d=7 x=u32::MAX, both fixed via ceil magic + widening add; proposer deferred, the architectural rewrite is a follow-up candidate). Batch 61 (2026-08-18, Research 068, the oxc distill - the first plan-FIRST batch, Plan 007) added 2 ParetoDominant rules: `keyword-map-to-match` (dispatch; **5.23-5.58x** - static-keyword `HashMap<&str,V>` dispatch -> plain match on string literals; LLVM lowers string-literal matches to a length-switch, the map pays SipHash per lookup; the oxc gperf refutation via `--emit=llvm-ir`; static key sets only, proposer deferred - key-set extraction) + `usize-index-to-u32` (layout; **1.81-1.99x** - usize index/offset fields in high-volume structs -> u32, the oxc Span 16B-to-8B change; twice the elements per cache line, wins even L1-resident; domain must bound below 2^32; proposer deferred - type ripple).
  **Batch 48 follow-up (2026-08-17): the three deferred proposers LANDED** - `hashset-insert-bool-return`, `collect-append-to-extend`, `vec-macro-zero-fill` are `--fix` eligible (`FIXABLE_RULES` 5->8) with fixture + clean-span + cross-domain gates; two syn facts learned: a type annotation wraps the let pattern (`Pat::Type` around the ident), and `LitFloat::base10_digits` keeps `.`/`e` separators (`"0.0"` digits are `"0.0"`); `filter-count-to-any` stays proposer-less BY DESIGN (ParetoDominated - a losing rewrite never gets a `--fix` path).
  **Batch 53 follow-up (2026-08-17 evening): the two Batch 53 ParetoDominant proposers LANDED** - `distance-sq-no-sqrt` (`(x*x+y*y).sqrt() < r` → `(x*x+y*y) < r * r`, either operand order; sum-of-squares receiver gate + pure-path/literal threshold gate; negative-threshold + 1-ulp boundary caveats documented) + `hashmap-with-capacity` (`HashMap::new()` + first-mention for-loop over a len-able iterable → `HashMap::with_capacity(iter.len())`; capacity-is-a-hint semantics make any pre-loop len safe). `FIXABLE_RULES` 8→10. `powi-vs-powf` stays proposer-less BY DESIGN (ALL-TIE 1.00x in release — zero gain gets no `--fix` path, the filter-count-to-any rationale applied to a tie) + `soa-hot-field` stays suggestion-only (architectural). Two implementation facts learned: field access `s.rows` parses as `Expr::Field` not `Expr::Path` (len-bases need a field-chain gate for `self.items` shapes), and the reversed-form emit order must be `t * t OP sos` (`t OP t * sos` parses as `t OP (t * sos)` — `*` binds tighter than comparisons; caught by the `reversed_operands_fire` fixture).

  **Batch 86 (2026-08-29, Research 100 — the Cloudflare 1.1.1.1 DNS-cache memory distill, Plan 047; found via online perf search; blog-only source, original fixtures) added 3 rules + 2 riders, 91→94 (allocation 20→22, layout 8→9): `serialize-once-byte-buffer-store` (allocation; **ParetoDominant 4.26x/3.52x/2.90x at 1K/10K/100K, verdict-stable x2 (run 2: 4.26/3.47/2.87), alloc axis EXACT 200,001 → 2 @ 100K — 2n+1 → 2, the malloc storm IS the mechanism; bytes 7.90 MB → 4.70 MB; G-det stable; the win SHRINKS with n as the copy dominates the per-alloc overhead, still dominant everywhere** — frozen records as per-field heap fields → ONE length-prefixed byte buffer + u32 offset table, copy-out rebuild; REGIME: sequential-rebuild consumers win, in-place random-mutators lose; ANTI-SHAPE: the source's own rejected full-wire-message form recorded in-rule) + `context-equal-field-elide` (allocation; **ParetoDominant 3.12x/3.01x/2.00x at the ~80% context-equal mix, verdict-stable x2, allocs 100,001 → 20,056 — the ~20% non-equal owners keep theirs, the elision regime made visible; bytes 4.04 MB → 2.76 MB** — a stored field that equals the enclosing context (the lookup key) becomes Option<Box<T>> None, resolved at read; two-regime note: low %-equal is pure overhead, unmeasured below ~50%; composes with serialize-once; distinct from interning/dedup — nothing shared, the value is ABSENT because the context re-derives it) + `bitflag-pack-when-footprint-dominates` (layout; **NotBenchable-speed — the deterministic size axis 12 → 8 pinned by the new corpus gate** — stored-at-scale bool cluster → one bitflag byte; the padding-collapse caveat (measure size_of before/after, never promise); CROSS-CITES the kernel_opt B32a `bool-fields-over-bitset-hot-reads` INVERSE regime). RIDERS: `box-str-immutable` generalizes to Box<[T]> for any frozen T (the source's 8 frozen fields/entry = 64 B/entry, >15 TB fleet-wide); `box-large-enum-variant` gains the honest costs (allocator size-class rounding, locality scatter, the RARE variant pays more; the ~80/20 skewed-frequency regime). GOAT: 809/0 default lib + 827/0 rust_perf_bench lib + clippy 0 both states `--all-targets` + retrieval floors 13/13 HELD (no re-pins) + verdict-stable ×2 both benchable rules + G-det stable; the 4 new gates: batch86_rules_present, batch86_pseudocode_parses, batch86_byte_buffer_store_reconstructs_exactly (G1 byte-exact incl. empty-name/empty-payload/multi-byte-length edges), batch86_bitflag_size_axis_and_roundtrip. Count pins walked incl. the STALE corpus-header line (said 89/79/10 — B85 never bumped it; now 94/82/12). Source: blog.cloudflare.com/dns-cache-memory-optimization-1111 (2026-08-27); cite only the near-isolated +13% insert / −5% lookup. **Batch 91 (2026-08-31, Research 106 — the Rustdoc filter-first distill, Plan 052; noahlev.org "Making Rustdoc 33% faster" + rust-lang PRs 159623/159779/159854; mine-time verification included reading the PR 159623 diff — the guard is un-normalized raw-ty-data, the post-filter + the entire Deref-chain machinery deleted) added 2 rules, 94→96 (iterators 9→10, complexity 17→18): `filter-before-expensive-transform` (iterators; **ParetoDominant 3/0/0 — 11.85x/11.30x/6.61x at 1K/30K/300K, verdict-stable x2 (run 1: ~12x/10.67x/8.28x — ratio jitters, verdict does not), alloc axis EXACT 300,014 → 18,764 (n+14 → 6.25%·n+14, the ~94% discarded renders ARE the mechanism), bytes 2.77 → 1.65 MB** — map(expensive).filter(relevant) → re-express the predicate on the RAW input and filter first; win ∝ discard-fraction × transform cost; IMPOSSIBLE when the predicate is not re-expressible on the input (the source's own precondition); over-permissive re-expression is correctness-safe when consumers tolerate extras; composes with filter-map-fuse, adjacent to eager-pointer-collect-lazy-fetch (access path vs pipeline order)) + `decision-projection-before-construction` (complexity; **ParetoDominant 3/0/0 — 24.58x/24.38x/24.37x, verdict-stable x2, alloc axis EXACT 300,000 → 0, bytes 9.60 → 0 MB** — transform-then-project to decide → compute the INVARIANT projection on the raw input, pay the canonicalization only behind the decision (PR 159854's head-before-param_env, −18%); invariance precondition pinned AS A TEST; distinct from ok-or-else/unwrap-or-else-lazy and filter-map-fuse). GOAT: 992/0 default + 1010/0 bench-state + 1091/0 latent + 1136/0/8i triple + clippy 0 self-crate `--all-targets` both states + retrieval floors HELD (no re-pins) + fixture-purity gate clean + verdict-stable x2 + G-det stable; 4 new gates (present/parses/filter-order-identity+render-count-axis/projection-invariance+canonicalize-count-axis); count pins walked (96/84/12/39/57 across header + 6 test sites + builder doc + list bullets); independent subagent verification ALL 5 PASS (annotation numbers proven exact-to-the-byte — the +14 shared Vec-growth events derivation).
- `rust_perf_verify` (opt-in): `cargo test` re-run for perf rule verification. Slow (seconds), requires cargo on PATH. Implies `rust_perf`.
- `rust_perf_bench` (opt-in, Plan 001 Phase 2 GOAT gate): generated microbenchmarks at multiple input sizes → Pareto verdict + the **deterministic alloc-count axis** (Issue 014 / Bench 020: a counting `#[global_allocator]` in every generated crate reports exact per-call alloc count + bytes — bit-identical across repeat runs on 40/40 benchable rules; timing stays the verdict axis, alloc gates Tie-class candidates). **GOAT G1+G5+G-det PASS (Bench 013/015/016/017/020)** — 37/40 benchable rules Pareto-dominant (o1-lookup 570x, bsearch-sorted-slice 16-2709x, simd-chunk 3.9x, bool-to-bitpack 7-11x, branchless-conditional 10x, static-str 42x, vec-with-capacity 1.1-4.3x, format-concat-loop 42-95x, mutex-to-atomic 2.5x, f32-counter-to-int 7.5-11x, entry-vs-contains-insert 1.5x, loop-fusion 1.5x, collect-avoidance 2.3-3.0x, dyn-to-enum 2.5-2.9x, prefix-sum-2d 34-30000x, ok-or-else 97x, swap-remove 3-7927x, chunks-exact 13-16x, fxhashmap 3.3-5.8x, retain-inplace 1.3-2.0x, mem-take 1.9-2.6x, avoid-clone 1.3-3.0x, box-str-immutable 1.1-1.3x, box-avoid 9.6-42x, smallvec-stack 27-75x, buffered-io 99-132x, log-write-direct 1.9-2.0x, sort-unstable 1.08-1.25x, bufwriter-batched 2.48-21.57x, vec-remove-front-to-vecdeque 3.65-1270x, btreemap-sorted-iteration 3.73-6.91x, cow-conditional-alloc 1.32-1.49x, **regex-hoist-lazylock 5443-5675x** (allocs 1.3M→0), **arc-str-payload 5.38-11.38x** (bytes 42×)); 2/40 honestly dominated (precompute-table: compiler optimizes popcount to POPCNT in release; papaya-lockfree: benefit needs multi-threaded contention); 1/40 Mixed (sort-by-key). Closes Proposal 001 caveat 3. Implies `rust_perf`.
- `kernel_heal` (opt-in, Plan 001 Phase 3): GPU kernel optimization healing domain. Absorbs Proposal 012. Enables `katgpt-core/variable_rank_domain_expert` for the `pick_domain` router (katgpt-rs Plan 558). 15 category experts (Plan 003), 8-dim activity vector with 15/15 reachable (Plan 004), routed heal pipeline composed in `kernel_heal_demo` example. 4090 LoRA + real popcorn corpus DEFERRED.
- `kernel_verify` (opt-in): CubeCL compile check. **STUB** — delegates to shared `RustAnalyzerPruner`; full numeric compare deferred (Proposal 012 §Phase 0). Implies `kernel_heal`.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (katgpt-rs Bench 572) without a code-level dep. Runs once per lint invocation (~400µs/10 candidates), not per-tick. Implies `clippy_lints`.
- `latent_retrieval` (opt-in, Plan 005 Phase 1): real-file span-level latent retrieval — the `--suggest` MVP. Indexes every enabled domain's corpus via riir-rag (`RuleIndex`: `ShardIndex` Clifford-wedge KNN + BM25, multi-scale), routes spans via corpus-derived domain centroids (`DomainRouter`, annotation only), retrieves ranked rules (`LatentMatcher`, fan-out across enabled domains). The suggest path itself has ZERO proposer code — suggestions are the corpus pattern texts; the proposers live in each domain's `proposers/` module (rust_perf Phase 4, clippy T6.4) + the shared `src/edit_emitter.rs` substrate, gated by the proposers' own Cargo features. Gates `dep:riir-rag` (heavy tree: riir-rag + transitively riir-neuron-db — the default build stays light). Powers the `cargo-heal` bin (`--suggest` / `--fix` / `--domain auto|clippy|perf|kernel` / `--top-k`). **GOAT G1–G4 ALL PASS (Bench 018, 2026-08-14):** G1 100% (12/12 top-1, target ≥80%), G2 349µs/chunk (<1ms, 2.9× headroom), G3 104 lib tests, G4 `SuggestScratch` stable across 50 repeat scans. Known limits: fan-out routing (no gate), no minimum-score threshold (every span suggests), 12-item G1 fixture.
- `rustc_errors` (**DEFAULT-ON since 2026-08-29, owner call** — Plan 019; all GOAT gates passed at Bench 045, promotion hold lifted by the owner): the 4th domain — `cargo heal --fix-compile`, oracle-driven compile-error repair keyed by rustc error code (check → playbook candidates → apply min-delta first → recheck → keep iff strictly fewer coded errors, else byte-exact revert; bounded `--max-iters`, default 5). **Driver-shaped, not a passive scan: inert until `--fix-compile` is invoked — default-on costs compile time only, zero runtime overhead** (unlike `docker`, which actively scans). Dep-tree unchanged (`compile_oracle = []` — std + serde only; the no-retrieval-dep rule stands). Default lib 828 → **900/0** (+72: the domain's unit tests join the default suite); integration targets `fix_compile*`/`floor_corpus_integrity`/`generated_corpus_spot`/`premise_probe` now run under plain `cargo test` (976/0 total); score-bench #15 flat (96.2/1.9/100/100 — the `--fix` path untouched). Installed `cargo-heal` rebuilt — `--fix-compile` live in the binary. Territory is MEASURED, not assumed (Bench 041): uncontested E0597/E0499/E0502 (cargo fix = 0%) + the E0505 aliasing-divergence opening; E0596/E0382/E0507/E0373 ship NO template (cargo fix 100% matched — delegated by the floor gate). **Phase 6 GOAT at N=212 PASS (Bench 045, 2026-08-22): uncontested stratum 105/105 fixed where cargo fix is 0/105; E0505 21/21 MATCHED where the floor's every repair silently diverges; 126/126 repaired cases pass their seeds' behavior tests post-fix (the G1 semantics pass — every corpus seed carries a ground-truth `#[cfg(test)]`); G2 p50 151ms/case + 78ms/iteration (release); G4 propose path ~1.5k allocs/pass bounded non-growing; never-wrong pinned (strict-decrease keep + byte-exact revert); T6.10 semantic notes printed in the diff for every semantics-changing template (`⚠ semantic change:` …), Box::leak never-emitted. G6/G7 ordering A/B HONEST NEGATIVE: static min-delta is already first-try-perfect on the corpus (126 iterations / 126 fixes / 0 reverts for every arm) — the T4.2 shape-conditional mechanism stays installed (flip-test pinned) with no claimed benefit; T4.6 stays per-repo store scope. Self-evolve first-class: trajectories → `.heal/` (neuron-db store under `latent_retrieval`, memory.json otherwise) + `.heal/misses/` (the authoring queue). Real use: `scripts/watch_heal.sh` (README §Watch mode). Corpus: `corpus_gen` (200 generated, deterministic, stratified) + 12 committed anchors; integrity + seed-validity + null-arm gates all green (two real corpus bugs caught on the first run: an invalid manifest name + 10 hand-counted string-length expectations).** **Issue 035 RESOLVED (2026-08-22, issue file removed per the noise-reduction rule):** the premise-probe + floor issue that forced Plan 019's thesis correction — T1 (probe, 6 tests) + T2 (floor harness, Bench 041) landed 2026-08-21; T3 (driver delegation) discharged by the Phase 3/6 landing (`77f93a9` + `c8c28c0`): every contested code now either ships a template with a measured win (E0505 → `E0505CloneAtMove`, 21/21 matched at N=212 where the floor diverges 21/21) or a recorded loss (E0596/E0382/E0507/E0373 → no template; `cargo fix` is 100% matched there — delegate, per Bench 041/045). T4 (re-measure on toolchain bumps) is STANDING, not schedulable: `tests/premise_probe.rs` fails on the bump, which IS the reminder; the stack's `rust-toolchain.toml` pins the stable channel so a bump is already a deliberate reviewed commit — re-run Bench 041 in it. Probe re-verified 6/6 green at close. **1.98.1 re-verify (2026-09-05, pin bump `4c96c64`): the probe PASSED UNCHANGED** — rustc 1.98 moved neither the contested/uncontested split nor any replacement; the full Bench 041+045 surface re-measured bit-stable (floor 50.5% / uncontested 0/105 / E0505 100%-fix-100%-diverge; driver 126/212, uncontested 105/105, G1 126/126; G2 p50 156 ms under sibling load) — addendum §4 in Bench 041.
- `docker` (**DEFAULT-ON since 2026-08-28**, Plan 030 / Proposal 006 — both halves of the promotion rule met: GOAT PASS **and** measured consumer pull, riir-chain): Dockerfile bounded-fix healing as the **5th** retrieval domain, and the first with **no `syn`** (a hand-written stanza scanner) and the first whose oracle is an **external binary** (`docker_verify` → hadolint). Scope: build-cache perf, image size, package hygiene, secrets, PID1/signal correctness, multi-platform, Rust-toolchain idiom. NOT compose/k8s, NOT GitHub Actions (zizmor), NOT image scanning (trivy/grype), NOT base-image version auditing (renovate) — and never a registry network call. **Phase 0+1 GOAT PASS (2026-08-28):** 27 rules — 14 Class-A (hadolint detects, we fix; the agreement gate re-runs the LIVE binary and every DL-coded rule agrees) + 13 Class-B (both incumbents silent), 14 with bounded fixes. **Mine-time falsification did real work:** 2 rules authored as Class B turned out to have live incumbents the Research 086 snapshot missed (`arg-secret-to-secret-mount` → DL3064, `entrypoint-shell-form-signal-block` → DL3025 covers ENTRYPOINT) and were RECLASSIFIED; a third apparent overlap was REFUTED by direct probe (`apt-update-install-same-run` vs DL3009 was a fixture artifact — on a split whose update RUN already deletes its lists, hadolint is entirely silent). 3 candidates killed for having no bounded fix (DL3008/DL3018 version pinning, DL3006 untagged base — the version is a registry fact and we make no network calls); DL3022 killed because hadolint FALSE-POSITIVES on our own tree (it cannot see BuildKit named contexts, so it reports 2 phantom findings on `riir-chain/deploy/Dockerfile`). **Refusals are data, not prose** — `refuses()` names DL3059 RUN-consolidation (ANTI-MODERN: separate RUNs are separate cache layers), `apt-get clean`, LABEL-merging and QEMU-native→cross rewriting, and `tests/docker_refusal_gate.rs` fails the moment a proposer emits one; the ONE sanctioned merge (the adjacent apt update/install pair, a cache-CORRECTNESS bug) is asserted not to generalize. **The caller gate** keeps 5 rules silent without evidence they cannot derive from the file (committed lockfile, multi-member workspace, context listing, multi-platform invocation) — which is what keeps the negative controls clean. G1 negative control: **zero fixable findings** on `riir-chain/deploy/Dockerfile` + mmorpg `authority-container` (the latter carries 1 pinned suggest-only citation: its distroless runtime declares no USER); G2 17.05 µs/scan release; G4 scan buffers reach a steady state, a clean file allocates no findings; **G5-real PASS** — `edge-wallet-container` healed (SHELL pipefail + 2 cargo cache mounts), `docker build` GREEN with the healed RUN compiling for 167.7 s, 141 MB image, binary present. Router annotation re-measured for all four docker states and it never DROPS, twice RISES (2-domain 71→73/101; 4D+K 78→82/131) — a distinctive off-vocabulary centroid absorbs fewer generic Rust spans. En-route fix: `EvalReport::per_domain` was `[DomainMetrics; 3]` while `domain_index(Sec)` already returned 3 — a latent out-of-bounds that never fired only because no sec-labeled fixture exists; now sized by `DomainId::ALL.len()`. The Issue 020 floors block was restructured from 4 cfg-gated consts to one `cfg!`-selected fn — a 5th domain would have made it 8 consts whose declaration cfg and use cfg must match exactly, the shape that turned CI Layer 5a red on the sec landing. **T1.6 adoption (2026-08-28) — and it falsified a rule.** The consumer is `riir-chain`: both tracked Dockerfiles now carry the healer's fixes (edge-wallet SHELL pipefail + 2 cargo cache mounts + `USER 65534:65534`, measured serving `/health` as uid 65534 and now hadolint-CLEAN; deploy `cargo build --locked`), and `riir-chain/scripts/dockerfile_heal_gate.sh` keeps them there from the `standalone` CI job — a two-tier gate (hadolint against a pinned ledger; `cargo heal --fix --domain docker` for the bounded half) that fails on an unpinned finding, on a count change, AND on a pin that has gone stale, verified by injecting 4 defect classes. **The falsification:** `cargo-locked-flag` wrote `--locked` onto the edge-wallet build and broke it (exit 101 at resolve — `build.sh` stages a trimmed workspace, so the committed lock cannot be satisfied). A committed lockfile is NECESSARY but not SUFFICIENT evidence, and the distinguishing fact has no in-file signal → **demoted to suggest-only, 14 bounded not 15**. G5-real structurally could not catch it: it healed a COPY, and a caller gate reads the file's SURROUNDINGS — measure caller-gated rules on the tracked file. A `--write` lane with no build gate must not carry a rule whose failure mode is "the build no longer resolves." **Phase 2 T2.1(a) — the TIER-2 oracle, 2026-08-29.** `docker build --check` (BuildKit's own linter) is now wired into the library behind `docker_buildcheck` (implies `docker_verify` — one feature axis, so the dispatch is a two-arm `cfg` pair instead of a 4-state cross product), and `class_a_agreement` routes a non-`DL` incumbent to it. The 4 BuildKit-check claims stop being a hand-measured postscript and become a standing gate: 4/4 agree live, and it is **verified by falsification** — 3 injected defect classes each turn it red with the right verdict. Three measured facts drove the design, none documented upstream: `--progress=rawjson` carries only `vertexes`/`statuses` so **text is the only interface**; findings land on stdout while build errors land on stderr; and **a resolve failure silently TRUNCATES the finding set while still exiting 1** — a two-defect file reported one warning — so `CheckRun::Errored` discards a partial result rather than let a truncated run read as near-clean. **It stays opt-in and that is not a GOAT verdict, it is a dependency-class one**: a running daemon plus a registry round-trip per non-`scratch` fixture (0.195 s scratch vs 1.29 s cold / **2.17 s repeat** on `alpine` — the registry hit does not warm) has no business in a default `cargo test`. Never runs in the heal path. **Phase 2 T2.1(b) — the suggest-only family, 2026-08-29.** 4 rules chosen by whether they DISCRIMINATE on the real in-tree images rather than by the seed table's order: `base-tag-not-digest-pinned` · `copy-link-artifact-rebase` · `musl-allocator-perf-annotation` · `healthcheck-tool-absent-in-runtime`. **27 → 31 rules, bounded UNCHANGED at 14** — the `--write` lane gains nothing, because each rule's action depends on a fact the file does not carry. Two rules were NARROWED by measurement after being written, each caught by an existing gate: the healthcheck rule cites the TOOL not the FORM (shell-form HEALTHCHECK is DL3025, verified live; an exec-form `curl` probe on a curl-less base is unoccupied), and `copy-link` stands down on `scratch` (no parent filesystem to rebase onto — it had fired on `deploy` AND the G4 clean fixture, and a rule that fires on every well-written multi-stage file carries no information). A prediction was corrected too: `authority-container` DOES draw the digest citation, because `distroless-distro-suffix-pin` only fires on a base MISSING the `-debianNN` suffix. **The batch found a bug in the domain's model of the frontend:** `SyntaxVersion::from_directive` read a bare major (`docker/dockerfile:1` — Docker's OWN recommended form) as `1.0`, so writing the canonical directive made `missing-syntax-directive` fire on a correct file. A minor-less tag is a MOVING label = latest-of-major; measured, not inferred — under `:1`, `COPY --exclude=` (1.19-only) passes `docker build --check` and builds green. Same lesson as T1.6 from the opposite direction: there adoption falsified a RULE, here it falsified the DOMAIN'S MODEL. **Adoption same day:** riir-chain TOOK `copy-link` on edge-wallet (+ the `# syntax=` line), rebuilt GREEN 44.7 s, verified serving `/health` as uid 65534(nobody), 141 MB unchanged; DECLINED the digest citation on a security trade (frozen base + no renovate is worse than tag-pulled patches for an image holding a signing key — the row flips when a bump policy exists); marked the musl annotation NOT INVESTIGATED with the settling measurement named. Record: [.docs/11_domains/docker.md](.docs/11_domains/docker.md) + [Bench 056](.benchmarks/056_docker_domain_phase0_1_goat.md).
- `sec` (opt-in, Plan 018 / Proposal 004; default-off — promotion needs measured consumer pull, the `rust_perf` precedent): security-pattern healing as the 4th RETRIEVAL domain (after clippy/perf/kernel; `rustc_errors` is driver-based). Scope: bounded-fix source patterns only — FFI panic containment, Drop-glue panic safety, raw-pointer null hygiene, the `from_raw_parts` len-zero contract, Rc backedge leaks. NOT dependency auditing (cargo-audit/cargo-deny), NOT a lint engine (dylint), NOT judgment-bearing detection. **Phase 0+1 GOAT PASS (Bench 046, 2026-08-22):** 6 rules shipped, 5 of 11 candidates KILLED at mine-time falsification with real-clippy probe evidence (the full table in the bench) — `improper_ctypes*` owns the extern-block declaration side (killed 2), 3 more had no bounded fix; the 2 measured surprises: `improper_ctypes_definitions` is SILENT on extern-fn definitions with reference params on 1.93 (the ANSSI territory is open) + `uninit_assumed_init` misses the let-bound uninit form. 4 conservative proposers (catch-unwind wrap for unit-return extern fns; ptr-null-check let-else prologue for Option-returning unsafe fns; Drop `.unwrap()`→`.unwrap_or_default()`; from_raw_parts len-zero guard); 2 suggest-only by design (signature-changing ref→ptr; constructor-rippling Rc→Weak). Boundary gate `tests/sec_boundary_gate.rs` runs REAL clippy with the banned restriction list over the fixtures — no banned lint may fire MachineApplicable (a clippy auto-fix in our territory FAILS the gate → the rule retires); the one allowed overlap (`unwrap_used` fires on the drop fixture with NO suggestion) is pinned. Negative control: dry `--fix --domain sec` over riir-viewbridge's 24 files = 0 edits (its `riir_ffi!` surface is already contained); positive control 3 edits end-to-end. G3: default 657/0 unchanged, sec 687/0, sec+kernel+latent 953/0, clippy `--all-targets` 0 both states; retrieval floors HELD + sec self-variants 100% top-1; router annotation re-pinned for the 4-domain state per the Issue 020 contract (44.6% 4D / 59.5% 4D+kernel — annotation is display-only, retrieval is what gates). En-route fix: `SpectralVocab`'s hardcoded `[u32; 3]` domain arrays → `Vec` (the 4-domain build panicked on the pre-existing limitation). Record: [Bench 046](.benchmarks/046_sec_domain_phase0_1_goat.md) + [Research 079](.research/079_sec_domain_phase0_1.md) + [.docs/11_domains/sec.md](.docs/11_domains/sec.md). **Phase 2 LANDED 2026-08-29 (Batch 88, owner-directed; Research 104): corpus 6 → 11 — `unsafe-impl-send-sync-unsound-field` (Rudra's unsound-impl class; suggest-only — the fix is an Arc/Mutex redesign), `ffi-export-panicking-body` (the VALUE-return complement of Phase-1 catch-unwind; suggest-only — sentinel choice is semantic), the dep-gated crypto trio `weak-hash-to-blake3` / `password-hash-argon2` (suggest-only — output-type/API changes) + `constant-time-secret-compare` (the 5th proposer: `==` → `.ct_eq(..).into()`, strict-gated on a subtle import + secret vocabulary in the enclosing-fn or operand names + plain-ident operands + test-mod skip). Probe-first: all 5 candidates OPEN vs real clippy 1.97 full groups (zero equivalent-fix firings). Boundary gate extended with the crypto deps in the probe manifest (compile errors are not diagnostics) + 2 new pinned allowed overlaps (`indexing_slicing` / `undocumented_unsafe_blocks`, both fixless). En-route: the corpus-lockstep gate caught the R5 fixture missing its subtle import (the dep gate is load-bearing — G1 doing its job); the kebab-case corpus gate relaxed to allow digits (the rust_perf convention, `usize-index-to-u32` class). G-gates: sec 939/0, sec+kernel+latent 1267/0/2i, all-features 1599/0/15i, clippy 0 both states, retrieval floors 14/14 HELD + sec self-variants still top-1 (11 rules), dep-tree unchanged (cargo tree diff empty), E2E verified (2 edits on a scratch victim — both secret compares healed compile-gated, the non-secret `a == b` untouched). Phase 2 of Plan 018 complete; T2.2 (Miri oracle) + T2.3 (20-rule bar) stay `[-]` with reopen conditions. **P8 intake LANDED 2026-09-01 (Plan 057): corpus 11 → 12, new `PanicSafety` family** — `nan-unsafe-sort-comparator` (`partial_cmp(..).unwrap_or(Equal)`/`.unwrap()` in a sort comparator → `total_cmp(..)`; NaN ordered deterministically instead of equal-to-everything, the std-sorts-panic-on-inconsistent-order class since 1.81; strict-gated: sort context + exact chain + `Equal`-only default + comment guard; arg verbatim — `PartialOrd` pins `(&self, other: &Self)`, the exact shape `total_cmp` takes, so the rewrite is signature-compatible and only non-`total_cmp` receivers E0599 → verify-reverted). Probe-first (zero clippy coverage on the shape, 1.97 full groups); the 6th FIXABLE proposer; boundary gate 2/2 green with the new fixture auto-covered; retrieval floors HELD incl. sec self-variant top-1; score-bench run #24 flat (96.2/1.9/100/100). The P8 verdict's type-blindness concern (float_order desc/asc + f32/f64 twins) dissolved by `total_cmp` — recorded in the intake close-out. ≈300 sibling library sites are now addressable via heal sweeps with `--features sec`.
- `self_evolve` (DEFAULT-ON since Issue 018, 2026-08-16; Bench 023): latent-first self-evolution loop — `LatentFixMemory` (3-axis retrieval over fix trajectories) + `LatentSkillEvolution` (EMA direction-vector update) + `EvolveRecorder` (the oracle-split standalone recorder for the `cargo heal --fix` path). GOAT G1-G5 PASS (G5: 70pp healing rate improvement in controlled fixture). **G6/G7 validated against real cargo clippy feedback (Issue 008): G6 10/10 corpus truthfulness PASS, G7 50.0pp improvement (50% → 100%) PASS** (re-validated 2026-08-16 post-promotion at the 40-lint corpus scale after the Issue 022 corpus-truthfulness fix — G6 40/40, G7 50%→100%, Bench 024). The modules are UNCONDITIONAL — the memory half runs wherever fixes are applied (+ persists across runs via the v1 JSON envelope); the EMA learning half no-ops with one log line without the `clippy_verify` oracle. The feature flag is a default-on back-compat alias (required-features lists). Opt-out: `RIIR_HEAL_SELF_EVOLVE=0`.
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc)
- `l4_lora` (opt-in, riir-train Plan 336 T5.2/T5.3): the **L4 generative fallback seam**. Ships the composition SEAM ONLY — trait + pipeline wiring, no model, no GPU, no heavy deps — and is **inert unless a fallback is installed** via `HealPipeline::with_l4_fallback`. Surface: `pub trait L4Fallback` (`src/l4_fallback.rs:56`), `HealPipeline::with_l4_fallback` / `has_l4_fallback` (`src/self_evolve.rs`), `impl L4Fallback for DaemonL4` (`src/l4_daemon.rs:186`, the `bonsai_clippy_l4_eval --serve` transport client). Contract enforced by `tests/g4_l4_composition.rs` (**13 tests**).
  **When L4 is consulted — the reachability contract.** L4 sees *only* misses, never a working modelless answer:
  1. **miss #1** — `heal()` has no candidate at all.
  2. **miss #2** — the best candidate does not parse.
  3. **miss #3 (relevance, Issue 030 — resolved, commit `d8440f6`)** — the best candidate is a memory trajectory for a rule the corpus has never covered. Before this gate the serve/decline decision was `parses` alone, i.e. a **syntactic test used as a relevance proxy**; off-distribution it served valid-but-unrelated Rust (e.g. `ptr_arg` answered with `fn f() -> String { "done".to_string() }`) and **suppressed L4 entirely** — measured miss rate 0/29 (Bench 037). With the gate: out-of-corpus 9/9 decline, in-distribution unchanged, addressable gap **9/29** (Bench 037).
  A parsing **and relevant** modelless fix is never replaced by L4 (`a_parsing_modelless_fix_is_never_replaced_by_l4`), and Stage-2 hits return before the gate, so the Issue 010 path cannot regress (`stage2_hit_never_reaches_l4`).
  **NOT promoted, and promotion is not close:** riir-train Plan 336 G1 is a measured **FAIL (0/60**, tied with the frozen-backbone control, Bench 465) and T2.7's re-gate repeated **0/60 DEGENERATE** (Bench 467); G2 is also a measured FAIL (Bench 463, ~191 s/fix). The bar is now concrete: on the 9 reachable cases an adapter must beat **decline**, and declining is a *correct* answer — a wrong-but-parsing L4 fix is worse than the `None` the pipeline now returns. **Serving-lane decision (owner-delegated perf/sec, 2026-08-28): local Bonsai stays opt-in + UNARMED (demote-loser); decline stays the default; the CF llama arm waits on the owner's leak-posture sign-off — no code change, decision recorded in `.docs/10_self_evolve/l4_fallback.md` §5.**
- `clippy_lora` (opt-in): **DEAD PLACEHOLDER — superseded by `l4_lora`.** Declared at `Cargo.toml:168` but referenced by no code anywhere in the tree (verified 2026-08-17). The L4 work ships under `l4_lora`. Kept only to avoid breaking any external `--features clippy_lora` invocation; do not add code behind it.
- `contrastive_scope` (opt-in, Bench 040 / Plan 016, 2026-08-20): the **input-scope gate** at the `heal()` seam — the Bench 669 consumer adoption of katgpt-core's `contrastive_scope` primitive (Issue 674 / Research 493). `ScopeModel` (`src/scope_model.rs`) wraps the primitive's `ContrastiveScoreTable` + `ScopeGate` with a deterministic sorted-vocab build over the domain corpus (in) vs `OUT_OF_SCOPE_DOCS` (18 canonical non-Rust docs, out) + a zero-alloc `&str`-slice tokenizer. Installed via `HealPipeline::with_scope_model`; inert without a model (byte-identical). The 2D complement of the Issue 030 rule-coverage gate: out-of-scope input (`D > θ`) declines the whole heal (never serve, never burn the ~48 s L4 call); in-scope inputs get the epistemic haircut on `evolution_score` — **bit-identical in-distribution** (κ pinned so f32 sigmoid saturates to exactly `1.0` at the worst corpus fixture). Measured (Bench 040): OOS 8/8 declined vs 8/8 served un-gated (seeded memory — the production state); 529 ns/input (release); steady-state alloc delta −1; L4 0 calls on garbage. κ/θ re-pinned from the measured gap (θ=0, in-margin 7.6 bits / out-margin 24.4 bits; κ=4.5, saturation 34.3 ≥ 16.6). **Honest scope:** Rust-vs-not-Rust garbage ONLY — cross-domain Rust discrimination stays out (Issue 020's ~70% lexical ceiling); garbage in an uncovered language family scores neutral and passes; short mixed snippets are the documented gray zone. **Katgpt-core promotion:** the primitive went DEFAULT-ON in katgpt-rs the same day per Bench 669's T5 rule (consumer adopted + gates pass). Gates: `tests/scope_gate_adoption.rs` (10 tests incl. anti-vacuity + counting-allocator G4).
- `ternary_inference` (opt-in, Bench 010): katgpt ternary generation on clippy prompts. The pre-existing compile error (`attn_sub_on` not found in riir-gpu) was **FIXED** in riir-ai commit `4bfe3b30` — the cfg gate on `attn_sub_on` + `ATTN_SUBSTAGE_MASK` + `ATTN_SUBSTAGE_ALL` was relaxed from `all(cubecl_runtime, ternary_gemm_batched)` to just `cubecl_runtime` (behavior-preserving: the mask defaults to all-on). `--all-features` CI layer 4 now passes clean.

The modelless path (L0-L3) is PRIMARY. LoRA (L4) is the conservative last resort,
and "last resort" is enforced in code, not by convention — see the three-condition
reachability contract under `l4_lora` above. The load-bearing lesson from Issue 030
generalizes beyond L4: **a syntactic check (`parses`) is not a relevance check.** Any
future stage that decides "did the modelless path succeed?" must test whether the
answer pertains to the input, or it will silently serve confident nonsense
off-distribution and starve whatever fallback sits behind it.
The self-evolution layer (Issue 004) is additive — it composes on top of L0-L2
without modifying the frozen pipeline. `rust_perf` was promoted to default-on
2026-08-13 (Bench 013 GOAT G1+G5 PASS) with the 2 ParetoDominated rules
carrying `context_notes` annotations ("demote loser" per global rule). The
annotations now propagate end-to-end through the drafter via a non-breaking
default method `HealEntry::context_notes() -> Option<&str> { None }` →
`DraftedFix::context_notes` (2026-08-13 post-promotion wiring). The drafter
surfaces the annotation; the consumer decides whether their context matches.
`kernel_opt` is NOT promoted to default-on until its GOAT gate (real corpus +
full numeric validator) passes.

## Full text of remaining sections compacted in AGENTS.md (operative boilerplate — archived so nothing removed is unpreserved)

### Boundary contract — full text (was AGENTS.md lines 5–22)

## Boundary contract — read `BOUNDARY.md` first

[`BOUNDARY.md`](BOUNDARY.md) is the authoritative per-repo contract: what this
repo **owns**, what it **does not own** (with the correct home for each), the
crate-granular **allowlist** of what it may depend on, links to the cross-repo
rules' one canonical home, and the **drift ledger** of known gaps. On any
conflict with prose in this file, BOUNDARY.md wins.

- **Domain test:** is this a **developer tool** (clippy healing, codegen) with zero game-domain coupling in the default build? NO → it belongs in another repo; file there.
- **Read it before** adding any dep, crate, module, System impl, or vocabulary
  type — and before assuming a concern is yours to implement.
- **Enforcement** is not prose: `../riir-ai/scripts/ci_boundary_contract.sh`
  fails on an undeclared cross-repo dep, on a drift row without its open issue,
  and on a contract row that no longer matches the measured graph. Run boundary
  checks VIA the `boundary-guard` skill, not as ad-hoc greps.
- **Found a violation?** File the issue FIRST (`.issues/NNN_boundary_*.md`), add
  the drift row, then fix. Closing the issue removes the row in the same commit.

### Why this is a separate repo — full text (was AGENTS.md lines 3613–3637)

## Why this is a separate repo (not in riir-ai)

The code healer borrows the **drafter+pruner+corpus pattern** from
`riir-games-quest/quest_grammar` (TernaryDraftModel + SealQuestCorpus +
ConstraintPruner), but has **zero game-domain coupling**:

- No dependency on `riir-games*`, `riir-game-sdk`, or any game crate —
  from ANY feature combination
- No dependency on `riir-chain`; `riir-neuron-db` only transitive (via
  `riir-rag` under the opt-in `latent_retrieval` feature)
- Default build: only public dep is `katgpt-core` (ConstraintPruner trait
  + ternary SIMD matvec + `pick_domain` for kernel_opt's expert router,
  katgpt-rs Plan 558). Two opt-in feature-gated arms consume non-game sibling
  crates where reimplementation would duplicate whole substrates:
  `ternary_inference` → `riir-engine` + `riir-gpu` (Bench 010 inference
  arm); `latent_retrieval` → `riir-rag` (Plan 005 retrieval substrate,
  `cargo heal --suggest`)
- The `TernaryDraftModel` pattern is reimplemented locally (~200 LOC), not
  imported cross-workspace

Putting it in riir-ai would violate the domain boundary: riir-ai is for **game
runtime + engine**, not developer tools. The prior rejection of extracting
riir-games from riir-ai (Proposal 017 era) was correct because riir-games has
heavy bidirectional coupling with riir-engine. Code healing has no such
coupling — it's a clean, self-contained developer tool.
### CI enforcement — full text (was AGENTS.md lines 3740–3758)

### CI enforcement (2026-09-02, katgpt-rs Issue 701 R2 self-apply)

`.github/workflows/rust.yml` now runs `scripts/ci_feature_guard.sh` — the
same guard a developer runs, not a CI-only subset. Triggers are deliberately
narrow (the workspace spending-limit convention): **weekly on `develop`**
(the rot check — the default branch IS the working branch here), pushes to
`main`, and `workflow_dispatch`. The `standalone` job pins the
escaping-path-dep set (`scripts/standalone_dep_gate.sh`: 9 entries across 3
siblings — katgpt-rs public; riir-ai + riir-neuron-db private via
`SIBLING_REPOS_TOKEN`, hard-erroring when the token is absent, the riir-chain
Issue 028 expiry-alarm pattern). Runner notes: the toolchain installs from
rust-toolchain.toml (never @stable — the single-toolchain-across-8-repos pin
is deliberate); the Tier-2 `docker build --check` agreement degrades
deterministically in CI via a dead `DOCKER_HOST` (daemon + registry from
shared runner IPs is a flake factory — local runs own the live-oracle
coverage); the guard's layer-5c wedge budget is raised to 7200 s for the
4-core runner. First scheduled runs are the cost measurement: budget ~300 min
(30-feature `--each-feature` layer + the all-features GPU tree, floating
sibling HEADs).
### Documentation Shape — full text (was AGENTS.md lines 3810–3840)

## Documentation Shape

This repo ships a **11-folder numbered `.docs/` book** (opened 2026-08-11,
Issue 007; the 11th folder was added for the Plan 001 domains), matching the `riir-ai/.docs/` / `riir-neuron-db/.docs/` /
`riir-game-sdk/.docs/` format: numbered folders for sort order, unnumbered
`.md` files inside, each folder has a `README.md` index with a purpose
blockquote + file table + "See also" cross-refs.

| Folder | Scope |
|---|---|
| `01_orientation/` | Crate role, sibling layout, boundary rationale, multi-domain architecture (Plan 001), build commands, numbering |
| `02_corpus/` | L0 rule distillation: `Corpus<D>` (generic) + BLAKE3 commitment + per-domain seed corpora |
| `03_drafter/` | L1 ternary SIMD matvec: `TernaryDrafter` (generic over `E: HealEntry`) + BLAKE3 `embedder` + Plan 005 latent retrieval (`RuleIndex`/`DomainRouter`/`LatentMatcher`, `--suggest` MVP, Bench 018 GOAT G1–G4) |
| `04_pruner/` | L2 syn validator: `RustAnalyzerPruner` + `ValidationResult` + honesty contract |
| `05_clippy_verify/` | clippy lint resolution: `ClippyVerifier` (opt-in `clippy_verify`) |
| `06_ruliology/` | L3 Pareto-front: `RuliologySearch` + `pareto_front` + `ast_delta_size` (opt-in) |
| `07_examples/` | The 8 in-tree examples (`heal_demo`, `perf_heal_demo`, `perf_bench_demo`, `kernel_heal_demo`, `clippy_verify_demo`, `ruliology_search_demo`, `self_evolve_real_world_demo`, `stage2_tuning_demo`) + the `cargo-heal` bin (Plan 005 `--suggest` CLI) |
| `08_benchmarks/` | GOAT gate methodology + measured results |
| `09_lessons/` | Canonical patterns: borrow-without-dep, modelless-first, freeze/thaw, honest POC caveats, sigmoid-not-softmax |
| `10_self_evolve/` | Issue 004 + Issue 008: latent-first self-evolution loop (`HealPipeline` + `LatentFixMemory` + `LatentSkillEvolution`, opt-in `self_evolve`; G1–G7 ALL PASS) |
| `11_domains/` | **Plan 001 Phase 2+3**: multi-domain architecture — `rust_perf` (Phase 2) + `kernel_opt` (Phase 3, absorbs Proposal 012) |

The top-level `.docs/README.md` is the index. `README.md` (repo root) remains
the build surface (Status + Quick Start + Usage); `AGENTS.md` (this file)
remains the authoritative repo-local context.

For `.docs/`: files inside numbered folders have **no number prefix**.
Filenames are bare slugs (e.g. `crate_role.md`). The folder number provides
sort order; the filename provides identity. Add a new doc by dropping
`slug.md` in the right group folder and adding one line to that folder's
`README.md` index table.
### Numbering Discipline + Research-note shape — full text (was AGENTS.md lines 3842–3879)

## Numbering Discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and never
reused**. Before creating a new `.issues/` file, read `.issues/.highwater`, use
`value + 1`, write back. Same for `.plans/`, `.docs/`, `.benchmarks/`,
`.research/`, and `.distill/` (the distill-program rolling resume state —
edit-in-place per batch, NOT one file per batch; see
`.distill/001_mining_queue_snapshot.md` and the pointer section in
`.agents/skills/riir-clippy/SKILL.md`).

**One counter per directory, enforced (Issue 069 T3, decided 2026-09-05 —
option (a)).** Every author reads the directory's `.highwater` and writes
`value + 1` back in the same commit; a renumber consumes its target exactly
like a fresh allocation (the riir-ai Bench 847 rule). The discipline is
checkable, not remembered: katgpt-rs `scripts/numbering_drift_sweep.py` reds
on tracked duplicates above this repo's pinned count and on a `.highwater`
below its directory max. Note `numbering_gate.py` (the per-push half) is
katgpt-rs-scoped — its floors file is that repo's — so this repo is covered
by the sweep, not the per-push gate; the standing fix for a collision is
renumber + citation rewrite per katgpt-rs Issue 724 T2, never a silent reuse.

### Research-note shape (convention, adopted 2026-08-16)

`.research/` notes are **born-complete distillation records**, not task
trackers — no `- [ ]` checkboxes (pending work lives in `.issues/`, the
SKILL.md mining queue, or the plan the note feeds). Two header rules:

1. Every new note carries a `**Status:**` line (not `**Outcome:**` — legacy
   notes using it are grandfathered) from a small vocabulary: `RECORD`
   (batch landed, closed) · `DISTILLED — pending owner decision` (has an
   actionable unimplemented delta) · `NEGATIVE` (measured refutation).
2. A note whose delta gets filed/implemented later gets a one-line status
   update pointing at the issue/commit — the note itself stays a record.

Rationale: 36/58 historical notes have no Status line, which is benign only
because they are closed records; the two genuinely-pending notes (053, 058)
DO carry explicit Status lines. The vocabulary makes that distinction
machine-skimmable for future sessions.

### The consent heartbeat + v0.1.3 (2026-09-08, 77e26a8 + a38a473)

`cargo heal --mine`/`--unmine` now send a one-shot SIGNED tier note
(`POST /mining/consent`, `kat_consent_client.rs`, golden signing-message
dual-pinned with the service's `ConsentDoc` — riir-dapps `548d336`), so the
ai.gist.rs adoption surface can show per-machine engagement tiers. Issue 075
filed the per-run-burn verdict (NOT LGTM) and refined Proposals 008/010 in
the same commit. v0.1.3 cut via `manual_release.sh` (Actions budget still
exhausted), tap + scoop bumped by hand. TWO live lessons in one e2e:
(1) macOS shells set `HOSTNAME` as a shell variable WITHOUT exporting it —
the first tier note landed with an empty machine label; the fix (a38a473)
adds the `hostname`-binary fallback to the env cascade, and the release was
republished same-hour (zero consumers) with the tap re-pinned; brew reinstall
verified byte-identical to the release artifact.
(2) `--unmine`'s logout half (74f0865) deletes the account key the heartbeat
needs — the correct order is consent-write THEN note, and `--mine` after a
logout honestly skips with "not logged in". Final live state: m3 recorded
in/out/in across the e2e, 177-row sync accepted by the service, the free-vs-
mine split + machines table live on the front page.

### The burn fast-settle e2e + the 075 soft gate (2026-09-08/09, 84d0d73 + de729c9)

Issue 076 closed end-to-end: the client auto burn-sync landed (f3a7cf2,
the git-analogy stacking proven 3/3 through the real binary against a
std-TCP mock), the service got `POST /burn` + the hourly
`KAT_FAST_SETTLE` bucket (riir-dapps `a2648c6`, armed in prod), and the
live e2e moved all three site surfaces at once (100 KAT burned ·
10 batches/179 rows · rust_perf 117 via the Batch-135 RustForge distill,
Research 155 + Plan 114). The cross-repo follow-ups closed 2026-09-09:
riir-dao `dd7d75a` (premise docs + the new
`fast_settle_bucket_contract_is_pinned`), the G2 bench riir-dapps
`23e0d47` (PASS/GO — hourly leg 0.03% of the crank budget at one year
of buckets; `.benchmarks/008`), and the two pre-existing kat-service
wasm32 clippy failures `4681084` (`AdoptionView::empty()` was dead under
wasm32 because every caller is `#[cfg(test)]` — the lane now green under
`-D warnings`, 83/83).

The 075 zero-balance decision landed as the soft gate (de729c9): v2 burn
ack carries `last_balance_micro` (one-time reset = a cumulative re-report,
a server no-op), the pure four-leg predicate (`kat_soft_gate.rs`), and
the STICKY machine-global `contributed_once.json` marker
(`kat_contribution.rs`) — sticky because the payout cadence read showed
`pay_due_rewards` rides the 7-day `settle_epoch`, NOT the hourly bucket,
so a time-boxed grace would brick contributing users mid-epoch. Presence
IS the fact (corrupt reads contributed — the anti-brick direction). The
gate is mode-wide: the bare dry run settles charge too (measured by
`kat_burn_stack` phase 1), so a `--write`-only gate would be a sieve.
G3 reprice swept the same commit (kat_meter docs, heal.rs warns, README
fuel model + soft-gate section, AGENTS CLI-UX + the stale release-set
claim — `kat_transport` ships since v0.1.2/`56bd9bf`). CLI contract:
`tests/kat_soft_gate_cli.rs` 7/7. Found validating, filed as Issue 078:
the two QR goldens fail at HEAD (pre-existing, proven via stash — the
independent-preimage halves PASS, so it is render drift, not a layout
break). Open owner call: riir-dapps `.issues/025` (the 100 KAT vs 100 M
KAT grant-units mismatch; m3 sits at balance 0 until it lands).

### Issue 078 resolved — the QR goldens encoded the RETIRED host (2026-09-09, ecbf542)

The record above says "render drift" — that hypothesis is now FALSIFIED,
and the provenance is proven: both pinned goldens are byte-exact the
digests of the PRE-`f7cbe31` URL (`kat.heal.gist.rs/payreq/71b5cbf...`,
verified by a since-removed scratch test). `dc9e179` captured them
against that host; `f7cbe31` flipped `DEFAULT_SERVICE_URL`/`QR_GOLDEN_URL`
to `heal.gist.rs` and its commit message claims "QR golden … follow",
but the diff touched only the URL — the pins kept encoding the retired
host's QR and both golden tests ran red from the flip until now. NOT
upstream: the lock resolved `qrcode 0.14.1` throughout (the existing
`qrcode_resolved_version_is_the_pinned_one` lock-read pin was green the
whole time — issue 078's fix-shape step 2 "add a version pin assertion"
was already satisfied by it).

Fix (ecbf542): both goldens re-derived at the CURRENT host
(`de761701→a06a9976…`, `213caa9e→4f0f6ee5…`), provenance in both doc
comments, and a new `qr_golden_url_tracks_the_default_service_url` tie
that fails AT THE URL LINE with the re-derive instruction when
`DEFAULT_SERVICE_URL` moves — the gap that let the flip run red.
Gate coverage (078's closing condition): the `kat_transport` lib tests
are compiled AND run by `ci_feature_guard.sh` Layer 5c (`cargo test
--all-features --tests` selects the lib unittest binary); the lane had
simply not completed since the flip. 6/6 `qr_*` + 30/30
`kat_lease_client` green, clippy `-D warnings` clean at
`--features kat_transport`. Issue file removed per the noise-reduction
rule; this section is the record.

## 2026-09-11 — Plan 124: `--config-audit` (post-mining intake classes A+B consumed)

The idle loop's Protocol C unit: the top-ranked open intake classes from
`.distill/001_mining_queue_snapshot.md` §Post-mining intake, consumed as a
report lane (the `--frontier-report` shape — top-level ungated module, pure
syn analysis, report-first gates nothing, always exit 0).

- **Class A** `config-default-is-a-named-constant-and-the-consumer-reads-the-constant`:
  a `pub` config field whose hand-written `Default` init is a bare const
  path, with ZERO field reads anywhere while the const IS read elsewhere.
  Probe-verified boundary (Plan 124 T1): clippy 1.98 fires ZERO findings on
  the inert shape in both lib and bin crate forms — the gap is measured, not
  assumed. Detector precision over the consumer's Python heuristic:
  destructuring binds count as reads (the documented blind spot that put
  three LIVE fields in its most-confident bucket), qualified defaults match
  full-path only (kills the cross-type `::COUNT` collision), and the
  Default's own RHS does not satisfy the "read elsewhere" condition.
- **Class B** `assert-on-default-config-value`: fields only ever asserted
  against their own Default (literal / const / `None` / tautological bool).
  The T5 hand-check on riir-chain forced the precision clause the intake
  spec names: asserts against COMPUTED expectations are behavioral coverage
  and count as reads (`GuardVerdict.backoff_ticks` — 8 asserts against decay
  curves, correctly silent; `GuardVerdict.redirect` — only ever
  `assert_eq!(v.redirect, None)`, accurately reported).
- Validation (T5): riir-chain 177 files/41 candidates → 1 verified TP;
  riir-neuron-db 102/76 → 0; self 232/34 → 0. Zero false positives.
- GOAT: G1 15/15 fixture matrix (fire/silent incl. blind-spot guard,
  cross-file index, collision guard, condition-3 self-read suppression,
  behavioral-assert refinement); G3 full lib suite green + clippy clean;
  G2/G4 N/A report-only; score-bench N/A (no proposer change). Ungated by
  design (zero new deps — syn + proc-macro2 + quote already core deps;
  rustc_errors precedent).
- Class C (`config-path-orphaned-by-refactor`) stays an intake row — it
  needs git history, not a per-compile scan (the intake itself says
  review-time).

## 2026-09-12 — Issue 089 T3: `--misses` generalized to every domain + the 09-08 bare-default regression fixed

`cargo heal --misses` now prints the Plan 091 rustc_errors per-CODE queue
VERBATIM plus one frontier-ordered RULE queue per ENABLED retrieval domain
(clippy/perf/kernel/sec/docker; `dist` excluded — suggest-only advice never
writes trajectories). Attribution is by corpus membership
(`frontier_report::{domain_queue, render_domain_queue}`, observed-rows-only
tables + per-domain coverage lines + an unattributed-keys footer for
partial-feature builds); the ordering is the shared dispersion control law,
and the per-RULE band report (`--frontier-report`, T1/T2) stays the separate
admission instrument — additive, not a replacement, exactly as the task
row specified.

The landing required three en-route fixes, each caught by the validation
itself:

1. **`cargo heal --misses` had been exit-2 broken since 2026-09-08.** The
   just-works bare default fired under EVERY standalone mode (only the four
   primary modes were excluded), setting `suggest=true` — and `--misses` is
   the one standalone mode with a strict conflict guard. `is_bare_posture`
   now excludes every standalone mode, pinned by
   `standalone_modes_are_never_bare` (16 modes; a new standalone mode adds
   its row there in the same commit).
2. **The first loader was an undercounting `collect()`** — store rows are
   per-(rule, shape), so a map-collect keeps only the last row per rule.
   `load_rule_outcomes` sums; live-validated against the T1 measurement
   (`use_self` 172 kept, learn-rows == dump-summed).
3. **Two "clean" clippy `--bins` lanes were INERT** — `cargo-heal` carries
   `required-features` (`latent_retrieval,rust_perf,clippy_lints,kat_service`)
   and cargo SILENTLY skips the target when unsatisfied (the katgpt-rs
   riir-train Issue 513 class, caught here in-session). The real lanes re-ran with the
   required set and found the two compile errors the inert lanes had
   blessed. `run_conversion_report` was also an ungated use of the
   `rustc_errors` module — now cfg-gated with a loud refusal.

The healer rode the same file for 5 pre-existing mechanical findings
(4× `semicolon_if_nothing_returned`, 1× `redundant_closure` — compile-gated,
verified). Validation: lib 1206/1206 default, bin 16/16 at the full feature
lane, clippy clean on lib+bins at default and at
`latent_retrieval,kat_service,kernel_heal,sec`; live smoke on this repo's
store: clippy 15/49 observed (`manual_is_multiple_of` 2/2 informative at
rank 1 — the queue's first genuinely informative domain row), perf 3/126,
sec 2/15, kernel 0/607 honest-empty, `--limit` caps per section,
`--misses --report` unchanged. Issue 089 T3 row carries the full record.

## 2026-09-12 — Issue 092 T1: the GW corpus-alignment PoC, measured and closed (defend-wrong NEGATIVE)

The 371/594 fusion idea (align two healer rule-corpora's similarity spaces
with GW → cross-domain correspondence map for dedup + fix-transfer) ran its
defend-wrong PoC and lost, cleanly and at matched review budget.

**Instrument:** katgpt-rs `b09f1368` exposed the GW coupling publicly
(`gw_coupling` — same deterministic multi-start solve, returns the winning
coupling matrix instead of only its scalar loss; identity-correspondence +
bit-determinism unit tests). The PoC harness is `examples/gw_corpus_poc.rs`
behind opt-in `gw_corpus` (measurement-only; no heal-path, no router).

**Protocol:** clippy_lints (49) × rust_perf (126) = 6,174 pairs. Arm (a) GW:
trigram-cosine distance over `buggy_code`+`fixed_code` ONLY (names withheld —
sharing the baseline's lexical signal would be circular), perf split even/odd
into 63-halves (GW_MAX=64), two solves unioned. Arm (b): token Jaccard over
rule identifiers (today's name/convention dedup as a ranking). Arm (c): stride
base rate. Ground truth: 180 hand-labeled pair reads (ALL top-60 of each arm +
60 stride pairs) against criteria fixed before labeling.

**Result:** GW precision@60 = 0.017 = the base rate (no signal); name/keyword
0.083 = 5× base rate. GW's strongest couplings (full-column masses) were
structural artifacts (`collapsible_else_if`×`powi-vs-powf`); the one
GW-exclusive TRUE (`manual_slice_fill`×`retain-inplace`) is a data point, not
a win. Robust to any single label flip. The planned known-negative class was
EMPTY (no perf `context_notes` names any of the 49 clippy lints — the
"Distinct from" refs are all within-domain).

**Mechanism:** surface-trigram distance structure is not fix-shape structure;
and this test bed's true correspondences are name-visible by construction
(both corpora name rules after the same Rust API surface — the disjoint-
vocabulary regime where structure-only alignment could win does not exist
between these two corpora). Reopen trigger recorded in the issue: a semantic
fix-shape embedding (AST/transform fingerprints) or a disjoint-vocabulary
corpus pair (clippy × docker).

Full record + 180-label appendix: `.benchmarks/083_gw_corpus_alignment_poc.md`.
Issue 092 closed (T2–T4 never fired). Research note 166 §A verdict updated.

## 2026-09-12 — Issue 093 T1: the per-run strategy-posterior PoC, measured WIN (defend-wrong)

The 373/592 fusion idea (a per-run `NonergodicFilter` over strategy
 generators — 545 §4's "file-level posterior, nonergodic upgrade of the
router", at strategy level) ran its defend-wrong PoC and WON all three
scenarios.

**Instrument:** the katgpt-rs Plan 592 kernel consumed directly —
`katgpt-micro-belief` becomes an 11th escaping path dep (opt-in, feature-
forwarded `nonergodic_belief`; already transitive via katgpt-core defaults —
the row is the direct gate, BOUNDARY + standalone pin updated same commit).
Harness: `examples/nonergodic_driver_poc.rs` behind opt-in `nonergodic_poc`
(measurement-only). Arm B is the REAL `shape_conditional_order` over a seeded
TrajStore — the actual shipped competitor, not a simulation of it.

**Protocol:** 1,200 paired file-runs per cell (240 files × 5 seeds); 10 REAL
playbook strategies (E0597's 6-strategy menu is where ordering bites);
outcomes are BLAKE3 draws from frozen (regime × difficulty) tables keyed by
(site, strategy, context-hash) — same bytes + same context → same outcome
(the tried-set guarantee), context change → fresh draw (the compiler's
verdict on a re-proposed edit depends on the current file). Scenarios:
aligned / stale (regime-swapped store — the world changed under the store) /
cold. Fourth arm D composes store+filter (λ_t = t/(t+1)) — the T2 production
shape.

**Result:** C (filter) resolved 41.7% vs A (min-delta) 35.2% and B 36.7%
aligned / 34.3% stale (B drops BELOW A under a stale store); wasted-attempt
54.8% vs 58.6/56.5; McNemar 160/82 vs A (z≈5.0), 146/86 vs B aligned (z≈3.9).
Misspec pass (±20% on the filter's tables, world unchanged): edge holds
entirely — the load-bearing property is the regime winner-SPREAD, not exact
values. C is store-independent by construction (identical rows across
scenarios — the paired-design sanity check).

**The harness bug worth remembering:** the first draft's revive rule fired on
posterior flip alone — 356 revives, 0 helped, provably: a frozen-context
retry is the same bytes against the same file, so its outcome CANNOT change.
Fixed before the recorded run: revive requires the suppression CONTEXT to
have moved too (another fix landed in between), which is also the faithful
model of the real driver (a changed diagnostic at the site is a NEW
TriedKey, not a revive). Post-fix: 57/50 net-positive. D under a stale store
shows the cost of composing with a lying prior (78/137 revives, 38.2% —
the λ-schedule needs a stale-detection gate, recorded as T2 design space).

**Honest bounds:** C's model family matches the world generator family by
construction (regimes ARE the generators; misspec covers parameters, not
family); the toy store is deliberately sparse (2 events/shape — B's aligned
win over A is NOT significant at this density, 138/120 p≈0.24, and NO claim
is made about the shipped ordering at real densities). T2's GOAT on the real
1%-fail surface is the family-error test.

**T2 unlocked** (T1 win + Issue 91 T1.5 ring landed `795ddc4c`): opt-in
wiring at the `rustc_errors` driver ordering seam, uniform or store-seeded
priors per run, context-scoped revives, GOAT = resolved-rate-at-equal-budget
on the real corpus via the G6/G7 floor A/B harness.

Full record: `.benchmarks/084_nonergodic_driver_poc.md`. Research note 166 §B
verdict updated.

## 2026-09-12 — Issue 094: trajectory covariability-class mining PoC, measured and closed (defend-wrong NEGATIVE)

The Research-309 consumer hypothesis — the sliceTCA drift-class share per
rule predicts which corpus rules needed re-mining (proposer revision/guard
after landing) — ran its defend-wrong PoC and lost to the activity
confound. The failure is the STORE SCHEMA's, mostly, and that distinction
is recorded below.

**Instrument:** `examples/tca_covariability_poc.rs` behind the existing
`latent_retrieval` feature (no new feature key — `slice_tca` is default-on
in katgpt-core since katgpt-rs Plan 596 / Bench 714, so no forwarding needed).
Stores read via `/tmp` MIRRORS only (sibling `.heal` trees are read-only to
a session; the mirror guarantees zero writes upstream). Ground truth = the
2026-09-12 re-mined(33)/stable(27) label snapshot dug from
`.distill/001` §Post-mining intake P1–P19 mapped to proposer-fix commits
(clippy lane snake_case keys, rust_perf kebab — the store's exact
spellings; dogfood commits and the mechanical b83230ee emitter migration
excluded from labels).

**Protocol:** tensor X[rule, day, shape-bucket] over the 10 store-bearing
repos — 12,557 rows → 4,496 cells after filters (rustc_errors `:` keys,
Withdrawn tier, 8,060 no-timestamp game-sdk rows). Axes: [35 rules] ×
[18 days] × [32 FNV-1a buckets]; cell = ln(1 + success + fail) mass.
Classifier path only (`covariability_shares`, 3 SVDs per probe — the
issue's "may suffice" arm). Per-rule score: leave-one-rule-out
ΔEVR_time. Eval: exact Mann-Whitney AUC over labeled rules carrying mass
(30 of 60; 23 re-mined, 7 stable), pre-registered PASS rule = beat every
baseline.

**Result:** AUC(S_tca, r=4) = 0.497 — chance (robustness: r=2 → 0.438,
r=8 → 0.469). Baselines: total log-mass 0.689 (best — the activity
confound: rules that fire get revised), fail-share 0.559, day-entropy
0.099 (stable rules spread MORE evenly — the inverse of the naive drift
expectation), majority 0.5. Global covariability shares saturate
(entity 0.990 / time 0.994 / episode 0.985): one hot rule
(`uninlined_format_args`, log-mass 1879 of ~2900) dominates every
unfolding's top-R spectrum, so per-rule LOO deltas are scale-driven, not
class-driven. FAIL per the pre-registered rule.

**Mechanism (honest bounds):** the store keeps CUMULATIVE counters per
(rule, shape) with only the LAST verdict's timestamp — the session axis
collapses to "day of last fire", so time-slicing measures shape-churn-day-
clustering, not within-shape drift; 30/60 labels carry zero mass
(survivorship: quiet stable rules are absent from the store, and the
stable rules that DO fire look like re-mined ones). The drift hypothesis
is NOT refuted for event-level data — it is untestable at this schema.
Reopen triggers: (a) per-event trajectory rows persisted store-side (the
fixseq ring is the precedent shape) giving a true session axis; (b)
balanced per-rule outcome labels (score-bench `rule_outcomes` rows are the
per-rule precedent).

Full numbers: the harness output (this commit's run) + the issue file's
git history. Issue 094 closed (the FAIL branch of its own task 4).
riir-neuron-db Research 309's consumer-surface line got a status delta (healer surface:
measured negative; neuron-db Plan 329 unaffected — different data,
event-level telemetry is its design center).

## 2026-09-12 — Issue 093 T2: the nonergodic driver wiring LANDED (gain-null on no-surface) + the orphan-report Windows blind-leg fix

**T2 landed** ([Bench 085](.benchmarks/085_nonergodic_driver_wiring.md)): the
PoC's winning per-run strategy posterior is wired at the production seam —
opt-in feature `nonergodic_driver` (NOT in the release set; the release
feature-set build verified untouched). Three pieces: (1) **driver seams**
(ungated, generic): `CompileDriver::with_observer` (fires at all five
attempt-outcome sites with the `AttemptRecord` ring record — the in-process
twin of the `.heal/fixseq/` dump, Issue 91 T1.5's one-instrument-two-consumers
design paying off) + `with_revive` (consulted for tried candidates in the
retain; unset = the shipped never-revive behavior — default-lane semantics
byte-identical, lib 1247/0 + integration 6+6+4+4). (2) **the policy**
(`src/domains/rustc_errors/nonergodic_order.rs`): a rank-rotation component
family — the honest, weaker port the PoC's bound #1 demanded (real rustc
regimes are latent; the family captures "which strategy wins here" without
semantic structure). Menu frozen at first ordering call; base ranking
store-seeded or incumbent; component k favors base rank k; two-state sticky
inner chain; token = menu_idx·3+outcome; arm-D λ=t/(t+1) composition with
the stale gate (committed non-base majority after ≥2 obs → store term
drops — the "lying store paid 3.5pp" defense); context-scoped one-shot
revive (span-hash + file-diagnostic-set moved AND commit moved AND gate
≥0.35). (3) **CLI install**: `--fix-compile` replaces the static
shape-conditional ordering when armed (kill-switch
`RIIR_HEAL_NONERGODIC=0|false|off`).

**Gates 6/6** (`tests/nonergodic_driver_ab.rs`, required-features gated):
mechanism — determinism, the learning flip (T6.7 flip-test class), revive
negatives + **flip-window-existence grid** (fired at [(0,2),(0,3),(0,4),
(1,4)] of the (kept,no-progress) grid — the PoC's narrow revive window
confirmed on real token arithmetic; precise-cell asserts deliberately
avoided, the constants' business ≠ the contract), stale gate; e2e (real
cargo, equal budget) — no-regression (both arms clean, remaining 0,
identical applied) + determinism (two fresh runs, identical attempt
sequences).

**The honest verdict: GAIN-NULL ON NO-SURFACE.** Both arms are
first-try-perfect on every real scaffold (0 wasted attempts) — the Bench
045 T6.7(b) class; the playbook menus are shape-gated (≈one strategy per
diagnostic), so ordering choice is site-order, and site-order only wastes
attempts on fix-interaction chains, which no available surface exhibits.
The family-error test — T2's whole point — moves to the **reopen trigger**:
the fixseq ring's first runs with ≥2 revert-class attempts become the A/B
e2e corpus (both orderings, equal budget). T3's negative clause NOT fired
(a no-surface null is not a measured negative). Issue 093 stays open as the
watch record.

**En route (pre-existing, Windows-only):** `orphan_report::GitCli::new`
compared canonicalized scan paths against git's root WITHOUT canonicalizing
the root — on Windows `canonicalize()` yields the verbatim `\?\C:\…` form
while `rev-parse --show-toplevel` returns `C:/…`, every pathspec dropped,
the whole history leg blind; `real_git_e2e_orphan_lifecycle` failed 3/3
deterministic on this box (it landed green from macOS, where canonicalize
adds no prefix — the L132 symlink comment shows the class was half-seen).
Fixed: canonicalize the root for the comparison only. This is the
"validated-on-one-OS" lesson again — the sibling-session's 1247/0 was true
on macOS and silently false on Windows.

## 2026-09-12 — Issue 098: the rule-corpus cross-OOO axis, measured PASS + wired (absorption candidates found)

**Issue 098 RESOLVED** ([Bench 086](.benchmarks/086_rule_corpus_cross_ooo_axis.md);
the Research 552 §F2 fusion from katgpt-rs Issue 759 / riir-ai Research 378).
The absorption/splitting audit aimed at this repo's own rule bank: rules =
units, each rule's oracle-labeled triggering spans = its MEI set (ALL of
them — a binary oracle has no graded activation, so the bank is RAGGED and
the free `ooo_score`/`cross_ooo` kernels compose into a hand-built
`BankAudit` for `greedy_curate`; `audit_bank_into`'s uniform top-K does not
apply).

**Metric evolution (the katgpt-rs Bench 758 metric-granularity law, observed live):**
plain TF-cosine over `RustOpBigramTokenizer` tokens was the control and was
REJECTED — it let the generic scaffold swamp the rare op signatures on
micro-span rules and pinned three pairs at exactly 0.500 (the tie
signature). The tokenizer's own design doc names the law ("ubiquitous runs
earn near-zero IDF... rare ones earn high IDF and do the separating") —
final metric: **TF-IDF cosine** (idf = ln(N/df), pool-ubiquitous terms read
exactly 0).

**Population (honest):** every non-clippy corpus is 1-entry-per-rule
(rust_perf 126/126, kernel_opt 607/607, sec 15/15, docker 31/31) — the axis
is UNMEASURABLE there at K≥2 (a corpus-shape finding; multi-span corpus
growth is the prerequisite). The clippy audit runs on the 100-span real
oracle-labeled fixture pool (v1_frozen 33 + gen6_real_repo 17 +
CLIPPY_SPANS 50; 42 rules, 17 auditable, 25 unauditable-reported).

**T3 verdict: PASS — 3 absorption candidates** (cross < 0.6):
`manual_map↔needless_return 0.462` (the gen-8 closure-birth entanglement,
independently corroborated), `manual_map↔single_match_else 0.500`, and
`single_match↔single_match_else 0.500` — **the calibration anchor**:
clippy's own split of one lint, detected by the axis unprompted
(defend-wrong validation, pinned by test). `unique_feature_count = 12 of
17` at paper-default cutoffs.

**Wiring (T3+T4):** `score_bench` forwards
`katgpt-core/direction_bank_audit` (the ood/regime_probe precedent);
`src/score_bench/cross_ooo.rs` (pool + TF-IDF sim + ragged audit + render,
7 lib tests); `frontier-report` gains the additive cross section (bin-side
under the gate — `frontier_report::render` stays ungated/untouched);
`score_history.jsonl` rows gain `cross_ooo` (the ood/strata additive law;
Warm-tier `ScoreRunRow` mirror drops it — the FOUR-exception
schema-identity contract); `--score-bench` prints the axis beside
rule-outcomes (one cached measurement, the `ood_axis` precedent); the
mining queue carries the merge candidates (single_match-family
consolidation candidate + the manual_map born-lint cluster record).

**Gates:** cross_ooo 7/7 · score_bench-lane module tests 80/80 · report
17/17 · traj_store 38/38 · default lib 1247/0 · clippy 0 warnings both
lanes (default + score_bench `--lib --examples`). En route: `cargo heal`
routed the new Rust files to the perf/docker domains and proposed only
false positives (0 edits) — logged as a healer router miss for the
post-mining queue; manual fixes landed instead.

Issue file removed per the noise rule; this entry + Bench 086 are the
record.
