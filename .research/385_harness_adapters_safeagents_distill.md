# Research 385 — Harness Adapters + safeagents (HF blog 2026-11-06) — dual @research+@distill

**Status:** DISTILLED — pending owner decision
**Source:** https://huggingface.co/blog/harness-hub/harness-adapters — "Harness adapters: Safe RL for efficient agentic development" (Bybelkada / Lysandre / Habib, HF harness-hub, 2026-11-06). Code home `huggingface/safeagents` (GitHub: "a lightweight local proxy between your agent and its model server; checks every tool call the model makes before the harness sees it", pushed ~2026-10-29) + `harness-hub/safeagents-judge-bench` (dataset) + `harness-hub/traj2envs-rl` (270-task release).
**Inspected:** 2026-11-07. Blog fetched in full this session; the safeagents repo verified by web search ONLY — NOT cloned. Every code-level quote at batch time re-pins against the repo (T0 of plan 352); license UNVERIFIED (outbound-PR FAIL-CLOSED until it is).
**Provenance chain:** HF blog → harness-hub org (built with TRL / OpenEnv / Harbor / vLLM) → safeagents proxy (atif-scan detectors + per-task regex plugins). No arXiv ID for the blog itself; published prior art on the class recorded in §Prior art.
**Verdict:** **YES B− corpus** [2 own sec rules + 1 expected-drop + 2 count-neutral faces after ~50% falsification] **/ LANE INTEL HIGH** (riir-train — the clippy-L4 RLVR lane RE-OPEN case; riir-train issue 677 filed) **/ PRODUCT COVERAGE YES** (the agent-egress-guard family's SECOND dedicated source after SPILLWAY B340).
**Queue slot:** nominal Batch 342, plan 352, STAGED unclaimed (sec-only delta — release-set binary NOT owed).

## What the source is

Three artifacts in one release, all serving one loop — RL-train a small LoRA adapter per (base model, agent harness) pair so the base model behaves better inside that harness:

1. **Harness adapters** — GRPO (TRL `AsyncGRPOTrainer`), LoRA rank 32 `all-linear`, LR 1e-5, temp 0.8, 8 rollouts/task, up to 4 weight-updates stale, token-budget micro-batches (131,072 / 65,536 tokens), harnesses (pi / opencode / hermes) run UNMODIFIED in per-rollout sandboxes. Reward = the task's own verifier + a shaped bonus **0.1 × (1 − calls/15) for fewer tool calls, only on correct rollouts** (weight from an ablation). Models: LFM2.5-24B-A2B, LFM3-1.7B, TwIL-8B (the experiment: a reasoning model never trained as an agent).
2. **Repo2RLEnvs** — public agent trajectories → verifiable tasks in six steps: screen (atif-scan drops reward-tampered / benchmark-leaking traces) → replay (every file write/edit applied to a virtual filesystem so each command sees the files as they were) → find work (**pure-rule typing, no LLM**: `repair` = a check exits non-zero, then the same command exits 0 after edits, no install between, tests untouched; `build` / `extend` / `qa` similarly closed-form) → build + **mutation-test the hidden tests** (pass on the oracle twice, FAIL on a stub and on an empty workspace, catch most injected bugs) → trial matrix (oracle full reward twice; nothing/stub/tamper = 0) → publish pinned by image digest. Oracle = the original agent's own verified work; the teacher LLM only writes tests, never trusted unchecked.
3. **safeagents** — a reverse proxy between harness and model (OpenAI + Anthropic APIs, streaming or not) vetoing every tool call with RULE-BASED detectors in 10–20 ms, deterministic ("the policy can't talk its way past it"). Stage 1: declared-tool membership + any host reached must appear in the conversation — **the model's own earlier turns don't count**. Stage 2: atif-scan detectors (reward tampering, test-file editing, benchmark lookups, proxy bypass) + per-task regex plugins matched **only against agent-authored content (commands, args, file contents, queries, URLs), never tool outputs**. Blocked call → text notice, stop reason downgraded, reward 0 — but token ids + logprobs pass through untouched so GRPO still trains on the blocked tokens. Stage 3: optional LLM judge that can only ADD blocks. Ablation: guard on = +0.0 ± 4.1 held-out pass@1 (costs nothing). A TwIL-8B rollout was caught at step 0 searching for the bucket the task's data came from.

Serving half: one vLLM hosts base weights + every adapter BY NAME; after each gradient step the trainer hot-swaps its adapter into vLLM; in-flight requests finish on the old version; eval loads 9 checkpoints AS ADAPTERS in one H200 job (~$5) vs ~$90 merging each into full weights.

## Dedup gate (both vocabularies; live sec corpus = 48 rules, greps run this session)

| # | Candidate | Source-vocab grep | Corpus-operator grep | Verdict |
|---|---|---|---|---|
| 1 | `credential-material-in-command-argv` — a spawned command's argv/args carrying credential material (env-var interpolation of a token, `auth login` CLI, token-file paths); argv is world-readable (ps/proc); fix = env (`Command::env`) / stdin / config transport, never argv | `HF_TOKEN`, `echo $`, `hf auth`, token-file | `Command::new\|.arg(\|argv\|process` → ONE family: `shell-string-command-interpolation` (CVEngine, CWE-88 argument injection) | **PASS — adjacent-DISTINCT.** That rule owns the INJECTION axis (untrusted data INTO a shell string; fix = arg array). This owns the EXPOSURE axis (credential material OUT via argv regardless of injection; fix = transport change). Sink-distinct per the egress-url/unbounded-read house shape. In-rule adjudication note mandatory. |
| 2 | `egress-control-bypass-flag` — a network call or spawned command carrying proxy/TLS-verification-disabling flags (`--noproxy`, `NO_PROXY=*`, `--insecure`, `danger_accept_invalid_certs`) that DEFEATS an existing egress/sandbox control | `noproxy`, `NO_PROXY`, `curl --noproxy`, insecure | `noproxy\|no_proxy\|insecure\|danger_accept\|invalid_cert\|proxy` → zero bypass-class hits. `unvalidated-egress-url` owns URL CHOICE (SSRF); `jwt-insecure-validation-flags` owns decode-config flags — neither owns CONTROL DEFEAT | **PASS.** Sink-distinct from both; the regime keys on an egress control EXISTING and the flag defeating it. |
| 3 | `verifier-self-modification` — non-build runtime code writing to the paths that grade/verify it (test files, grader/verifier configs) | `tamper.test_files`, `/tests/grader`, reward write | `tamper\|verifier\|grader` → zero owners | **C, expected-drop-allowed.** The Rust shape is thin: build lanes and fixtures legitimately write test files; the regime must key on a runtime (non-build) mutation path. Author only if the fixture set earns it. |
| 4 | host-sanction provenance (stage-1 host check: human-turn-derived allowlist; model-echo never self-authorizes) | "host … appears nowhere in the conversation" | spillway `read-output-trust-inherits-argument-taint` (B340) | **FACE, not own rule** — the trust-inheritance law applied to conversation sources; the per-field-class taint direction is the face. Rider on the spillway family. |
| 5 | agent-authored-only scoping (plugins match what the agent writes, never tool outputs) | `AUTHORED` scope in the blog's own `RegexDetector` snippet | same family | **FACE** — the per-field-class trust inversion is the decision content; rider on the same family. Plus day-one application of `ascii-token-unicode-word-boundary-scan-miss` (B330) to every name-anchored detector pattern. |

## Ranked candidate table

| Pattern | Source surface | Grade | Notes |
|---|---|---|---|
| `credential-material-in-command-argv` | safeagents `hf_credential` plugins + stage-1 checks | **B−** | The lead. Deterministic, bounded fix-space (transport change), universal std surface (`std::process::Command`), zero corpus owners on the exposure axis. |
| `egress-control-bypass-flag` | safeagents `network.proxy_bypass` + atif-scan | **B−** | Second rule. The anti-regime must be carried: operators legitimately disable proxies in some deployments — the rule keys on an egress CONTROL existing + the flag named as bypass. |
| `verifier-self-modification` | atif-scan `tamper.test_files` / `tamper.reward_write` | **C, drop allowed** | Thin Rust shape; author only if the fixture earns it. |
| host-sanction-provenance | safeagents stage-1 | face | rider on spillway read-output-trust family |
| agent-authored-only scoping | safeagents plugins | face | rider, same family + Unicode-boundary day-one |

**Process adoptions (NOT corpus rules — recorded for the lanes that own them):**
- **The 4-arm fixture matrix**: oracle×2 (determinism tripwire) / stub FAILS / empty FAILS / tamper 0 + a measured mutant-catch floor with the population pinned. Our fixture discipline (standalone-compiled AND executed, falsification arms, the B340 fake-adversary catch) already carries the spirit; the named 4-arm matrix + mutant floor is the widening. Owns: the corpus fixture gates + `proof_negative_test`-class instruments.
- **Wilson-bounded difficulty banding**: bands at n=8 rollouts are noise (Wilson half-width ~±0.17 at p=0.5); band on the bound with CI-crossing hysteresis. Owns: riir-reflex harness (`wilson95` already ships in `ceiling_probe.rs` — consume).
- **Correctness-gated efficiency metric**: `0.1×(1−calls/15)` gated on correct rollouts — the gate is the content (efficiency can never buy correctness); monotone in calls; clamp the negative tail. Report-only column for comparison lanes.

## GOAT candidate test

`credential-material-in-command-argv` is the lead: (1) benchable — **fails for a DOMAIN reason** (exposure class, not a Pareto/perf validator axis) ⇒ suggestion-only `NotBenchable-by-domain` annotation, the sec lane standard (Batch 136/258/340 precedent), verdict stands on decision content × novelty × dedup × yield; (2) exact semantics — string/AST-level pattern, deterministic verdict; (3) magnitude story is EXPOSURE not cycles (argv readable by every local user/process listing); (4) deterministic. Criteria 2–4 pass.

## Three-track panel (merged; advocates ran in one spawn round this session)

**Path 0 outcome (No-GD advocate, coordinator-merged):** 12 of 14 extracted components ship without gradient descent; 11 ride substrate that already exists (`SecRule`, the fixture matrix, `--frontier-report`, `wilson95`, `LoRAHotSwap`/`LoraPair`/`SstAdmission`, the q8-sidecar loader pattern). Genuine builds: the conversation-veto detector family (R1/R2 above), pure-rule trace task typing (a four-rule FSM over `(command, exit_code, touched_files)` — mining-pipeline lane-intel, not corpus), vFS trace replay (blake3-pinned determinism; network-dependent traces dropped LOUD). Not Path-0: GRPO itself, the stage-3 judge (hosted, additive-only — contract doctrine, `StoodByModelless` kinship), vLLM internals, docker-per-rollout sandboxes, the ATIF wire format.

**Coordinator spot-check corrections (the rule: advocate code-facts are cited only after one grep; round-1 reviewer corrections applied):**
- `filter_zero_advantage_groups` (advocate: "already shipped, Plan 348") — **VERIFIED at `crates/riir-train-gpu/examples/plan348_za_go_train_ab.rs:605,613`** (the Plan 348 A/B arm). The coordinator's first pass read UNVERIFIED because the grep returned a paginated first page and the page was read as a finished sweep — the exact pagination-miss class the research skill names; the round-1 reviewer caught it.
- `loss_grpo.rs` (AGENTS.md naming) — not confirmed by the grep; the GRPO trainers found are the four files above. The issue cites the verified set.
- `TrajRowDoc { lint_key, shape, fix, direction, last_run_id }` + `TrajNeighbor` outcomes — **VERIFIED** in `riir-refine/src/traj_store.rs` this session.
- `crates/riir-train-engine/examples/clippy_l4_rlvr_train.rs` — **VERIFIED on disk**: Plan 336 Phase 3, GRPO with `cargo clippy` pass/fail binary reward, `nf_grpo::group_normalize`, BLAKE3 reward cache, `#![cfg(feature = "gemma2_lora")]`, CPU-only backward (4090 OOM), and its own comment stating the proposer update path (`Proposer::update_grpo`) is stubbed in both trait impls. The example has NEVER RUN: Phase 3 is recorded BLOCKED in Plan 336 (see §Path 0.5).

**Track verdicts (one per track, never pooled):**
- **(a) modelless track — GOAT-band Gain** (files created: this note + plan 352; the corpus batch is the proof path).
- **(c) model-based track — Path 0.5 APPLICABLE, POC-issue filed** (riir-train issue 677). Not a plan yet: the T0 census (traj-store × git-join population) must prove the corpus before any plan pins bars — the ungoat discipline. The lane EXISTS but has NEVER RUN (Plan 336 Phase 3 recorded BLOCKED); the re-open lever is the recipe deltas, recorded in the issue.

## Path 0.5 — the riir-train re-open case (issue 677)

The mapping is near-exact: our `fix_verify` loop IS the Repo2RL oracle (check fails → edits → same check passes); `TrajRowDoc` IS the (broken, fixed, checked) triple with outcome labels; GRPO trainers ship. The lane has never run: Plan 336 Phase 3 is recorded BLOCKED because the Phase 2 SFT policy scored 0/60 EM on the formal chunk_00 gate (T2.5 v2 Bench 460 + the T2.7 re-gate Bench 467, via `bonsai_clippy_l4_eval` — Bonsai, the preferred model; NOT the retired gemma, which only gates the example's cfg). The block's own stated reason is the dead-group law ("a policy at 0/60 gives GRPO a reward that is flat at zero on every rollout") — the blog CONFIRMS that law from outside and supplies the remedy Plan 336 never had: task selection where the pass rate is above zero. The deltas:

1. **Task-mix law** — "GRPO only learns from groups where some rollouts pass and others fail." The blog weights tasks solved sometimes-but-not-always. Plan 336's block line states the same law; the NEW content is the remedy — **frontier-band seeding from the BASE policy (pass rate > 0)**, which can start GRPO without the working SFT adapter Phase 3 was waiting on. T4 measures the base-policy pass rate on the candidate set BEFORE group liveness means anything. Our `--frontier-report` already implements the mixed-band ordering modelless-side.
2. **Corpus from the traj-store** — real fixes, in-distribution, instead of Plan 336's synthetic fixture tasks; conjunctive verifier (lint-gone ∧ compile ∧ mutation arm) replaces the binary lint-gone reward; split BY REPO.
3. **Base model** — the runnable example (`clippy_l4_rlvr_train.rs`) is `#![cfg(feature = "gemma2_lora")]` and gemma2 is RETIRED (owner law), so the example must re-seat on Bonsai-2-27B PQ2_0 ternary or qwen3.8 per the model-scope law. The 0/60 record itself is Bonsai (`bonsai_clippy_l4_eval`) and unaffected by the retirement.
4. **Shaped reward** — the tool-economy bonus gated on correctness (fewer verify rounds), ablation-armed.
5. **Guard-reward-0 semantics** — train THROUGH the guards (bench-guard / secret-scan refusals become reward-0 rollouts; tokens still trained), matching `safeagents`' blocked-call handling and our `StoodByModelless` doctrine.

Not transferable at our scale (recorded, with the honest numbers): 48 concurrent sessions (ceiling 2–4 verifier processes), 131K/65K-token micro-batches (4090 caps far lower — the existing example runs CPU backward at ~21 tok/s rollouts), 2–4×H200, MoE active-param economics, the model roster (owner law). Offline RLVR (advantage-weighted regression over logged triples — the `.issues/675` arm's shape) is the corpus-scale posture; online GRPO stays the 30–50-task POC validation arm.

## Internal cousins (read, not re-owned)

- **SPILLWAY B340 (Research 381)** — the agent-egress-guard family's FIRST source (2 rust_perf + 2 sec). This verdict is the family's second; candidates 1–2 land beside it, faces attach to its rules.
- **katgpt-rs Research 25 / Plan 054** — path-hacking detector, OUTPUT-side (`fully_faithful`/`reward_hacking` path consistency). safeagents is PRE-EXECUTION input-side veto: adjacent, different enforcement point; compose never merge.
- **riir-refine Research 377 (SkySynth auditor)** — five-shape reward-hack taxonomy + mutant-validated tests: the taxonomy vocabulary and the 4-arm matrix face already have an internal record; this source is independent corroboration.
- **katgpt-rs Research 260 (MaxProof)** — reward-hacking taxonomy + defense-in-depth verifier; training-pipeline routing precedent for the class.
- **riir-refine `--frontier-report`** — the task-mix law already ships modelless; the blog independently corroborates it from the RL side ("GRPO only learns from groups where some rollouts pass").
- **katgpt-rs Research 21 (G-Zero)** — reward-hacking as the verifier-ceiling failure mode in self-play.
- **riir-ai `LoRAHotSwap`/`LoraPair`/`sst_merge_admission` + reflex A2/A7 + `LaneSlot` epoch tags** — the serving pattern (named adapters over one base, slot-lock swap, in-flight finishes on old, fork/downgrade refused) is our law re-derived on vLLM: external validation, note-level only.

## Published prior art (recorded; our entries are transcriptions, never novelty claims)

arXiv 2510.10931 (mitigating tool-call hacking in deep-research agents, RL setting); PMLR v306 Reward Hacking Benchmark (RL post-training 0.6% → 13.9% hacking rate); Georgia-Tech 2026 thesis (coding-agent reward-hacking behaviors under outcome-reward RL); karanxa/saroku-guard (third-party tool-call classifier). The CLASS is published — dedup was INTERNAL, per the corpus standard.

## Lane-intel summary

1. **riir-train HIGH** — the re-open case above; riir-train issue 677 filed with the census-gated task list. The queue line names it; the owning lane consumes it.
2. **riir-ai / riir-infer (note-level, NO issue — no open lane names this a lever, the resident-router WATCH-CLASS precedent)** — adapter-per-surface serving validated externally; eval-as-adapters is the eval-economy cell shape (their $5-vs-$90 is their hardware — NOT transferable, serving-recipe law); the loader widening (overlay load in `safetensors_loader`, sidecar convention in `gguf_loader`) is a named future lane, model-scope law binds.
3. **riir-reflex (consume)** — Wilson difficulty banding + the gated-efficiency column ride the existing harness; `--frontier-report` corroborated.

## Product-coverage

Agent-egress-guard family: SPILLWAY (first, B340) + safeagents (second, this) = the family now has two dedicated sources. Follow-up pool (accumulation clause — sources read before the batch window fold into Batch 342): `huggingface/safeagents` detectors dir (T0 re-read at the pin), `harness-hub/safeagents-judge-bench` (an eval corpus for detector rules — candidate for fixture adversarial arms), `karanxa/saroku-guard`, arXiv 2510.10931.

## Honest caveats

- **Blog-graded** (the dial9 precedent): recipe/decision content graded from the blog's in-body rule table; the safeagents repo NOT cloned — batch T0 pins sha + license BEFORE any entry lands; license missing ⇒ decision-content-only posture, outbound-PR FAIL-CLOSED.
- The training half carries NO corpus decision content — lane-intel + issue only.
- Detector source is Python; fixtures are ORIGINAL Rust (transcription law; the CVEngine/penthera precedents).
- The stage-3 judge is hosted-tier and additive-only by design — never a corpus rule; recorded as contract doctrine.
- All absolute numbers (10–20 ms checks, $5/$90, tok/s) are theirs on their hardware — never quoted as ours.
- The `repair`-typing rule ("no new install between, tests untouched") is RL-environment logic, not a Rust fix-space — recorded as mining-pipeline lane-intel, not a corpus candidate.

## Verdict history

- Advocate panel: 2 spawn agents, one round, merged above (coordinator corrections recorded in §Three-track).
- §5 round 1: **REVISE** — five corrections, verified against Plan 336 on disk and applied: (1) the 0/60 is the Phase 2 SFT G1 gate (Bench 460/467), NOT an RLVR result — Phase 3 is recorded BLOCKED, never run; (2) the 0/60 is Bonsai (`bonsai_clippy_l4_eval`), NOT the retired gemma — only the example's cfg is `gemma2_lora`; (3) the dead-group law is already Plan 336's own block reason — the blog confirms it and supplies the remedy (frontier-band seeding from the base policy), now the issue's stated lever, with the honest scope that the recipe does NOT diagnose the SFT collapse (Bench 460's corpus+SFT diagnosis stands); (4) `filter_zero_advantage_groups` VERIFIED (Plan 348 A/B, `plan348_za_go_train_ab.rs:605,613`) — the coordinator's UNVERIFIED was a grep-pagination miss; the issue now states what it adds beyond that A/B (corpus-side levers); (5) the example path fixed to `crates/riir-train-engine/examples/…`; the ~191 s/fix figure dropped entirely (it is the M3 CPU-forward eval-driver figure, re-measured at 47.7 s in-plan — a property of the eval driver, not of the lane).
- §5 round 2: **AGREE** (claude_code channel; the reviewer re-verified each correction by grep on disk). Two non-blocking landing notes honored: named-file staging (the sibling's kernel_pregate WIP excluded — staged_set_audit clean, 5 files, one episode) + dual_allocation re-run at commit (0/0/0). **Landing: this commit on riir-refine develop — the landing commit is the arbiter** (a commit cannot carry its own hash in-file; `git log --grep harnessdistill` resolves it).
