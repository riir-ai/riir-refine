# Release notes — v0.3.1 (the mainnet-first cut)

Cut 2026-10-12. The `scripts/manual_release.sh` publish step requires this
file and its `corpus:` line (the line becomes the release TITLE — the update
alert quotes it). Corpus counts verified at cut time from `cargo refine
--info` + the AGENTS domain tables: unchanged since v0.3.0.

---

corpus: no count delta since v0.3.0 (clippy_lints 100, rust_perf 198, kernel_opt 795, dist 7); lease riders: useless_conversion block-tail shape (P48) + the issue-161 module-applicable pool composition

The mainnet-first release (owner call 2026-10-12, "mainnet all the way").

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.3.0 (user-visible)

- **First-join default is MAINNET** — a fresh install now points at
  `mainnet.gist.rs` (the network of record) instead of devnet. Existing
  installs keep whatever they joined with; `--net <devnet|mainnet>` and the
  env dropdown still switch per invocation (riir-refine `4837ace4`).
- **The zero-burn stall is fixed** — the v0.3.0 per-net state rebase left
  long-running installs comparing the net-scoped charge book against the
  ACCOUNT-scoped server watermark, so every burn sync answered `Skipped`
  forever and the server's burn total froze (the contribution-graph stall,
  riir-kat Issue 005 / `301fab1`). This release rides the ack-v6 client
  (`charged_at_ack_micro` + a pending total that is acked-base + net delta);
  existing installs heal on their next sync — no re-adopt, no data loss.
  (The stranded pre-rebase charge delta is unrecoverable by design; the
  issue records why.)
- Corpus-lease riders ship with the next `cargo refine update`:
  `useless_conversion` grows the block-tail `.into()` shape (P48, the 76/76
  census), and the heal-seat pool gains the issue-161 module-applicable
  candidates (T1.6 PASS, Bench 114).

## Honest exposure bound (the T15 note — unchanged)

Every binary shipped before v0.3.0 carries the bidirectional service-host
failover bug (fixed in source 2026-10-07, `6d7567a` in riir-kat); the
exposure ends definitively at the v2-only cutover (HTTP 426 `client too
old`, naming this upgrade). v0.3.0 and v0.3.1 both carry the closure; this
release adds no new exposure.

## Upgrade channels

`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
