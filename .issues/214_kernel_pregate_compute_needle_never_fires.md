# Issue 214 — the kernel pregate's `@compute` needle has never fired (first-byte fold bug) — fix = a widened signal set, pre-registered

**Status:** OPEN 2026-11-06 (katgpt-rs-18) — found while reading `src/kernel_pregate.rs` for Issue 213; bars pinned BEFORE any code change or measurement.
**Claimed:** zed-13afd776@m3, 1794011737, expires 2026-11-09T00:35:37Z

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

- [ ] T1 the fold fix + unit pins (one commit, tests first)
- [ ] T2 the C0 census (a small report script or `--census` example over the four roots; flipped files + spans listed)
- [ ] T3 G5 run iff C0 > 0 on the registered walk (rig queue; never m3)
- [ ] T4 record + close (HISTORY entry; Bench 131's needle table annotated with the corrected effective set)
