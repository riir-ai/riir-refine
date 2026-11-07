# Issue 214 — the kernel pregate's `@compute` needle has never fired (first-byte fold bug) — fix = a widened signal set, pre-registered

**Status:** OPEN 2026-11-06 (katgpt-rs-18) — T1+T2 landed 2026-11-07 (8e781fce0); C0 > 0 on the registered walk (0 file flips / 72 span-seat flips across 2 files) → T3 G5 rig run required (orchestrator's lane); T4 open.
**Lease closed:** zed-13afd776@m3, 1794011737 — T1(+T2) landed 8e781fce0: the ASCII-letter-only fold + pins + the census instrument; C0 = 0 file flips / 72 span-seat flips (shader_perf/scanner.rs 61, kernel_pregate.rs 11) on the registered walk → G5 required, T3/T4 open.

## The defect

`signal_lines` folds every scanned byte with `| 0x20` and tests it against `NEEDLE_FIRST_BYTES = *b"kwctd@"`. `b'@' | 0x20 == b'`'` (0x60), which is not in the set, so a position holding `@` never reaches the needle loop — and the needle loop's own first-byte check (`needle[0] | 0x20 == b`) would reject it too. The `@compute` entry in `SIGNAL_NEEDLES` (Bench 131's pin) is therefore dead: a file signals on `@compute` only when it ALSO carries another needle. The effective pinned set is the nine letter-initial needles. (`ci_matches` has the mirror quirk — `|0x20` maps `@` and `` ` `` to the same byte — harmless once the first byte is checked correctly.)

Probably low-impact (WGSL `@compute` entry points nearly always carry `@workgroup_size`, which hits `workgroup`), but "probably" is the census's job, not the fix's.

## Pre-registered bars (Bench 131 rule: widening the signal set = a NEW registration)

- **Fix shape:** an ASCII-LETTER-only fold (`u8::to_ascii_lowercase`) for the first-byte test and the needle comparison, `@` added to the first-byte set as itself. No other needle changes.
- **C0 census (report, no bar):** on the Bench-137 walk (`src`, 600 files) PLUS the WGSL-in-Rust surfaces (`../riir-gpu`, `../riir-shader`, `../seal-render` tracked `*.rs`), count files whose signal verdict flips OFF→ON and spans whose Scope verdict flips. Zero flips ⇒ the fix is behavior-neutral on the registered corpus; record and land without the cost run.
- **G1 recall:** on flipped files, kernel `by_domain` citations may only be ADDED (the gate is recall-biased; a removal = a defect).
- **G3:** every non-kernel domain byte-identical (structural — the pregate touches only the kernel bit).
- **G5 cost (only if C0 > 0 on the registered walk):** the Bench-137 composed posture, protocol verbatim (`examples/kernel_goat_g5.rs` / `scripts/g5_domain_cost.sh`, interleaved rounds, all must pass): added **p50 ≤ 1.0 ms/file AND p95 ≤ 5.0 ms/file**, quiet rig via `agent-mail rig run`, PROVENANCE beside every number. Fail ⇒ the fix still lands (a dead needle is a lie in the pin) but `@compute` is REMOVED from `SIGNAL_NEEDLES` instead, so the pinned set states what actually runs.
- Unit pins: `@compute` alone signals; `` ` `` alone does not; the existing `signal_lines` tests stay unmodified and green.

## Tasks

- [x] T1 the fold fix + unit pins (one commit, tests first)
  - Landed 8e781fce0 (2026-11-07): pins landed FIRST — `at_compute_needle_signals_alone` FAILED pre-fix (`[]` vs `[2]`, the defect reproduced); `lone_backtick_never_signals` is the green-both-sides regression guard (the old class gate rejected backtick before ci_matches could ever equate it with `@`). Existing `signal_lines` tests unmodified; kernel_pregate 8/8; default lib 3482/0/1 ignored.
  - The fix shape applied verbatim: `to_ascii_lowercase` for the first-byte test + needle comparison, `@` rides in `NEEDLE_FIRST_BYTES` as itself, no other needle touched. Clippy 1.99's `manual_ignore_case_cmp` spelled the comparison as `eq_ignore_ascii_case` (the identical letter-only fold). Rulewright MISS recorded: the release binary's corpus (build 5e61bb594541) predates the lint — `cargo refine fix` applied 0 edits, `cargo clippy --fix` applied the 1 mechanical fix (post-mining queue).
  - En-route repair (pre-existing HEAD red, disclosed in the commit): `batch339_rules_landed` (116/100) + `batch337_rules_landed` (139/100) `too_many_lines` in `src/domains/kernel_opt/corpus/tests.rs` — `#[allow]` with the ledger-locality reason (the repo's existing convention); assertions untouched.
- [x] T2 the C0 census (a small report script or `--census` example over the four roots; flipped files + spans listed)
  - Instrument: `examples/kernel_pregate_census.rs` (kept as the standing report; exit 0; OLD `| 0x20` arm inlined byte-faithful, NEW arm = the crate's own `signal_lines` + `span_in_scope`). Spans measured at 1-line seats — `span_in_scope` is monotone in its range, so a real span flips iff it contains a flipped seat.
  - Result (final code state): registered walk (./src, 611 files) **0 signal OFF→ON flips / 0 ON→OFF / 72 span-seat flips across 2 files** — `src/domains/shader_perf/scanner.rs` new-only `@compute` lines 82+784 (61 seats) and `src/kernel_pregate.rs` itself (11 seats — the gate's own source self-matching, most of it this issue's pin strings/comments). `../riir-shader` 0/0 (127 files); `../seal-render` 0/0 (132 files); `../riir-gpu` **UNSEEN** (absent on this box — never zero). G1 holds by construction: only `@` changes class, letters fold identically → additions only.
  - **C0 > 0 on the registered walk** (span verdicts flip) → the G5 cost run is REQUIRED (T3, orchestrator queues the rig; never m3). T4 stays unchecked until the G5 verdict.
- [ ] T3 G5 run iff C0 > 0 on the registered walk (rig queue; never m3)
- [ ] T4 record + close (HISTORY entry; Bench 131's needle table annotated with the corrected effective set)
