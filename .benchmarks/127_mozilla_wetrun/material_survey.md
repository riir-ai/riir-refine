# Mozilla outbound fix-PR wetrun — material survey (2026-11-02)

## Scan results (BEFORE state)

| repo | pin | clippy @ their posture | pedantic sweep | verdict |
|---|---|---|---|---|
| uniffi-rs | ef9625b68 | **0 warnings** (598 artifacts, their 1.94.0 toolchain) | 3668 pedantic findings (docs/must_use/cast lints — NOT their posture, not PR material) | clean tree → issue-driven fix |
| sccache | 71d107e0 | **0 warnings** (their CI -D warnings gate live) | 1577 pedantic findings (same class) | clean tree → issue-driven fix |
| rust-code-analysis | 37e5d83c | not scanned (dormant since 2026-01, no clippy gate in CI) | — | scan-only, no PR (one-shot rule) |
| mp4parse-rust | d3a9abf | **0 warnings** (both workspace crates) | — | clean tree |
| supply-chain | e6648794 | n/a (0 .rs files) | — | skip |

Conclusion: NO clippy-lane material exists at any target's own posture. The 022 pipeline gate worked — mechanical-only PRs are correctly empty. Wetrun pivots to issue-driven fixes (still healer-assisted + 022 disclosure).

## Upstream PR-climate evidence (uniffi-rs)

- bendk merges steady feature/fix PRs; badboy reviews (changes-requested with real CI output).
- External PR #3020 (issue 3011 JNA pool, by Fesshompa): CHANGES_REQUESTED 2026-10-12, NO maintainer reply for 3 weeks, CI failure (`@JvmField var error_buf: RustBuffer.ByValue` non-null initialiser breaks KTS compile: "property must be initialized or be abstract").
- The author removed the field initialiser (issue 3011 item 1) but kept null-write paths (`error_buf.data = null`, `= null` clears) — incompatible with a non-null field + JNA Structure.write; that's the standing CI break.
- Issue 3013 (Swift copyBytes 3.6–10.7×): author = us (katopz), 0 replies, no PR.

## Day-1 candidate PRs (owner-gated queue)

1. **uniffi-rs #3020 rescue** (Kotlin JNA pool): patch author branch with @JvmField-null-safety rework OR sit on the review thread with a compile-fixed variant + harness data. Highest accept odds (maintainer already engaged via CHANGES_REQUESTED).
2. **sccache #2975 CACHEDIR.TAG**: additive, spec'd by freedesktop, zero behavior risk. Clean first-PR shape.
3. (held) Swift #3013 lane — pending owner decision (self-issue, needs upstream signal).

## sccache non-goals (T-scc honesty)

Research 362's A-lead (pre_exec/posix_spawn) is sccache's OWN landed fix at the pin — mock_command.rs:269-282 + share_jobserver flag + call site :707. NOT open fix space. Zero corpus-relevant open issues; the CI clippy gate is green. sccache PR material = #2975 only.
