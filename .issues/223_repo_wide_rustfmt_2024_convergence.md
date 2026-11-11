# Issue 223 — repo-wide rustfmt style-2024 convergence: 392-file drift at the 1.99.0 toolchain, with byte-input exclusion map

**Status:** OPEN — T4 (the fmt-gate question) remains the one open task, OWNER-GATED (a process change, not a code change). T1/T2/T3/T5 LANDED 2026-11-10 (zed-0b69b81d@amo): T1 `rustfmt.toml` pinned (`style_edition = "2024"`, edition stays 2021 — BOTH cargo fmt and bare rustfmt converge on style 2024 while parsing at 2021, which also dissolves the generations.rs parse blocker for the pass); T2 the two `gen: u8` params renamed to `generation` (wire byte-identical — the names render into `construction:gen{generation}` provenance strings whose literal `gen` prefix is a format-string literal, unchanged); T3 the sweep: 821 candidates minus the exclusion map = 359 files reformatted, ONE commit `0d7c97eb` (folded the Layer-5a-required lint repairs in the touched files: headroom_probe doc backticks ×3 + the mid-function STRUCTURAL_ABSENCE consts hoisted ×3 — pre-existing findings the -D posture required); T5 `.git-blame-ignore-revs` carries `0d7c97eb`. Verified: residual drift 0, exclusions byte-identical, clippy -D warnings 0 errors at BOTH postures, lib 3778 green, retrieval floors green (5b2), 74 single-feature combos green; Layer 5c's 7 failures verified PRE-EXISTING at clean HEAD via a detached worktree (points_client net-pin overlap by-design ×5 + v2/v3 freeze tables stale at all-features ×2 — the corpus lane's regen duty, NOT this sweep's). **En-route gate repairs landed separately** (pre-existing develop-reds the guard run exposed): `774f41dc` (corpus_diagnose required-features must name clippy_lints+rust_perf — a6eb6be0's unconditional corpus references) + `55b335a3` (jury_disagreement fixture loops gate on latent_retrieval — the clippy_lints gate + one ungated loop E0433'd every clippy_verify-without-latent_retrieval posture) + `56236dd8` (repo_consent const-block semicolon — latent since 71a22de5, surfaced by 1c6656a1's expansion). **Rulewright bug found en-route (recorded for the trainer's queue, not fixed here):** the len-zero class rewrote `assert!(!rows.is_empty())` into `assert_ne!(rows, [] as [RouterEvalRow; 0])` — a Vec≠array comparison that does not compile (E0369); edit reverted, hand-fixed instead.
**Claimed:** zed-m3-i223t4, 1794445000, expires 2026-11-13T12:30:00+00:00
**Lease closed:** zed-0b69b81d@amo, epoch 1794285000, was expires 2026-11-12T04:30:00Z (the claimed scope T1/T2/T3/T5 landed; T4 owner-gated remains, unclaimed)
**Found by:** the 11-10 takeover sweep — a dead m3 session (Nov 9, mtimes 12:48–19:54) had run the pass over 12 core files and never committed; salvage `refs/wip/refine-1110-fmt-regen-salvage` (m3-local). Its 11 formatable files landed on develop `style:` (4 files still differed post-pull; amo's 11-10 landings had already brought the other 7).

## The drift

`rustfmt --edition 2024 --check` (the stack's sanctioned invocation, no rustfmt.toml in this repo, crate edition 2021) wants changes in **392 tracked .rs files** (396 minus the 4 landed with the takeover). The toolchain has been pinned 1.99.0 stack-wide since the issue-963 bump, but this repo has **no fmt gate**, so per-author invocation drift accumulated: some agents format `--edition 2024` (style 2024), some run `cargo fmt` (formats at the crate's edition 2021 = style 2021 — the OPPOSITE direction), some never format.

## The blocker that makes this non-trivial: `gen` is a reserved keyword in edition 2024

`src/score_bench/generations.rs` uses `gen: u8` struct fields (legal in edition 2021, where the crate compiles). `rustfmt --edition 2024` **cannot parse the file at all** (E: expected identifier, found reserved keyword `gen`, lines 627/672). Consequences:

- A repo-wide `--edition 2024` pass dies on this file — the pass must be per-file with this file (and any sibling with `gen` identifiers) EXCLUDED, or the fields renamed (touching the freeze-table code — needs its own verified cell).
- `cargo fmt` on this repo today is actively HARMFUL to convergence: it would reformat everything to style 2021, reverting the 2024-style files (the dead session's direction). Do NOT run bare `cargo fmt --all` here — and it would also hit the path-dep siblings (the riir-reflex lesson).

## The exclusion map (byte-exact and record files — NEVER format)

Verified during the takeover sweep; each row checked, not assumed:

| exclusion | why |
|---|---|
| `tests/fixtures/*.rs` (`dot_per_row_{postfix,prefix}_698.rs`) | `include_str!`-ed byte-exact into `tests/wgsl_mine_pairs.rs` (`const DOT_POST: &str = include_str!(...)`) — mining/gates match their CONTENT; a reformat changes mining results |
| `src/domains/*/corpus/fixtures/b*_fixures.rs` (b322/336/337/339/340/341/343/345 families) | executed-provenance artifacts — "Compiled AND executed BEFORE entry: rustc --edition 2021 --test" — the entries' fix arms embed the same code; the files must stay byte-identical to what was executed |
| `vendor/**` | vendored third-party (block) — diff noise against upstream |
| `.benchmarks/**/*.rs` (`112_fsearch_outbound_pr_bench/{bench_scope,gen_names}.rs`) | bench RECORDS — the apparatus a record pins stays as recorded |
| `.proofs/**/*.rs` (`kani_sec_poc/src/lib.rs`) | proof POC artifacts — records |
| `validator_src/**` | workspace-EXCLUDED no_std cdylib sources — own rules, separate toolchain |
| `src/score_bench/generations.rs` | the `gen:` keyword parse blocker (above) — excluded until fields renamed |

Everything else in the drift set (src/**, tests/**, benches/**, examples/**, build.rs, crates/*/src/**) is fair game — formatting is semantics-preserving and the corpus entries embed code as STRINGS inside .rs sources, which rustfmt never touches.

## The work

- [x] T1 — the posture: (a) pin a repo `rustfmt.toml` (`style_edition = "2024"`, edition stays 2021) — LANDED `0d7c97eb`. The `gen:` rename turned out NOT to be a precondition (the toml's edition=2021 parsing dissolves the blocker), but T2 landed anyway as 2024-edition hardening.
- [x] T2 — the `gen:` rename — LANDED `0d7c97eb` (two params, wire byte-identical; no serde on the file, the names only render into format-string literals).
- [x] T3 — the repo-wide pass — LANDED `0d7c97eb` (359 files, one style commit; the 216 coordination resolved: the tree was CLEAN, no mid-flight WIP, the sweep touched tracked files only).
- [ ] T4 — the gate question (owner-gated — a process change, not a code change; the posture now makes a bare `cargo fmt` SAFE for convergence, so the gate's value is drift PREVENTION, decide on that basis).
- [x] T5 — `.git-blame-ignore-revs` entry for `0d7c97eb`.
