# Release notes — v0.3.4 (the gen-2 mainnet client)

Cut 2026-10-30 (held one day past its 10-29 draft so a late fix could ride). The client
for the gen-2 mainnet (genesis accepted 2026-10-29 at riir.ai — a fresh ledger,
auth v2).

---

corpus: +237 rules since v0.3.3 — rust_perf 278 (+48) · kernel_opt 1022 (+174) · sec 31 (+15) · clippy_lints 112 (=) · dist 7 (=)

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
- **TOTP-armed accounts: consent heartbeats work.** A fix rode in before this
cut: the consent heartbeat shipped its `X-Kat-Totp` header twice (the runtime
comma-joins repeats, the edge read an 18-char code and refused every armed
heartbeat with 403 `totp_required`). **No released binary ever carried this** —
v0.3.3 and earlier cannot reach gen-2 at all — it lived only in local
gen-2-era builds between the version bump (10-29) and this cut; found and
fixed on the four-rig contribution lane 2026-10-30.
- Secret prompts (the genesis authenticator code) no longer echo on macOS.

## Honest exposure bound (the T15 note)

v0.3.3 and earlier cannot sign v2, so against gen-2 they fail closed with a
clear "client too old — upgrade" message (HTTP 426) — no request is accepted
in the old form, nothing is half-applied. Gen-1 history is not carried to
gen-2 (a fresh ledger by design): gen-1 balances and burn state do not exist
there. No other prior-release defect is newly disclosed by this cut. The
TOTP double-header above never reached a release binary either — disclosed
here for the local-build window it affected.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
