---
name: riir-clippy
description: Auto-continue idle loop + riir-clippy batch mining/heal workflow for the riir-* workspace. Activate when the user says "continue", "auto continue", "keep going", "nothing to do", "idle", "next", "next batch", "Batch 32+", or when a session summary shows remaining riir-clippy batches; when asked to fix clippy across repos, mine new rust/kernel perf rules, dry-run `cargo heal --suggest/--fix` on workspace repos and adapt proposers when suggestions are wrong, find/finish issues or plans, run boundary-guard/substrate-first routine checks, run benchmark regression sweeps, search online to unblock, or find related papers. Chains doc-sync + research skills at the right steps.
---

# riir-clippy — the auto-continue idle loop

You are the idle-time worker for the workspace. When activated without a
specific task, run the priority queue below, **one unit per cycle**,
re-scanning after each completed unit. **Commit + push each completed unit
before starting the next.**

**Rule 0 — session context outranks the queue.** User-reported
regressions/bugs/questions in the current conversation always go first.
The queue is for idle time only.

## The idle priority queue

| # | Item | How |
|---|------|-----|
| 1 | **Fix open issues** | Scan `.issues/` across repos, oldest first, skip deferred. Fix → validate → doc → remove file (noise-reduction rule). |
| 2 | **Finish in-flight plans** | `.plans/` with unchecked `- [ ]` — complete the next task group, honor `- [-]` defers. |
| 3 | **Clippy sweep, all repos** | Heal-driven where possible (see heal protocol below), manual `cargo clippy --all-targets` otherwise. |
| 4 | **Resolved-issue hygiene** | Find issue files whose work is verifiably landed but file remains → doc at `.docs`/AGENTS.md → `rm` the file. |
| 5 | **Batch mining + heal dry-run** | The centerpiece — see the two protocols below. |
| 6 | **boundary-guard routine check** | Run `scripts/ci_boundary_guard.sh` where present (riir-mmorpg-examples); otherwise the 7-surface grep table from the boundary-guard skill. File violations as `.issues/` — do NOT fix in the same commit as detection. Record the run in the skill's Run log. |
| 7 | **substrate-first drift audit** | Mode 2 of the substrate-first skill: inventory substrate primitives, grep consumers for parallel systems (3+ vocabulary variants!), classify (false positive / POC simplification / DRY violation / architectural), file issues. |
| 8 | **Benchmark regression sweep** | GPU-exclusivity check FIRST (see discipline), then pinned suites; compare vs `.benchmarks/` baselines; drift → bisect → root-cause fix → new bench file. |
| 9 | **Online unblock search** | For each blocked/`[-]` item: web search the error/blocker, fetch via `https://r.jina.ai/...` per global Search rules. |
| 10 | **Defer-marking sweep** | Plans/issues with silently-stalled tasks → mark `- [-]` WITH a reason + unblock condition; file a follow-up issue when the blocker is external. |
| 11 | **Online rust/kernel perf search** | rust-optimize checklist, nnethercote/perf-book, bevy, CubeCL/burn, wgpu, CUDA best practices → candidate rules → into the batch pipeline. |
| 12 | **Related-paper research** | Hand off to the `research` skill (its pre-flight + prior-art gate are mandatory — ad-hoc "search online" is the #1 false-PASS mode). |
| 13 | **doc-sync pass** | Invoke the doc-sync skill against repos with landed-but-undocumented work. |

Items 6 + 7 run BEFORE item 8 (benchmarks) by design — boundary/DRY
findings are cheaper to fix before you benchmark around them.

## Protocol A — batch mining (queue item 5)

1. Read the **mining queue snapshot** below + `ls` the target tree. Pick
   the next unmined cluster (1–2 files, ~8–12 rules). NEVER mine a
   sibling-blocked file.
2. **Dedup grep BEFORE proposing** — grep the existing corpus
   (`src/domains/kernel_opt/`, `src/domains/rust_perf/`) for each
   candidate concept. Batch 30 dropped 13 candidates as
   duplicates/adjacent; that filter is load-bearing.
3. Spawn **2 parallel subagents** to read the target file(s) + propose
   rules (pattern, bounded fix-space, category, rationale, source
   quote). **Re-verify every quoted line against the real file** —
   Batch 31's full-quote re-verification caught drift before it landed.
4. Add corpus entries + extend the standing syn-parse gate
   (`batchNN_pseudocode_parses`).
5. **GOAT** — a batch is NOT done at corpus-add time:
   - G1: parse/syn check (all new rules)
   - G2: lib tests pass
   - G3: no-regression — clippy 0 in BOTH feature states + CI guard
   - rust_perf rules: the Pareto bench (`rust_perf_bench` feature) +
     `context_notes` annotations on regime-fragile/Mixed verdicts;
     demote/annotate losers. Never promote a perf gain on a wrong result.
6. Write the Research note (`.research/NNN_*.md` — next free number),
   bump the corpus count in `AGENTS.md`, update `.docs/11_domains/`.
7. Commit `feat: batch NN: ...` + push. Update the mining queue snapshot
   in THIS file (same commit).

## Protocol B — heal dry-run on workspace repos (queue items 3 + 5)

```bash
cd /Users/katopz/git/riir-clippy
cargo run --features latent_retrieval --bin cargo-heal -- --suggest <target-repo>/src/
cargo run --features latent_retrieval --bin cargo-heal -- --fix <target-repo>/src/          # preview
cargo run --features latent_retrieval --bin cargo-heal -- --fix --write <target-repo>/src/  # apply
```

- Validate in the TARGET repo: `cargo clippy --all-targets` +
  `cargo test --lib`. Use `CARGO_TARGET_DIR=/tmp/<repo>-heal` if the
  repo's target dir is locked by a sibling build; clean it up after.
- **Feature-aware validation (the bevy_shared lesson, 2026-08-15):** a
  cfg-gated edit site is INVISIBLE to default `cargo check`/`--all-targets`
  when the gate is a non-test feature — an E0432 sat green in
  `riir-game-sdk/src/backend/bevy_shared.rs` until validated with
  `--features backend_bevy`. After a heal, compile the edited files under
  the features that actually include them (grep the `#[cfg(...)]` on the
  enclosing `mod`); a passing default check proves nothing about gated
  code. Same class as the count-assertion rule below.
- cfg-gated test files: assert COUNTS, not exit codes — a green run can
  mean "ran nothing" (riir-chain lesson).
- **Wrong suggestion → fix the proposer at root cause** (rule/pattern in
  `src/domains/*/proposers/`), add a fixture regression test, re-run.
  NEVER suppress the lint, NEVER hand-edit past it without a fixture —
  the healer must adapt each iteration, not accumulate suppressed lints.
- Commit separately: `fix: heal <repo> ...` (target repo) and
  `fix: proposer ...` (riir-clippy).

## Discipline (non-negotiable)

- **Commit + push per unit** — `feat:`/`fix:`/`docs:` prefixes; stay on
  `develop` (`main` for riir-train); rebase non-interactive only.
- **Numbering**: read `.highwater`, +1, AND re-scan the folder at WRITE
  time — concurrent sessions dual-allocated 675 AND 677 in one night.
- **Sibling safety**: never `git stash` sibling WIP; commit only your own
  hunks; isolate builds under `/tmp`; before blaming CI on riir-clippy,
  check whether `src/domains/*/bench.rs` WIP is the sibling's.
- **GPU exclusivity before benches/gates**: `ps aux | grep -E
  "cargo|riir|katgpt" | grep -v grep` — no second compute consumer
  (Unity Editor + Zed are exempt). Bench 649: 12.3× correctness swing
  under contention.
- Don't fix unrelated bugs — note them in the final summary.
- Low confidence → STOP, emit the 4-part `## Summary` (the auto_prompt
  harness depends on that exact format), let the user re-aim.
- **All queues empty → idle report** (what was checked, all clean). Do
  NOT invent work.

## Skill homes (canonical-in-repo + global symlink — never copies)

| Skill | Canonical home | Global |
|---|---|---|
| `riir-clippy` (this) | `riir-clippy/.agents/skills/riir-clippy/` | symlink ✓ |
| `boundary-guard` | `katgpt-rs/.agents/skills/boundary-guard/` | symlink ✓ |
| `substrate-first` | `katgpt-rs/.agents/skills/substrate-first/` | symlink ✓ |
| `doc-sync`, `rust-optimize` | global-only real dirs (no repo home yet — pending) | — |
| `feature-gate-audit`, `goat-audit`, `proposal`, `research` | `katgpt-rs/.agents/skills/` (tracked) | no symlink yet |

Copies drift — the substrate-first global duplicate (identical today,
diff-verified 2026-08-15) was removed for exactly this reason. When
creating a new skill: canonical home = the repo that owns the workflow;
global = symlink only.

## Mining queue snapshot (update per batch — this is the resume state)

- **Batch 32b-1 DONE (2026-08-15, Research 035):** the edge_lora training
  half (topology_trainer + chain_commit + sigmoid_gate + mod) — 10 rules,
  corpus 228 → 238 (A 39, B 39, C 25, D 35, E 14). Bug yield filed as
  riir-ai Issue 684 (hot-loop allocation storm: per-node `vec!` on a FALSE
  aliasing premise ×2 + per-edge `to_vec()` borrow workaround + backward
  staging buffer + the O(E·L²) routing re-derivation ×3 loops + dead
  scratch clears + triple-BLAKE3 load path + 7-site sigmoid duplication
  + discarded topology validation error + u32→u16 narrowing).
- **Batch 32b-2 (NEXT)**: the edge_lora self-play/arena/consolidation
  half — `sleep_consolidation.rs` (615L), `arena_eval.rs` (604L),
  `{go,bomber,fft}_self_play.rs` (730/521/506L), `cross_game_{edge,real}.rs`
  (504/224L) + `kernels/` Rust wrappers → Research 036, corpus 238 → ~246.
  - Note: `sigmoid_outcome_reward` is triplicated across the self-play
    files (part of the Issue 684 7-site sigmoid family) — dedup candidate,
    not a rule candidate.
  - Validation pattern (proven T4 + 32a + 32b-1): corpus files are NOT
    sibling WIP — edit + run GOAT in the main checkout with
    `CARGO_TARGET_DIR=/tmp/riir-clippy-bNN`; `--features kernel_heal`
    for the corpus suite (309 tests at 32b-1), default for the 201 suite.
    `--all-features` is currently broken by SIBLING WIP
    (`rust_perf/bench.rs:335 alloc_axis`) — pre-existing, not yours.
  - Count-pin sites when adding rules (complete list, re-verified 32b-1):
    corpus.rs `entries.len()` assert + KERNEL_CATEGORIES rows + per-category
    count test + 2 grouped-total tests + router.rs `rules_for_expert` total
    + the corpus-count mentions in router.rs doc header + the Plan 003
    doc-comment `238-rule corpus` line above KERNEL_CATEGORIES.
  - **Landing lesson (32b-1, self-caught)**: after a large multi-entry
    corpus edit, `awk`-scan for `rule_id:` lines NOT preceded by a
    standalone `KernelRule {` BEFORE the first compile — JSON `\n`
    escaping in comment blocks can land as literal 2-char `\n` text,
    swallowing the `KernelRule {` opener into the comment (9/10 entries
    broken; rustc's `vec![` unclosed-delimiter error points at the symptom,
    not the cause).
- **BLOCKED**: `ternary_deltanet_gpu_forward.rs` (+ `_cudarc.rs` — sibling
  WIP STILL ACTIVE as of 2026-08-15 evening: qv_lora.rs + cudarc_kernels/
  + riir-gpu/lib.rs + ternary_deltanet_gpu_forward_cudarc.rs all dirty)
  — mine only after their WIP lands.
- **Progress**: riir-gpu ~62 of ~200 files mined. Corpus: 238 rules.
- **Also open (riir-clippy issues)**: 014 (rust_perf v4, 12 tasks), 016
  (clippy structural expansion, 6), 017 (retrieval precision at scale,
  5), 018 (self_evolve promotion, 6). 015 = sibling's ACTIVE WIP
  (fix_verify mode — do not touch `src/bin/heal.rs`, `src/fix_verify.rs`,
  `src/lib.rs`, `Cargo.toml`, proposers mod.rs/vec_with_capacity, bench.rs,
  clippy proposers mod.rs).

## Manual invocation

In Zed: `/riir-clippy` in the agent panel, or ask "run the riir-clippy
skill". From a terminal, the underlying healer is `cargo-heal`
(see Protocol B).
