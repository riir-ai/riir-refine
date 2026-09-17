---
name: distill
description: Verdict workflow for evaluating external code sources (GitHub repos, crates, kernel libraries, benchmark suites, perf articles) as mining candidates for riir-clippy's rule corpora — kernel_opt, rust_perf, clippy_lints, and proposed future domains (sec, wasm). Use when the user asks whether a repo/crate is a good distill source or mining candidate, before opening a mining batch on an external source, when deciding which domain a source routes to, or when grading rule candidates and estimating batch yield. Enforces corpus dedup grep, domain routing, decision-content testing, GOAT candidate identification (two failure classes — domain-unbenchable is an annotation, pattern weakness is MARGINAL), the product-coverage axis for famous-lib families, and honest WIP/provenance caveats.
---

# Distillation-Source Verdict Workflow — riir-clippy Mining

This skill does for CODE SOURCES what the `research` skill does for papers: a disciplined verdict on whether an external source is worth mining, which domain it feeds, what the rule candidates are, and what will falsify. The unit of value is a **corpus rule** — a named pattern with a bounded fix-space — not a summary of the source.

## When to use

- The user asks "is `<repo/crate>` a good distill source / mining candidate?" (canonical case: raygon-renderer/thermite, 2026-08-18).
- Before opening any mining batch on a NEW external source (already-approved in-repo mining is the `riir-clippy` skill's workflow, not this one).
- Deciding which domain a source routes to (the CPU-SIMD-vs-kernel_opt trap).
- Grading individual rule candidates + estimating realistic batch yield.
- A new domain is being proposed (sec, wasm, ...) and the source would be its seed.

Do NOT activate for: in-repo hazard audits of already-mined files, proposer bug fixes, clippy heal sweeps, or paper distillation (that is `research`).

## The domains — routing table

| Domain | Corpus | Accepts | Rejects / reroutes |
|---|---|---|---|
| `kernel_opt` | GPU kernel rules (`src/domains/kernel_opt/`, corpus COMPLETE for riir-gpu files since Batch 51) | CubeCL/WGSL/CUDA/Metal kernel patterns, host-orchestration around GPU, measured-GOAT distillations of landed fixes | CPU SIMD → `rust_perf`; generic Rust → `rust_perf` |
| `rust_perf` | CPU hot-path rules (`src/domains/rust_perf/`) | simd, allocation, layout, concurrency, arithmetic, iterators, dispatch, caching, io, bevy, wasm categories | GPU kernels → `kernel_opt` |
| `clippy_lints` | Structural lint fix patterns | Parse-only provable lint shapes | Perf patterns |
| Proposed (`sec`, ...) | **DOES NOT EXIST YET** | — | A security-shaped source is NOT mineable today. The verdict may RECOMMEND opening the domain (Proposal + `HealDomain` trait extension per Plan 001), but must state the gap explicitly — never pretend the domain exists. |

**Routing rule of thumb:** CPU SIMD tiering/polyfill libraries → `rust_perf` simd. GPU shader/kernel code → `kernel_opt`. Lint-shaped fix patterns → `clippy_lints`. A source landed a perf fix we can re-measure → measured-GOAT distillation (B49/B52/B54/B55 precedent), any domain.

**Domain-complete state awareness:** the kernel_opt riir-gpu file-mining frontier is CLOSED. Live intake paths are (a) `rust_perf` external sources, (b) measured-GOAT distillations, (c) new-domain proposals. A verdict proposing "mine this GPU repo file-by-file" must check the mining queue snapshot first.

## Pre-flight — MANDATORY before any verdict

1. Read the mining queue snapshot in `riir-clippy/.distill/001_mining_queue_snapshot.md` (corpus counts, sibling-blocked files, active frontier).
2. Grep the target domain's corpus file (`src/domains/<domain>/corpus.rs`) to know the category taxonomy + current rule count.
3. Internalize the falsification history: coordinator verification kills ~50% of seeds (ambient structure, dedup, wrong regime, unreachable code). Recent batches land 1-3 rules. Yield claims above ~5 rules need extraordinary evidence.

## Source inspection protocol

Parallelize round 1: repo structure fetch (2-3 levels deep) + web search (what is it, crates.io presence, maintenance) + nothing local yet. Round 2 after structure: read `Cargo.toml` + `lib.rs` + the 3-5 files **whose names carry mechanism** (`divider.rs`, `dp.rs`, `policy.rs` — the load-bearing heuristic). Round 3: dedup greps (below).

Inspection checklist:

- **Workspace + release profile** — `opt-level = 3`, `lto`, `codegen-units = 1` in Cargo.toml means they benchmark their code. Good signal, cite it.
- **Completeness signals** — README presence, `todo!()`/`unimplemented!()` density, `#![allow(unused)]`, commented-out impls. WIP *modules* are not mineable; *completed modules inside a WIP repo* are (thermite: `divider.rs` complete while `thermite-special` was mostly `todo!()`). Grade at MODULE level, not repo level.
- **Provenance chain** — is it a port of a known technique? Cite BOTH the port and the upstream (thermite divider ← libdivide). The upstream often holds the deeper variant set; if mining the pattern, cite upstream for the math and the repo for the Rust shape.
- **Maintenance** — stars/last commit matter only for the provenance note, never for the grade. Patterns are timeless; abandoned repos still mine fine.
- **Unread files** — anything that timed out or wasn't read gets marked "unverified — re-read at mine time" in the verdict. Never grade an unread file.

### Full-source access — clone into `.raw/` (ephemeral, `rm` when done)

Round 1/2 structure fetches go over the wire; **mining batches and byte-accurate verdicts need the full tree** — full-text grep, multi-file reads, quote verification against the exact commit. When inspection outgrows URL fetches, clone into `.raw/`:

```bash
git clone --depth 1 https://github.com/<org>/<repo>.git .raw/<repo>   # shallow unless mining git history
git -C .raw/<repo> rev-parse HEAD                                     # PIN this sha + license in the verdict BEFORE deleting
```

Rules (all mandatory):

1. **Location:** `riir-clippy/.raw/<repo>/` (gitignored — a vendored upstream tree would be swept into sibling bulk adds and bloats the repo). Ephemeral scratch, never committed, never a sibling repo, never a Cargo/path dep target.
2. **Pin before verdict:** the verdict/note records `@ <full-sha>` + license BEFORE the clone is deleted — the sha is the provenance; the clone is throwaway (precedent: Batches 96–98 cite `antirez/ds4 @ 110afdd8…` / `PrismML/llama.cpp @ e311ed38f` with quotes "re-verified at mine time"). A beta-quality fast-moving repo makes the pin load-bearing — quotes against an unpinned checkout are worthless.
3. **Read-only.** Never modify files inside `.raw`; copy out for patch experiments.
4. **Grep scope hygiene:** `.raw` is NOT shipped substrate — it stays OUT of dedup-gate/pre-flight grep scopes over `riir-clippy/src/` (a hit inside the clone is the source's own code, not coverage).
5. **Cleanup is part of the batch:** when the verdict/note/plan/queue line is committed, `rm -rf .raw/<repo>`. A landed batch with a live `.raw/` entry is an unfinished batch. On session resume, any stale `.raw/` entry is UNTRUSTED — delete and re-clone at the pinned sha; never verdict against a checkout whose commit you did not pin yourself.
6. **Shallow unless history-mining:** `--depth 1` default; full clone only when the task needs `git log`/diff archaeology — say so in the note. `--filter=blob:none` for very large repos with few read files.

## Dedup gate — MANDATORY before claiming any candidate is new

For EACH candidate pattern, grep `riir-clippy/src/` under BOTH vocabularies:

- The source's own term (`divider`, `libdivide`, `double pump`, `target_feature`).
- The corpus's operator-name variant (`invariant multiplication`, `register pairing`, `cpu feature tier`).

Vocabulary translation is load-bearing — corpus concepts ship under rule_ids, not library names (same defense as the research skill's R296 lesson). Zero hits under BOTH vocabularies = passes. Record one line of evidence per pattern in the verdict. A single hit is a LEAD, not coverage — read the hit before calling it a duplicate.

## Decision-content test — the #1 kill reason

Every candidate must be a **named pattern with a bounded fix-space + regime conditions** (when it applies AND when it does not).

- KEEP: two-regime shapes (constant-vs-dynamic divisor, sparse-vs-dense read sets, lane-local-vs-cross-lane ops) — the corpus's favorite.
- KEEP: placement decisions (fold-at-gather vs fold-at-upload; branchful vs branchfree).
- KILL: ambient structure — type boilerplate, import organization, macro plumbing, "uses const fn", API surface area.
- KILL: positive instances of existing rules (the file IS rule N) — that is not a new rule; it may be a hazard filing instead.

## GOAT candidate identification

The verdict MUST name the single strongest benchable candidate and test it against:

1. **Benchable** on an existing validator (rust_perf → perf-bench Pareto validator + exact alloc-count axis; kernel_opt → measured-GOAT distillation shape).
2. **Exact semantics achievable** — integer math → bit-identical G1 trivially; float → needs a tolerance strategy stated up front.
3. **Magnitude story** — a mechanism-level cycle/instruction/allocation argument (hardware div 20-40+ cycles vs mul+shift ~5), not "probably faster".
4. **Deterministic** — same input, same output, no timing-dependent verdicts.

If no candidate passes all four, record WHICH criterion failed and WHY before grading — the two failure classes grade differently (the 2026-09-17 dial9 correction; both grades preserved in Research 179):

- **Criterion 1 fails for a DOMAIN reason** — the win metric lives in a runtime the validator cannot host (tokio fairness/contention latency, bevy frame pacing, wasm FFI), so *no* rule from this domain can ever Pareto-bench. That is a per-rule ANNOTATION (`NotBenchable`, `bench_spec: None`), not a demotion: the corpus lands suggestion-only decision-content entries as first-class (**Batch 136 precedent — the Lattimore rayon blog distill landed 3 NotBenchable rules from a source with NO measured numbers**; the corpus's tokio/bevy/wasm families are built this way). Grade on decision content × novelty × dedup clearance × yield; the verdict can be YES.
- **Criteria 2–4 fail** (or 1 fails because the PATTERN is unbenchable while its domain siblings are not) — the pattern itself is weak: no exact semantics, no magnitude story, timing-dependent. That is MARGINAL **as a corpus batch** at best — say so, then run the lane-intel axis below before recording: a corpus-MARGINAL source can still be the highest-value verdict of the session when it maps onto an open perf lane (the qwen38-recipe precedent).

⛔ **A domain-level limitation must never demote a source — only source-level properties can** (thin decision content, dedup collisions, unreachable code). The retired veto reading ("no benchable candidate ⇒ MARGINAL") meant no tokio source could EVER grade YES while the corpus contained a tokio family graded in by earlier verdicts; that contradiction is the tell that the gate was misread.

## The lane-intel axis — MANDATORY second output (added 2026-09-05, the qwen38-recipe miss)

The corpus grade answers ONE question: "how many rules does this source feed riir-clippy?" It does NOT answer "does this source move one of the workspace's OPEN perf lanes?" — and a source can be corpus-MARGINAL while its lane intelligence is the single most valuable thing in it. The qwen38-flash-6bit-m3-ultra-recipe verdict (2026-09-05) was graded C+/MARGINAL correctly on the corpus axis and simultaneously filed the owner-challenged finding — a production-qualified instance of the ANE-GDN configuration our own Bench 775/779 named as the sign-flip path for the open Issue 769 lane — as a conversation footnote. That was the miss this section exists to prevent.

**For EVERY verdict on a perf-relevant source, BEFORE recording, map the source's techniques against the workspace's open lanes:**

1. Read the league durable record (`../riir-ai/.docs/09_performance/perf_rematch_league.md`) scoreboard — which rows FAIL and by how much (prefill 0.53–0.57× vs llama.cpp at the 2026-09 regate; decode WON).
2. Grep the target lane's open issues (`../riir-ai/.issues/*.md` for M3/GPU lanes) for the source's mechanism keywords (e.g. ANE, offload, prefill, quant).
3. Check the source-shelf docs (`../riir-ai/.docs/09_performance/*sources*.md`) — a named-unmined-lead row pointing at this source family is a STRONG prior that the distill should run even if the repo looks small.

**If a mapping to an open lane exists, the verdict MUST produce — regardless of corpus grade:**

- A dated **league/lane-intel line in the mining queue snapshot** (`.distill/001_mining_queue_snapshot.md`) recording: the mapped lane + issue numbers, the source's config/mechanism deltas vs our measured-negative config, what is NOT transferable (with the measured negative that closes it), and the offered follow-up (targeted distill / re-arm probe / issue addendum).
- A recommendation to **file or update the target issue** with the reference design (config deltas, pin sha) — the verdict alone does not update sibling-repo issues; it names the file and the content.
- The absolute-numbers caveat: a serving recipe's tok/s on different silicon / different model / different quant is NOT headroom evidence — only the same-box league cells own that comparison.

**Corpus-MARGINAL + lane-mapped is a valid combined verdict.** It produces the lane-intel artifacts above but NOT a mining batch. Do not let a C+ corpus grade suppress a lane mapping; do not let a lane mapping inflate the corpus grade.

**Source-class note:** serving-recipe / measured-config repos (env-gated patches around someone else's runtime, JSON tuning tables, benchmark JSONs) are corpus-MARGINAL by construction — their unit of value is the QUALIFIED CONFIGURATION, not kernels. Grade them on the lane axis; mine them only when a component survives the dedup gate as a new rule.

## The product-coverage axis — MANDATORY third output (added 2026-09-17, the dial9 tokio miss)

The lane-intel axis greps OUR workspace issue ledger — it answers "does this move OUR open lanes?" and nothing else. But `cargo heal` ships publicly (gist-rs releases + tap/bucket + the KAT mining network); its consumers are the world's Rust repos. **"Famous lib" coverage is a first-class corpus goal**: tokio, rayon, serde, regex, bevy — the ecosystem's top-tier infrastructure. A 3-rule tokio family against tokio's real surface (hyper/axum/reqwest/sqlx/tonic/aws-sdk) is a PRODUCT GAP, not a niche interest. The corpus carries bevy/wasm/rayon families for exactly this reason; ecosystem coverage is why the corpus exists.

**For EVERY verdict, BEFORE recording:**

1. Identify which ecosystem libraries the source's patterns live in (tokio async? rayon parallel? serde wire? std io?).
2. Grep the target corpus for that lib's family size (e.g. `tokio|spawn_blocking|yield_now` in `rust_perf` corpus entries).
3. Read a SMALL family on a top-tier lib as a COVERAGE GAP — a seeding source can grade YES on decision content even with zero workspace lane mapping and no benchable GOAT. The founding case: the dial9 tokio verdict (2026-09-17) recorded "0 tokio hits in riir-ai/.issues" — an ISSUE-LEDGER fact that must never be read as a surface fact (our own repos run tokio; the rules fire at home too).

**When the product-coverage axis fires, the research note MUST also name the family's follow-up pool** — the canonical upstream sources for that lib (tokio → Alice Ryhl "What is Blocking?" + the official blocking docs + tokio-rs/mini-redis; rayon → the Lattimore posts; ...) — with an accumulation clause: pool sources read before the batch window folds into the same batch (the Batch-145/146 staged-charter precedent).

Do not let a lane-intel zero suppress a product-coverage YES; do not let product coverage inflate a weak corpus grade — the grade still counts rules, the axis sets the family's priority and continuation.

## Verdict format — exact output shape, in conversation

1. **Verdict line** — YES / NO / MARGINAL + letter grade, PLUS the lane-intel verdict when that axis fires (e.g. "MARGINAL C+ corpus / LEAGUE INTEL HIGH"), PLUS the product-coverage verdict when a famous-lib family is in play (e.g. "YES B− corpus / no lane mapping / seeds the tokio family").
2. **Domain routing** — target corpus + category, explicitly naming the tempting WRONG domain (the CPU-SIMD-is-not-kernel_opt trap).
3. **Lane mapping** — when the axis fires: mapped lane + issues, transferable vs not-transferable (each with its measured negative), follow-up offered. **Product-coverage** — when a famous-lib family is in play: the family's current size, the gap, and the follow-up source pool (this lives in the research note; the verdict line just names it).
4. **Ranked candidate table** — pattern | source file | grade | notes. Include the Drop rows (killed candidates) for transparency.
5. **Yield estimate** — expected rules landing after ~50% coordinator falsification.
6. **Honest caveats** — WIP status, provenance, maintenance, consumer-base narrowness, unread files.
7. **Bottom line** — "mine as one batch (next batch number), lead with `<GOAT rule>`" + offer to add to the mining queue; or the lane-intel follow-up when corpus-MARGINAL; or the family-seeding follow-up pool when product-coverage fires on a MARGINAL corpus grade.

## Recording the verdict

- **NO / MARGINAL, no lane mapping** → conversation only. No files, no queue entry.
- **NO / MARGINAL, WITH lane mapping** → the lane-intel line in the queue snapshot (committed, `docs:` prefix) + the issue-filing recommendation recorded in that line. No research note, no plan, no corpus batch.
- **YES** (any B-band grade — B+/B/B−; a C-band YES records the same artifacts plus an explicit weak-grade caveat; the letter is informational, the STATUS drives recording) → three artifacts, same commit:
  1. One line in the mining queue snapshot at `riir-clippy/.distill/001_mining_queue_snapshot.md`.
  2. A `.research/` note in riir-clippy following the repo's research-note shape — `**Status:** DISTILLED — pending owner decision` (the vocabulary is RECORD / DISTILLED / NEGATIVE), born-complete, no checkboxes. Number per `.research/.highwater` monotonic discipline. The note records: source URL, inspection date, provenance chain, ranked table, dedup-grep evidence, caveats, queue slot. Future mining batches cite it.
  3. **The batch plan** — `riir-clippy/.plans/NNN_*.md` (next number per `.plans/.highwater`, re-scan the folder at write time), created BEFORE any corpus edit when the verdict proceeds to a mining batch. `- [ ]` checkbox tasks for: dedup re-verification, entry authoring, standing gates, count pins, GOAT (G1/G2/G3 + retrieval floors), research note, docs/AGENTS/queue-snapshot updates, commit+push. Mark `[x]` on completion, `- [-]` for defers with a reason. The verdict note + queue line cite the plan number. (Adopted 2026-08-18, owner call — the canonical retroactive plan is 006/Batch 60; every batch after it is plan-FIRST.)

## Canonical worked example (thermite, 2026-08-18)

User: "is raygon-renderer/thermite a good distill source?" Verdict: **YES, B+, route `rust_perf` NOT kernel_opt.**

- Inspection: no_std portable-SIMD lib, tiered x86 backends (v1/v2/v3 + polyfills), libdivide-derived `divider.rs`, `DoublePump` register pairing, capability-gated approx ops, benched release profile. WIP signals: no README, `todo!()`s in thermite-special.
- Dedup grep: zero corpus hits for divider / branch-hint / double-pump under both vocabularies → all novel.
- GOAT: `division-by-invariant-multiplication` — branchful-for-constant vs branchfree-for-dynamic two-regime design; integer → bit-identical G1; 5-10x expected; trivially benchable on the Pareto validator.
- Ranked: divider A (GOAT) · capability-gated-approx-op B · double-pump B− · input-domain algorithm selection C · branch-hint-cold-fn C · MM_SHUFFLE macros + packed-struct drop.
- Caveats: WIP repo (mine completed modules only), CPU-SIMD-only domain fit, cite libdivide upstream, `math/policy.rs` unread (timed out — verify at mine time).
- Yield: 1-3 rules. Record: `.research/` note + queue line, same commit.

## Second worked example (dial9 tokio blog, 2026-09-17 — the corrected-verdict precedent)

User: distill `dial9-rs "Principles for fast Tokio applications"`. First pass graded MARGINAL B− — every candidate failed GOAT criterion 1 (fairness/coordination metrics live outside the Pareto validator), and "no candidate passes all four ⇒ MARGINAL" was applied as a veto. Owner challenge ("why you ditch B grade?") forced the re-examination that produced this skill's GOAT two-class split and the product-coverage axis. Re-graded: **YES B− / no lane mapping / seeds the tokio family** (Research 179 / Plan 142, nominal Batch 152).

- Dedup: 2 kills were positive instances of the existing tokio family (`tokio-guard-across-await`, `papaya-lockfree`), 1 kill was the corpus's own park-vs-spin rejection — the gate working; recorded as evidence, never as a grade penalty.
- Candidates: `batch-blocking-pool-segment` B (zero corpus hits; distinct from buffered-io/bufwriter-batched SYSCALL batching) · adaptive-yield-K-consecutive-ready B+ content (distinct-from `async-yield-point` BOTH directions + the light/heavy-load anti-regime — long polls are benign until the runtime OR the OS is loaded) · bounded-fanout-semaphore B− (expected drop; adjacent to `bounded-channel-backpressure`, same family distinct fix-space).
- The reusable lesson: **a corrected verdict keeps BOTH grades in the research note** — the correction is resolved precedent the next borderline call cites, not history to quietly overwrite.

## Constraints (non-negotiable)

- Corpus fixtures are ORIGINAL minimal reproductions. Never paste external source verbatim into `buggy_code`/`fixed_code` — license hygiene + the binary-only distribution rule.
- **Ported numeric algorithm ⇒ exhaustive equivalence gate.** When a batch ports a numeric construction (magic-number division, approx identities, bit tricks), an exhaustive/boundary equivalence test against the reference op (e.g. all divisors 1..=4096 × boundary + LCG numerators vs hardware `/`) is MANDATORY before landing — casual tests miss subtle wrongness (Batch 59: floor-vs-ceil magic at d=3 x=3 + u32-wrap carry loss at d=7 x=u32::MAX, both caught pre-landing, both invisible at small random tests).
- WIP sources: snapshot/quote at mine time (siblings drift), mine completed modules only.
- New domains (sec, wasm, ...) require a Proposal + `HealDomain` trait extension before any corpus entry — a verdict may recommend opening one but must not assume it.
- Large sources: split inspection across 2 subagent lanes by module tree with disjoint scopes (the established 2-lane mining pattern); the coordinator verifies every CRITICAL reachability claim with its own grep (the Research 057 lesson — a subagent's failed grep is not evidence of absence).
- **Plan-first for mining batches.** A YES verdict that proceeds to implementation opens its `.plans/NNN_*.md` plan (checkbox tasks per the global rule) BEFORE the first corpus edit; the batch's Research note and queue line reference the plan. A batch implemented without a plan is a process defect — record it honestly if it ever happens again (precedent: Plan 006 was created retroactively at owner request).
- Commit the queue update + research note in the same unit (idle-loop commit discipline), `feat:`/`docs:` prefix per convention, on `develop`.
