# Issue 242: Rule rebuild description-fidelity audit (the iWSP text-bottleneck metric on the rule corpus)

**Status:** Open — fusion idea; **T0 EXECUTED 2026-11-17 (zed-m3-i242-t0): NOT redundant — no existing surface measures re-derivability; T1 remains justified, sample design sharpened below**; report-only until a defect justifies promotion. **T1 HOSTED LEG BLOCKED-OWNER (verified 2026-11-21, amo idle lane zed-daa5e0d7@amo):** the hosted writer seat has no reachable arm on this box — no `RIIR_HOSTED_EXPERT_TOKEN` anywhere (riir-refine `.env` is 0 bytes; shell env clean), `.refine/escalation_manifest.json` does not exist (the manifest ships ALL-EMPTY — no R1 endpoint row was ever armed here), no loopback expert serving on the known expert ports (8091/8092/8149/8793/8096/8000/8078/8095 all refused), and the `RIIR_REFINE_CODE_EGRESS=snippet` consent is unset (the call refuses pre-socket by design). Arming = a CREDENTIAL TOUCH (token into `.env` per the `.env.example` Groq shape + the R1 endpoint row into the escalation manifest + the egress consent) — the owner gate per the 2026-11-11 gating decree. Everything short of the token stays doable: the stratified sample freeze, the description composition, the fixture scoring harness — the run itself waits on the seat.
**Lease closed:** zed-m3-i242-t1prep, 1795386645, was expires 2026-11-24T22:30Z — the T1-prep lane LANDED same session (scope: the description composition + the audit-harness protocol of record; the run itself stays seat-gated) — see the T1-PREP LANDED row below
**Lease closed:** amo-idle-i242-preflight, 1795244100, was expires 2026-11-22T07:00Z — scope discharged same session: the pre-flight verdict above (reads only, no builds — the amo calibration-boot window's poison law held).
**Lease closed:** zed-m3-i242-t0, 1794900078, was expires 2026-11-19T07:21:18Z — T0 landed in the window (census + cross-read recorded below); T1 (hosted spend + new report lane) was explicitly outside the lease and stays unstarted
**Source:** arXiv:2611.18114 "Interpretable weight-space programming builds transparent transformers and brain encoding models" (Singh et al., MSR/JHU/Columbia/NYU, 2026-11-13) — the **text-bottleneck rebuild metric**: describe an interpretable model in ≤200 words → a fresh agent session rebuilds it from the description ALONE → the rebuild preserves behavior (their numbers: all 13 algorithmic rebuilds 100% hidden accuracy, NLP −1.4 pts, rebuild params 1.74× the original vs **2,043×** for a description-less control, NLP error-correlation 0.74 vs 0.51). Distillation + verdict (Gain): `katgpt-rs/.research/678_iWSP_Interpretable_Weight_Space_Programming.md`; verdict round-1 AGREE conditions are baked into this issue.

## The idea

The rule corpus IS the model (the corpus-is-the-model doctrine), and its rules already travel as descriptions (`context_notes` propagated end-to-end; the hosted seat consumes them). The paper's metric gives an operational test of description fidelity the corpus has never measured: **can a rule's behavior be re-derived from its description alone?**

- For a sampled rule: compose a ≤N-word description of the rule's behavior (trigger + fix shape, not its code), hand the description to a hosted seat (`hosted_expert`, opt-in), and have it RE-DRAFT the fix from the description only.
- Score the re-drafted fixes against the rule's authored fixes on the oracle-labeled eval fixtures (`retrieval_eval.rs` fixtures class) — a fix-match rate per rule.
- **Control arm:** the same re-draft task with NO description (the paper's 2,043× control). The metric is the paired Δ.

A rule whose behavior survives the text bottleneck is a well-formed description; one that doesn't is machinery only its code can speak — a candidate for better `context_notes`, not auto-deletion.

## T0 — the free redundancy pre-check (BEFORE any tokens)

The corpus already carries `context_notes` + quality gates (`--gate-calibration`, `--coverage-report`). First check — zero cost, no hosted calls — whether those surfaces already answer the fidelity question (e.g., a census of how many rules carry substantive `context_notes` vs empty/boilerplate, cross-read against `--gate-calibration` strata). If they do, this issue CLOSES as redundant with the reason recorded. This is the verdict round's named weakness, promoted to a gate.

## T1 — the capped first run (only if T0 leaves a gap)

**T1 — the capped first run (only if T0 leaves a gap)**

- **Sample: FROZEN 2026-11-21 (amo-idle-i242-preflight), before any hosted call.** 30 rules, 10/10/10 stratified clippy_lints / rust_perf / kernel_opt, deterministic: `sorted(ids, key=sha256("242:<domain>:<rule_id>"))[:10]` over the corpus sources at riir-refine develop `bb47308e` — extractors: clippy `name:` (120 ids), rust_perf/kernel_opt `rule_id:` (380 / 1098 ids). **sample-digest sha256 `a5ad3f9fba066fa15e5867779a1a0c09e154726ab922afca22d679212a5506f4`** over the sorted-key JSON. The frozen ids:
  - clippy_lints: `doc_lazy_continuation`, `needless_bool`, `filter_next`, `unnecessary_lazy_evaluations`, `cast_lossless`, `needless_range_loop`, `unnecessary_map_or`, `map_or_identity`, `match_bool`, `manual_repeat_n`
  - rust_perf: `eager-pointer-collect-lazy-fetch`, `watchdog-deadline-moves-on-command-progress`, `bufread-scan-in-buffer-scratch-fragment`, `saturating-f2i-cast-proven-range-to-int-unchecked`, `batch-reserve-monotonic-counters-offload-bulk`, `byte-flag-table-masked-run-scan`, `retain-inplace`, `work-removed-build-is-not-a-valid-lower-bound`, `regex-hoist-lazylock`, `smallvec-stack`
  - kernel_opt: `boot-identity-hash-read-once`, `same-input-gemv-segments-single-launch`, `walk-terminating-fill-for-unused-buffer-capacity`, `tile-coo-sparse-tile-aligned-format`, `enumerable-domain-bit-identity-admission-gate`, `verify-fold-gqa-repeats-into-row-axis-tile-fill`, `defer-promoted-write-to-first-consumer-dispatch`, `dtype-branch-dispatch-parity-in-place-precondition`, `moe-group-selection-fused-into-topk`, `block-scaled-mma-scale-register-owns-its-rows`
  - Re-derivation: any corpus change re-freezes by the same procedure at the new commit (the digest is the audit anchor, not a barrier).
- **Hosted budget:** ≤ $10 list-price cap for the whole run (the dspy-wsp writer-spend datum, Research 671); refuse to start without the cap recorded.
- **Kill bar (pre-registered):** if description-only re-drafts reach **Δ < 10 points of fix-match over the description-less control** on the fixtures, the audit CLOSES NEGATIVE — the corpus's descriptions carry no re-derivable signal beyond task prompts, and the instrument dies with the negative recorded (Issue-565 precedent).
- **Pass bar:** Δ ≥ 10 points with per-rule variance that identifies specific low-fidelity rule families — the only promotion-worthy outcome is **a rule-quality defect the existing gates missed** (e.g., a family whose `context_notes` never survived the bottleneck).
- **Report-only:** `cargo refine --rule-rebuild-audit` (report lane, exit 0); NO promotion path, NO corpus mutation, NO gate wiring until the defect-showing bar is met in a NEW pre-registration.

## Disclosures

- The re-draft needs a hosted writer — `hosted_expert` posture, code-egress consent scopes apply (rule descriptions leave the box, rule code need not).
- The metric is validated on neural networks (this paper), not rule corpora — the domain transfer is exactly what T1 measures; do not cite the paper's rebuild numbers as evidence the audit will work here.
- External metric provenance: the rebuild protocol is the paper's; no novelty claim — this issue is an adoption idea (Research 678 verdict round 1 AGREE, session 1eef961b-5552-47cf-94f2-b87e80914a6b).

## T0 RESULT (2026-11-17, zed-m3-i242-t0) — NOT redundant; the census sharpens T1

Heading fix in the same edit: the title read "Issue 241" (template copy-slip from the disk-clean issue) while filename, status lines and body all say 242 — the citation oracle reads headings, so the misattribution was live; corrected to 242.

### The description-surface census (grep-derived at HEAD `b59974c8`, ±2 parse slack on Some/None splits; standing counts are the pins_manifest's, never these)

| Domain | Rules | Description surface | Fidelity posture |
|---|---|---|---|
| kernel_opt | 1091 | `gpu_concept` — 970 parsed, min 969 / median 5,285 / max 41,314 chars, zero under 200: MECHANISM + verbatim LAW quotes + CLASS SIGNATURE + REGIME + ANTI-REGIME + KINSHIP + Source | description-RICH (the decision content IS the description) |
| rust_perf | ~362 constructor sites | `context_notes: Option<String>` — 306 Some (283 ≥ 400 chars), ~52 None (seed-era rules like `o1-lookup`, `bsearch-sorted-slice`) | 85% description-RICH, 15% code-only |
| clippy_lints | 120 | **NONE** — `ClippyFixEntry { lint_name, lint_group, buggy_code, fixed_code }`; the lint name is the only prose | description-POOR |
| sec | ~76 parsed | `sec_concept` one-liner — median 639 chars (range 32–4,052) | mixed; median substantive |
| rustc_errors | generated cases | `GeneratedCase { name, expect_code, kind, seed, broken }` — no prose field | description-POOR |
| docker | 6 | `docker_concept` one-liner + `trigger` (the detector contract, in prose — the retrieval surface reads this) | description-OK |
| shader_perf | 4 | `shader_concept` one-liner | description-POOR, tiny corpus |
| dist / ane | 4 / 8 | `notes: Option<String>` on 2 / 6 | partial |

### The cross-read — no existing surface answers the fidelity question

- **`--gate-calibration`** (Issue 130): groups score-history rows by stratum (`repo`/`authored`/`construction`) into heal-rate δ + OOD drift + corpus-stall verdicts. Source grep: ZERO `description`/`context_notes` mentions — it measures admission-gate evidence, not description fidelity.
- **`--coverage-report`** (Issue 119): corpus-vs-driver lint coverage — a PRESENCE axis, never re-derivability.
- **`--half-split-report`** (Issue 232 T1): the CLOSEST adjacent instrument — scores a neutral probe set through the corpus's two text surfaces (trigger A vs resolution Q) and prints pick-agreement. It tests whether the corpus's own text surfaces agree on picks; it does not test whether an external agent can REBUILD the behavior from the description alone, and it needs a neutral probe set + the opt-in feature.
- **`context_notes` propagation** (Plan 001): a signal channel to `DraftedFix`, not a fidelity test.

**Verdict: the redundancy close FAILS — the question "can a rule's behavior be re-derived from its description alone?" is measured by nothing.** T1 stays justified.

### What T0 changes about T1

The corpus is sharply BIMODAL, so the rebuild test's information is not evenly spread: on kernel_opt (median 5.3 KB decision-content concepts) and the rust_perf annotated set, the description already IS the rule's source-of-truth text — a rebuild test there mostly measures the transcriber, which the standalone-compiled fixture gates already pin. The open question lives in the **description-POOR strata**: clippy_lints (120 rules whose only prose is the lint name), rustc_errors (generated cases), the ~52 seed-era rust_perf `None` rules, shader_perf, and sec's short-concept tail. T1's 20–30 stratified sample should draw PRIMARILY from those strata; a kernel_opt arm is a secondary control (expected to pass trivially — that expectation is itself the control's value).

T1 remains unstarted: it needs the `--rule-rebuild-audit` report lane (new code + build), the hosted-expert writer with the $10 cap recorded pre-run, and code-egress consent scoped to rule descriptions. None of that ran in this lease.

**Routing note (2026-11-17, shikuwa-4090 disk/housekeeping lane):** T1 is CREDENTIAL-BLOCKED on the 4090 box — `RIIR_HOSTED_EXPERT_TOKEN`, `GROQ_API_KEY`, `INFRON_API_KEY` and `CLOUDFLARE_OPS_API_TOKEN` are UNSET at every env scope (process/User/Machine), no `.env` exists in the relevant checkouts, and no live `.refine/escalation_manifest.json` exists there (the Oct-8 `escalation_manifest_heal_armH.json` is a loopback seat_rerank row, not a hosted writer). The hosted half needs an owner-provided token on a credentialed box (m3/amo carry the seat lanes) — per the 2026-11-11 gating law credential provision is the owner's act, so this parks here as the unblock condition: token + manifest row on some box, then any lane may claim T1 and run the capped cell. The report-lane build half is claimable anywhere but stays parked until the hosted half has somewhere to run — a lane without its seat measures nothing.

## T1-PREP LANDED (2026-11-22, zed-m3-i242-t1prep) — the composition + harness protocol exist; the run stays seat-gated

- **Re-freeze CONFIRMED, no-op:** the deterministic selection re-derived at HEAD `972a59596f9b` after the B398–B400 corpus delta (unique ids: clippy 120 unchanged, rust_perf 380→389, kernel_opt 1098→1112) reproduces the IDENTICAL 30 ids in all three domains — the v1 freeze (digest `a5ad3f9f…`) is selection-stable and stands; the digest remains the anchor.
- **The 30 descriptions COMPOSED + FROZEN:** `assets/rule_rebuild_audit/descriptions_v1.json` — ≤200-word prose (trigger + fix shape + regime/anti-regime decision content), canonical digest `de09306af862b4e400765b0c855c22c66ea397faeccc7c1c3d7ff8821495cc20`, composed at `972a59596f9b`, frozen PRE-RUN (a post-run description change lands only as a new digest version — no in-place edits after the run consumes v1). Composition law + the full verification record (mechanical word-count/anchor/token checks + every measured number grepped at its entry) in `assets/rule_rebuild_audit/README.md`.
- **The run protocol of record:** `assets/rule_rebuild_audit/README.md` — paired description-arm vs description-less-control arm over the entries' oracle-labeled fixture pairs, fix-match primary, the ≤$10 cap, and the issue's kill/pass bars carried verbatim. The scorer build stays parked per the 2026-11-17 routing note (a lane without its seat measures nothing) — buildable at run time on a credentialed box.
- **What remains (all seat-gated):** the owner credential act (token + R1 endpoint row + `RIIR_REFINE_CODE_EGRESS=snippet` consent on some box), then the run lane + the `.benchmarks/` record of the paired Δ.
