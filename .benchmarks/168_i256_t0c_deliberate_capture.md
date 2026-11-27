# 168 — Issue 256 T0c(b): the deliberate capture campaign (172 multi-attempt rings) + the census re-run — T0 WEAK-SUPPORT (typed), T1 UNDECIDABLE, T2 stays gated

**Status:** CAPTURE EXECUTED + CENSUS RE-RUN 2026-11-27T18:0x–19:0xZ (4090, this box); **T0c(b) bar NOT met — the ≥10% non-clean target read 3.0% (5/168)**; T0 early-read AUC 1.000 / recall 1.000 over a 5-positive class (typed WEAK-SUPPORT — the evidence class the 165 record warned about one row up, plus a DELIBERATE-corpus posture the organic lane cannot pool with); T1 macrostate 169× `Undecidable{WeakSupport}` (0 Closed / 0 Lossy); **T2 pre-registration stays GATED** per the issue's own law; claim lease zed-600a1d37@4090 `6214f72a145`, fold same session (the landed-batch-lease class).

## The capture (T0c(b), the issue's named deliberate leg)

The instrument is the lane's own mint posture — `examples/rustc_errors_floor --only driver --dump-fixseq <root> --dump-fixseq-cfg <cfg>` (the Issue-126/128 label-gen posture), NOT the CLI: the driver arm runs the production repair loop in-process over the known-inverse corpus (12 anchors + 276 generated + 4 interacting pairs) and dumps each case's attempt ring via the SAME `dump_fixseq` writer the CLI uses (schema-identical; the census loader reads both).

- **chunked** mint (`compose_multi(generate_corpus(), 5)` + interacting pairs): 80 run files, 270 attempts (kept 231 / reverted 39), driver fix rate 77.2% (92 mutants), span_embed 270/270.
- **strided** mint (`compose_multi_strided(.., 5)`, families mix per world): 92 run files, 304 attempts (kept 256 / reverted 48), fix rate 50.0%, span_embed 304/304.
- **Union:** 172 rings / 574 attempts (487 kept / 87 reverted) at `.benchmarks/data/168_i256_t0c_rings/.refine/fixseq/` + `manifest.json` (per-file BLAKE3; ⚠ the second mint OVERWROTE the root's manifest label — it reads `cfg: strided` while covering the 172-file union; the per-file digests are authoritative, the label is not).
- per-code kept rates: E0499/E0597/E0614 100% · E0382 61/80 · E0505 58/78 · E0502 122/170 (72%) — the reverts concentrate exactly where the playbook registry is thinnest (the floor's own Bench-041 territory map, reproduced at composed-world scale).

## The census read (the instrument's own output, 0 bad lines)

| arm | result | honest read |
|---|---|---|
| corpus | 172 runs, 168 multi-attempt, **ends-bad 5/168 (3.0%)** | the T0c target ≥10% non-clean NOT met — the deliberate corpus is driver-dominated (both configs fix 50–77%) |
| T0 early-read | LORO ridge, 168 folds (pos 5 / neg 163): **AUC 1.000, recall@median 1.000, precision@median 0.050** | perfect separability over a 5-positive class = coin-flip-class evidence (the 165 warning at n=4, one row up); the median cut degenerates to the class prior (0.050 == 5/168); the record does NOT print the per-strategy Laplace floor's own score — the printed base floor is 0.5, and "must beat the history floor" stays UNADJUDICATED here |
| T1 macrostate | 169× `Undecidable{WeakSupport{test_windows 0–2, support_floor 128}}`, 0 Closed / 0 Lossy | per-run windows (2–4 states) are structurally 64× under the primitive's 128-window support floor at world granularity — unreachable by corpus GROWTH of this shape; pooling folds is a primitive amendment, out of the census's report-only scope |

## Verdict

1. **T0c(b) EXECUTED, its readiness bar FAILED honestly:** the deliberate corpus cannot serve the pre-registered recall-floor read (≥10% non-clean required for the false-abort pricing; 3.0% measured). Composed worlds are the wrong negative-class generator — the driver fixes E0499/E0597/E0614 always and the corpus's E0507/E0596 mutants (which `cargo fix` repairs, the contested territory) end clean too often for the census's needs.
2. **T0's AUC 1.000 is WEAK-SUPPORT, never a GOAT claim:** 5 positives, deliberate posture, floor comparison incomplete. It confirms the instrument END-TO-END (loader → features → LORO → typed readout) on real rings — the 165 instrument's "armed on the go-forward capture" clause is discharged.
3. **T2/T3 stay GATED** (the issue's own law holds): no wiring, no pre-registration written. The re-open bar moves to ORGANIC rings: the capture now needs `--fix-compile --write` runs on REAL workspace breakage (organic accumulate via any future fix-compile run — zero cost, auto-dumping) until the multi-attempt + non-clean floors are met on data that wasn't composed to order.
4. **T1's 128-window floor needs the primitive's own pooling decision** (katgpt-rs 1244's lane) if world-granularity trajectories are ever in scope — recorded, not planned here.

## What this campaign is NOT

- NOT a negative result on the predictor concept — the early-state features have never been measured on organic multi-attempt data; this corpus is synthetic-by-design (the known-inverse mutation corpus, the lane's own GOAT harness).
- NOT a fix-rate claim — the driver fix rates above are the floor harness's own measured posture on the generated corpus, quoted only as capture provenance.
- NOT a timing claim — the box was quiet (load 7%, 17.9/31.8 GiB RAM, AC) but no latency figure is quoted or consumed; the census scores kept/reverted verdicts, load-invariant.

## Box state + provenance

- 4090 (shikuwa, Windows 11, i7-13700K 24 threads, RTX 4090), AC, load 7% at run time, 17.9 GiB RAM free, E: 122 GiB free; build `cargo build --release --locked --features rustc_errors,fix_trajectory_census --examples` at HEAD `0a533bfb62f`+ (dev tree at the claim commit `6214f72a145`); cargo-refine 0.5.0 (8a32c41d44c6) unrelated to the capture (the driver ran in-process).
- Corpus: `riir-refine` in-tree (`corpus_gen::generate_corpus` + `compose_multi{,_strided}` + `corpus_interacting`), byte-stable per build (the generator's determinism pin).
- Instrument: `examples/rustc_errors_floor.rs` (--only driver + dump flags, the 126/128 posture) + `examples/fix_trajectory_census.rs` + `src/fix_trajectory_census.rs` (all landed, gate-tested).
- Issue: riir-refine `.issues/256` (claim `6214f72a145`, folded in the closing commit); sibling consumer riir-ai `.issues/1259` (the corpus's downstream) — the 172 rings now EXIST for it; the organic-growth re-open in (3) is its cheapest path too.
- Research: katgpt-rs `.research/692_RET_Macrostate_Closure_Audit.md`; riir-refine Research 168; prior record `.benchmarks/165` (the NO-VERDICT census this one supersedes with data).
