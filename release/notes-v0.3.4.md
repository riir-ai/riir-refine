# Release notes — v0.3.4 (the gen-2 mainnet client)

Cut 2026-10-29. The client for the gen-2 mainnet (genesis accepted the same
day at riir.ai — a fresh ledger, auth v2).

---

corpus: rust_perf +48 (230 → 278), kernel_opt +159 (848 → 1007); clippy_lints 112, dist 7 unchanged

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.3.3 (user-visible)

- **Mainnet is now `https://mainnet.riir.ai` (gen-2).** Bare commands target it
  by default; `mainnet.gist.rs` / `ai.gist.rs` become legacy aliases (gen-1 is
  being retired; its hosts redirect to riir.ai). Devnet stays `devnet.gist.rs`.
- **Network-bound (v2) signatures.** Every signed request (claim, sync, burn,
  pay, stake, vote, consent, decstat, fixstat, replay) now binds the ledger
  instance id. gen-2 refuses the older form with "client too old" — **v0.3.3
  and earlier cannot use gen-2; upgrade.**
- **Your first `cargo refine account` on gen-2 creates your account** and claims
  the free tier: 100 TUNA trial credit (never KAT). The claim line and the
  welcome now say so correctly.
- **Leftover local state from gen-1** stays in its own per-network folder; the
  pre-split flat files can be quarantined with
  `cargo refine config adopt-state mainnet --discard` (nothing is deleted).
- `account custody new-se` (macOS): create an account key born inside the Secure
  Enclave — it never exists in a file.
- Secret prompts (the genesis authenticator code) no longer echo on macOS.

## Honest exposure bound (the T15 note)

v0.3.3 and earlier cannot sign v2, so against gen-2 they fail closed with a
clear "client too old — upgrade" message (HTTP 426) — no request is accepted
in the old form, nothing is half-applied. Gen-1 history is not carried to
gen-2 (a fresh ledger by design): gen-1 balances and burn state do not exist
there. No other prior-release defect is newly disclosed by this cut.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
