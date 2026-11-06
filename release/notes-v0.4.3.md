# Release notes — v0.4.3 (the kernel lane ships + the health verbs)

Cut 2026-11-06 on the owner's promotion law: "any gain no perf hurt must
enabled by default, dont make good goat sad" — and the health ask: "can i
`cargo refine fixer`, `cargo refine trainer` to see the network and it
stats? (i want it)".

---

corpus: kernel_opt 1061 (+10) — rust_perf 302 (+8) · sec 45 (+7) · clippy_lints 112 (=) · dist 8 (+1)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.4.2 (user-visible)

- **The kernel domain ships in every scan.** The kernel pregate is promoted
  to default + the shipped set: every run first scans each file for
  kernel-bearing code (`.wgsl` always; `.rs` by a pinned GPU-vocabulary
  needle set) and the 1061-rule kernel corpus joins the fan-out only where
  it signals. Measured cost of the gate: added p50 0.036–0.191 ms per file
  (173×/920× median cut vs the ungated matcher); the composed posture
  cleared the pre-registered ≤ 5 ms p95 fence with headroom confirmed in an
  alone re-run. Kernel findings cite the restructuring laws (suggest-only —
  the same posture as `sec`); mechanical spans fix compile-gated like
  everywhere else.
- **Demote envs, not flags.** The three posture envs are DEMOTE-ONLY — the
  exact literal `0` restores the pre-promotion behavior byte-identically:
  `RIIR_KERNEL_PREGATE_SCOPE=0` (the file-level gate only),
  `RIIR_KERNEL_PREGATE_LINE_CAP=0` (uncapped line queries),
  `RIIR_KERNEL_PREGATE_QUERY_TEXT_CAP=0` (no per-query text budget). The
  other pinned literals (4; 4096/1024) remain accepted tunings.
- **`cargo refine fixer` — the fixer lane's health view.** Binary posture,
  compiled corpus counts, the posture switches, this machine's fix-campaign
  history (campaigns · findings · kept · reverted), the last run's roll-up,
  the authoring-queue count, and the contribution-network aggregate (epoch
  · contributors · contributions — aggregated counts only, never repo
  content). Read-only.
- **`cargo refine trainer` — the trainer lane's health view.** The learning
  half's posture (the shipped binary is memory-only by design — the line
  says so), the self-evolve memory store's size census (read WITHOUT
  opening the encrypted store, so it never contends with a concurrent run),
  the opt-out env state, and the installed corpus-lease epoch through the
  same net-partitioned resolution `update` uses. Read-only, fully offline.
- **`cargo refine fix` is the verb; `--fix` stays the alias.** The docs,
  usage text and help now teach the verb form; existing scripts keep
  working unchanged.
- **`fix --write` when there is nothing to fix is a clean no-op** (the
  fix-compile green-zero trap closed).
- **The auto-oracle cache.** The clippy session (base + gap passes + the
  census) is content-keyed by a workspace fingerprint and persisted:
  warm runs reuse the diagnostics (cold 6.4 s → hit 1.27 s measured on a
  63 MB tree), one-byte source changes miss honestly, `RIIR_REFINE_ORACLE_CACHE=0`
  demotes, `rm -rf .refine/oracle-cache` resets.
- **The scan-parallel lane.** Per-file scan precompute runs on scoped
  threads (byte-prefix chunks; results byte-identical; 3.1–3.4× wall on a
  770-file tree). `RIIR_REFINE_SCAN_PARALLEL=0` demotes byte-identically.

## Honest notes

- The 46 incumbent-cited kernel rules lose all citations under the default
  query-text cap while the cited census RISES 462 → 538/1061 — disclosed
  and accepted by the promotion call; the non-lossy complement is tracked
  upstream (riir-neuron-db issue 642).
- `kernel_verify` remains a stub (parse-level only): kernel fixes cite and
  apply mechanically fixable spans; a real CubeCL compile oracle is future
  work (Proposal 012 Phase 0).
- The devnet tokenomics reconcile currently disagrees with its own script
  on copy vocabulary (contribution vs mining) — numbers agree; tracked as
  riir-dapps issue 159.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
