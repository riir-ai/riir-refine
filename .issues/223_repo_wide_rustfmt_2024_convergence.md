# Issue 223 — repo-wide rustfmt style-2024 convergence: 392-file drift at the 1.99.0 toolchain, with byte-input exclusion map

**Status:** OPEN — filed 2026-11-10 (takeover sweep, m3). Unclaimed; no lease taken.
**Claimed:** zed-0b69b81d@amo, 1794285000, expires 2026-11-12T04:30:00Z
(scope: T1 posture + T2 gen rename + T3 the per-file pass + T5 blame-ignore; T4 stays owner-gated. Coordination note for the live amo-idle-216 claim: the tree is CLEAN — no mid-flight WIP anywhere — and the sweep touches TRACKED files only; if the 216 lane resumes mid-sweep, its new PoC files are untracked and outside the pass, and any tracked-file edit it lands rebases on the formatted tree — the benign collision class.)
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

- [ ] T1 — decide the posture: either (a) pin a repo `rustfmt.toml` (`style_edition = "2024"`, edition stays 2021) so BOTH `cargo fmt` and bare `rustfmt` converge on style 2024 and the split dies; or (b) declare style 2021 canonical and reformat the 2024-style files back. (a) matches the stack convention (riir-reflex/katgpt-rs AGENTS.md prescribe `rustfmt --edition 2024`); requires the `gen:` field rename first (or the per-file exclusion standing).
- [ ] T2 — the `gen:` field rename in `src/score_bench/generations.rs` (mechanical: `gen:` → `generation:` or `r#gen`; the freeze-table code + its tests re-run; the regen script output unchanged — field NAMES are not the wire).
- [ ] T3 — the repo-wide per-file `rustfmt --edition 2024` pass over the allowed set (the exclusion map above), ONE `style:` commit, landable when no sibling holds a live lease on src/ files (amo's 216 codegram lease was live at filing — coordinate).
- [ ] T4 — the gate question: whether this repo wants a fmt gate (the katgpt-rs `fmt_gate.py` shape) or the standing "format touched files before committing" discipline. Owner call — the gate is a process change, not a code change.
- [ ] T5 — after convergence: add `.git-blame-ignore-revs` entry for the style commit (the katgpt-rs precedent) so blame stays useful.
