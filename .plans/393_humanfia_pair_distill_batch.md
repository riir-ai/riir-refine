# Plan 393 — the humanfia pair distill batch (qda + humanize → 3 transcription rules; nominal Batch 377)

**Status:** FILED — claim with a lease line before the first corpus edit (claim-lease check first, katgpt-rs Issue 943 discipline). T0.1 DONE 2026-11-21 (re-clone + re-pin + Apache-2.0 verified + zero drift); T0.2+ open. Companion: Research 427 (the verdict, dedup evidence, Path 0 table).
**Claimed:** zed-m3-humanfia@m3, 1795288385, expires 2026-11-23T19:13:05+00:00
**Lease closed:** zed-393-t01@amo — T0.1 delivered same session (claim record: zed-393-t01@amo, epoch 1795224851, was expires 2026-11-23T01:34:11+00:00)

**Pre-registered proof path (ungoat law):** all three rules are suggestion-only, NotBenchable-by-domain (the metric-mismatch/annotation class, Batch-136 precedent) — the proof IS the fixture + falsifier + in-rule adjudication set below, plus the standing gates (standalone compile+execute pre-entry, pins_manifest, score-bench). No perf bars exist for this batch; any promotion beyond suggestion-only is out of scope by construction.

## Phase 0 — re-pin + re-verify

- [ ] T0.1 rm stale `.raw` clones; re-clone shallow at the pinned shas (qda `dc39a02bfd87bb2790e34001708d54ce9131e076`, humanize `565e12051378a72603135292c449189c9e8b64bb`); verify Apache-2.0; record any upstream drift since 2026-11-21
  - DONE 2026-11-21 zed-393-t01@amo: no stale qda/humanize clones existed (removed at close per Research 427 §1; `.raw/` holds only sibling lanes `bbtree`/`mozilla-heal` — untouched); re-cloned shallow, checked-out HEAD == pin on BOTH; Apache-2.0 LICENSE verified at both pins; drift ZERO since 2026-11-21 (ls-remote HEAD == pin on both default branches at re-clone time); clones live at `.raw/{qda,humanize}` for T0.2, rm at T3.4
- [ ] T0.2 re-read the UNREAD residue (Research 427 §9): qda-quant `calib/contamination.py` body + solver entry, qda-kl internals, coganchor `fence/{landlock,proxy,wrap}.rs` + `supervisor.rs`; quote every rule's anchor verbatim at the pin
- [ ] T0.3 re-run the three dedup greps (Research 427 §4) at authoring HEAD; re-adjudicate any sibling rows landed meanwhile (LIVE-STATE check — sibling batches land continuously)

## Phase 1 — rust_perf measurement-honesty ×2

- [ ] T1.1 `tuning-source-contamination-filter-vs-scorer-reference` — four-arm fixture (buggy: a tuning/optimization loop reads its data source unfiltered from the scorer's own reference set; fixed: reference-digest index + normalized-text + 20-word-window filter with the scaffolding exemption, drop+log-never-refuse; all-safe; tampered-verdict arm). In-rule adjudications BOTH directions: `adoption_gate`'s `Decontaminator` (outcome-row dedup vs calibration-reference filtering), `corpus_referee` DiD-drift class; grammar-not-answers doctrine cited as the law's home. Multilingual no-space-script window arm in the fixture (the Thai/Devanagari/Arabic/Cyrillic class)
- [ ] T1.2 `proxy-selection-runs-behavioral-catch-on-survivors` — fixture plants the proxy-good+gen-bad class (a selector promotes on a proxy score alone while its survivors loop/generate degenerate output); the staged ladder {proxy → behavioral catch on SURVIVORS → gold on finalists}; the four claim classes counted, never pooled; the truncation-count face folded in-notes. Adjudications BOTH directions: `degenerate-output-invalidates-throughput-claim` (SELECTION vs THROUGHPUT claim site), `gate-cut-fitted-on-serving-representation` (representation vs behavior), `unique-prompts-defeat-prefix-cache…` (compose), `self-caused-wait-excluded-from-rate-estimate` (compose)
- [ ] T1.3 category placement: measurement-honesty (the 15th category); router/elo spans re-derived per their own conventions if the population moves

## Phase 2 — sec agent-sandbox

- [ ] T2.1 `agent-child-unsandboxed-account-authority` — four-arm fixture (defect: a harness spawns an agent CLI or agent-driven command with full invoking-user authority — no filesystem scope, no egress control, no scratch; fixed: the fence-construction posture — scoped filesystem + egress proxy env + scratch redirection, fail-closed on out-of-scope writes; all-safe: trusted build tooling (cargo-class) and user-initiated interactive shells; tampered-verdict arm). **Pre-registered demotion criterion (named BEFORE authoring, the architecture-not-pattern kill class):** if no bounded spawn-shape predicate separates {agent CLI / agent-driven command} from {cargo-class trusted tooling, user-initiated interactive shell} on the four-arm fixture WITHOUT listing product names, DEMOTE to a face on `egress-control-bypass-flag` / opal R2 — do not ship it as a rule. Faces folded in-notes: scratch-fail-closed (fetched content landing outside granted paths stays out, failing closed) + remote-status-standin (a stand-in stub that may run reads exit-0-as-success → the process is killed instead). Adjudications BOTH directions: opal R2 `agent-code-static-boundary-diagnostics-never-gate` (diagnostics vs FENCE posture), `egress-control-bypass-flag` (DEFEAT vs ABSENCE of controls), `credential-material-in-command-argv` (secret exposure vs authority scoping), `shell-string-command-interpolation` (injection vs authority)
- [ ] T2.2 family placement: the sec agent-harness family (0sec/spillway/safeagents + opal lineage — coganchor is its third source and first large pure-Rust fence); family census + AGENTS table row updated only if the family formalizes a new named axis

## Phase 3 — standing gates + landing

- [ ] T3.1 fixtures standalone-compiled AND executed pre-entry (the held gate catches fixture defects — expect it to); count-neutral faces pinned on their owners, never new rules
- [ ] T3.2 `pins_manifest --write` in-landing; score-bench run; the gen2/gen3 freeze-table regeneration in the SAME batch (plan 137 T7.5) if the corpus generation moves
- [ ] T3.3 retrieval floors re-derived (blended/clippy/perf) per their own Plan-030-style conventions; HELD or re-pinned with measured numbers, attribution recorded
- [ ] T3.4 Research 427 + queue 001 updated in-landing (batch number, verdict, landed rule ids); commit+push; rm the `.raw` clones
