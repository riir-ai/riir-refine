# Release notes — v0.4.0 (the public-goods pivot: join / leave / rank / corpus, points in, KAT billing out)

Cut 2026-11-03 on the public-goods pivot (owner v4 directives, riir-refine
issue 192). This is a POSTURE release: the shipped binary is the charitable
set — contribution points replace the KAT billing plane, and the contribution
verbs ship.

---

corpus: kernel_opt 1051 (+29 since the v0.3.5 build) — rust_perf 293 (+15) · sec 38 (+7) · clippy_lints 112 (=) · dist 7 (=)

**Verify with SHA256SUMS; dependency attributions in THIRD_PARTY_LICENSES.md. Install: see README.**

## What changed since v0.3.5 (user-visible)

- **`cargo refine fix` is free and consent-first.** Local healing never needs
  consent or network. The first run asks ONCE: join, share anonymously under
  a random client UUID, or never send. `RIIR_REFINE_OFFLINE=1` kills every
  network lane (air-gapped users supported).
- **New verbs: `join` / `leave` (+ hidden `left` alias) / `rank` / `info` /
  `corpus` / `version`.** `join` = policy accept + GitHub device flow (the
  minimal `read:user` scope — proving who you are, never write scope) + the
  signed contribution binding. `rank` is joined-only (anonymous = local
  stats + the named reason: no rotating-identity farming of the board).
  Anonymous callers get public top-ten only.
- **Contribution points, not tokens.** Points are non-transferable, expiring
  reputation (the balance decays for ranking; contribution history and the
  MIT corpus are permanent). No purchase, no KAT for users, no speculation —
  "contribution points meter hosted lanes as fair use" is the whole economy.
- **The KAT billing plane is compiled out.** No wallet, no top-up, no packs,
  no mine/meter verbs in this build — the `kat_service`/`kat_transport`
  features are OUT of the shipping set (the charitable sweep holds its
  hard-zero KAT classes). Refine is mainnet-only: there is no devnet lane.
- **Privacy law:** the client ID is a locally generated random UUID
  (CSPRNG, resettable via `config reset-client-id`) — never a hardware
  fingerprint; consent text says so in plain words.
- **29 new kernel_opt rules, 15 new rust_perf rules, 7 new sec rules** since
  v0.3.5 (the firezone/sanstun, sccache, TensorFold and vllm distills among
  them) — every verified fix still records a self-evolve trajectory.

## Honest exposure bound (the T15 note)

The pivot ships a NEW points client talking to a NEW service surface
(`/points/*` on the riir.ai mainnet worker, deployed 2026-11-03, version
`a623b49b`). `join` requires the `tuna.riir.ai` host to resolve — if the
domain attach has not landed yet, `join` names the DNS failure instead of
hanging. The KAT-era verbs this release compiles out were already
inert-by-server for new accounts (marketing frozen); nothing a v0.3.5 user
could do is lost except the frozen KAT billing surfaces themselves, which
are gone by design, not by defect. No prior-release defect is newly
disclosed by this cut.

## Upgrade channels

`cargo refine upgrade` (verified: SHA256SUMS-checked fetch + backup-swap) ·
`brew upgrade cargo-refine` · `scoop update cargo-refine` ·
<https://github.com/gist-rs/cargo-refine/releases> — the tap formula + scoop
manifest are bumped in the same window as this release (manual mode, main
direct).
